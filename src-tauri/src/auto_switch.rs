use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Notify;
use tokio::time::{Duration, sleep};

use crate::windsurf::{self, WindsurfCreditsResult, WindsurfSwitchAccountResult, WindsurfTarget};

const WORKER_POLL_SECONDS: u64 = 2;
const VERIFY_TIMEOUT_MS: i64 = 15_000;
const VERIFY_POLL_MS: u64 = 1_000;
const MAX_ATTEMPTS: usize = 3;
const FAILURE_BASE_MS: i64 = 60_000;
const FAILURE_MAX_MS: i64 = 900_000;
const ACTIVITY_WAKE_DELAY_MS: i64 = 5_000;
const ACTIVITY_WAKE_SKIP_WITHIN_MS: i64 = 15_000;
const IDLE_FRONTMOST_PROBE_MS: i64 = 30_000;
const COOLDOWN_FRONTMOST_PROBE_MS: i64 = 15_000;

#[derive(Debug, Clone, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AutoSwitchConfig {
    pub enabled: bool,
    pub interval_seconds: i64,
    pub idle_interval_seconds: i64,
    pub idle_after_unchanged_checks: i64,
    pub daily_threshold_percent: i64,
    pub weekly_threshold_percent: i64,
    pub cooldown_seconds: i64,
    pub all_exhausted_cooldown_seconds: i64,
}

impl Default for AutoSwitchConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_seconds: 60,
            idle_interval_seconds: 300,
            idle_after_unchanged_checks: 3,
            daily_threshold_percent: 15,
            weekly_threshold_percent: 15,
            cooldown_seconds: 45,
            all_exhausted_cooldown_seconds: 600,
        }
    }
}

impl AutoSwitchConfig {
    fn normalized(mut self) -> Self {
        self.interval_seconds = self.interval_seconds.clamp(15, 86_400);
        self.idle_interval_seconds = self.idle_interval_seconds.clamp(30, 86_400);
        if self.idle_interval_seconds < self.interval_seconds {
            self.idle_interval_seconds = self.interval_seconds;
        }
        self.idle_after_unchanged_checks = self.idle_after_unchanged_checks.clamp(1, 100);
        self.daily_threshold_percent = self.daily_threshold_percent.clamp(0, 100);
        self.weekly_threshold_percent = self.weekly_threshold_percent.clamp(0, 100);
        self.cooldown_seconds = self.cooldown_seconds.clamp(5, 86_400);
        self.all_exhausted_cooldown_seconds = self.all_exhausted_cooldown_seconds.clamp(60, 86_400);
        self
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AutoSwitchAccount {
    pub id: Option<String>,
    pub email: Option<String>,
    pub name: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub status: Option<String>,
    pub auth_token: Option<String>,
    pub session_token: Option<String>,
    pub devin_account_id: Option<String>,
    pub devin_primary_org_id: Option<String>,
    pub auth_provider: Option<String>,
    pub plan_name: Option<String>,
    pub teams_tier: Option<i64>,
    pub teams_tier_name: Option<String>,
    pub plan_type: Option<String>,
    pub tier: Option<String>,
    pub used_credits: Option<i64>,
    pub total_credits: Option<i64>,
    pub used_prompt_credits: Option<i64>,
    pub available_prompt_credits: Option<i64>,
    pub used_flow_credits: Option<i64>,
    pub available_flow_credits: Option<i64>,
    pub used_flex_credits: Option<i64>,
    pub available_flex_credits: Option<i64>,
    pub monthly_prompt_credits: Option<i64>,
    pub monthly_flow_credits: Option<i64>,
    pub daily_quota_remaining_percent: Option<i64>,
    pub weekly_quota_remaining_percent: Option<i64>,
    pub daily_quota_reset_at_unix: Option<i64>,
    pub weekly_quota_reset_at_unix: Option<i64>,
    pub expires_at: Option<String>,
    pub plan_start: Option<String>,
    pub plan_start_unix: Option<i64>,
    pub plan_end_unix: Option<i64>,
    pub credits_updated_at: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AutoSwitchSnapshot {
    pub config: AutoSwitchConfig,
    pub accounts: Vec<AutoSwitchAccount>,
    pub windsurf_target: Option<WindsurfTarget>,
    pub last_switch_email: Option<String>,
    pub immediate: bool,
}

impl AutoSwitchSnapshot {
    fn normalized(mut self) -> Self {
        self.config = self.config.normalized();
        self.last_switch_email = self
            .last_switch_email
            .map(|v| normalize_email(&v))
            .filter(|v| !v.is_empty());
        self
    }
}

#[derive(Debug, Clone, Default)]
struct AutoSwitchRuntime {
    busy: bool,
    idle: bool,
    unchanged_checks: i64,
    last_quota_signature: String,
    cooldown_until: i64,
    next_due_at: i64,
    last_check_at: Option<String>,
    last_switch_at: Option<String>,
    last_activity_wake_at: i64,
    last_full_idle_check_at: i64,
    last_window_probe_at: i64,
    last_switch_email: Option<String>,
    message: String,
    failed_until: HashMap<String, i64>,
    failed_count: HashMap<String, i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoSwitchStatus {
    pub enabled: bool,
    pub busy: bool,
    pub idle: bool,
    pub unchanged_checks: i64,
    pub cooldown_until: i64,
    pub next_due_at: i64,
    pub last_check_at: Option<String>,
    pub last_switch_at: Option<String>,
    pub last_activity_wake_at: Option<i64>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AutoSwitchEvent {
    kind: String,
    status: AutoSwitchStatus,
    account: Option<AutoSwitchAccount>,
    email: Option<String>,
    reason: Option<String>,
    message: String,
}

pub struct AutoSwitchState {
    snapshot: Mutex<AutoSwitchSnapshot>,
    runtime: Mutex<AutoSwitchRuntime>,
    notify: Notify,
}

impl Default for AutoSwitchState {
    fn default() -> Self {
        Self {
            snapshot: Mutex::new(AutoSwitchSnapshot::default()),
            runtime: Mutex::new(AutoSwitchRuntime::default()),
            notify: Notify::new(),
        }
    }
}

#[tauri::command]
pub async fn windsurf_auto_switch_update_snapshot(
    state: State<'_, AutoSwitchState>,
    snapshot: AutoSwitchSnapshot,
) -> Result<AutoSwitchStatus, String> {
    let snapshot = snapshot.normalized();
    let now = now_ms();
    {
        let mut stored = state
            .snapshot
            .lock()
            .map_err(|_| "自动切号状态锁异常".to_string())?;
        *stored = snapshot.clone();
    }
    {
        let mut runtime = state
            .runtime
            .lock()
            .map_err(|_| "自动切号运行态锁异常".to_string())?;
        if !snapshot.config.enabled {
            *runtime = AutoSwitchRuntime::default();
            runtime.message = "自动切号未开启".to_string();
        } else {
            if runtime.last_switch_email.is_none() {
                runtime.last_switch_email = snapshot.last_switch_email.clone();
            }
            if snapshot.immediate || runtime.next_due_at <= 0 {
                runtime.next_due_at = now
                    + if snapshot.immediate {
                        2_000
                    } else {
                        snapshot.config.interval_seconds * 1_000
                    };
            }
            if runtime.message.is_empty() {
                runtime.message = "自动切号已开启，等待后台检测".to_string();
            }
        }
    }
    state.notify.notify_one();
    Ok(build_status(&state))
}

#[tauri::command]
pub async fn windsurf_auto_switch_request_check(
    state: State<'_, AutoSwitchState>,
) -> Result<AutoSwitchStatus, String> {
    {
        let mut runtime = state
            .runtime
            .lock()
            .map_err(|_| "自动切号运行态锁异常".to_string())?;
        runtime.cooldown_until = 0;
        runtime.idle = false;
        runtime.next_due_at = now_ms();
        runtime.message = "已请求立即后台检测".to_string();
    }
    state.notify.notify_one();
    Ok(build_status(&state))
}

#[tauri::command]
pub fn windsurf_auto_switch_get_status(
    state: State<'_, AutoSwitchState>,
) -> Result<AutoSwitchStatus, String> {
    Ok(build_status(&state))
}

pub fn spawn_worker(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            let state = app.state::<AutoSwitchState>();
            tokio::select! {
                _ = sleep(Duration::from_secs(WORKER_POLL_SECONDS)) => {}
                _ = state.notify.notified() => {}
            }
            run_worker_tick(&app).await;
        }
    });
}

async fn run_worker_tick(app: &AppHandle) {
    let snapshot = match snapshot_clone(app) {
        Some(v) => v,
        None => return,
    };
    if !snapshot.config.enabled {
        return;
    }
    let now = now_ms();
    let action = {
        let state = app.state::<AutoSwitchState>();
        let mut runtime = match state.runtime.lock() {
            Ok(v) => v,
            Err(_) => return,
        };
        if runtime.busy {
            return;
        }
        if runtime.next_due_at <= 0 {
            runtime.next_due_at = now + snapshot.config.interval_seconds * 1_000;
            emit_status(app, "status", None, None, None);
            return;
        }
        if now < runtime.next_due_at {
            return;
        }
        if runtime.cooldown_until > now {
            if now - runtime.last_window_probe_at >= COOLDOWN_FRONTMOST_PROBE_MS {
                runtime.last_window_probe_at = now;
                WorkerAction::ProbeCooldown
            } else {
                runtime.next_due_at =
                    (runtime.cooldown_until).min(now + COOLDOWN_FRONTMOST_PROBE_MS);
                emit_status(app, "status", None, None, None);
                return;
            }
        } else if runtime.idle {
            let full_due =
                runtime.last_full_idle_check_at + snapshot.config.idle_interval_seconds * 1_000;
            if now < full_due {
                if now - runtime.last_window_probe_at >= IDLE_FRONTMOST_PROBE_MS {
                    runtime.last_window_probe_at = now;
                    WorkerAction::ProbeIdle
                } else {
                    runtime.next_due_at = full_due.min(now + IDLE_FRONTMOST_PROBE_MS);
                    emit_status(app, "status", None, None, None);
                    return;
                }
            } else {
                runtime.busy = true;
                WorkerAction::FullCheck
            }
        } else {
            runtime.busy = true;
            WorkerAction::FullCheck
        }
    };

    match action {
        WorkerAction::FullCheck => {
            emit_status(app, "status", None, None, None);
            run_full_check(app, snapshot).await;
            finish_full_check(app);
        }
        WorkerAction::ProbeCooldown => {
            probe_windsurf_activity(app, snapshot, true);
        }
        WorkerAction::ProbeIdle => {
            probe_windsurf_activity(app, snapshot, false);
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum WorkerAction {
    FullCheck,
    ProbeCooldown,
    ProbeIdle,
}

async fn run_full_check(app: &AppHandle, snapshot: AutoSwitchSnapshot) {
    if snapshot.accounts.is_empty() {
        set_message(app, "暂无账号，自动切号等待导入账号");
        return;
    }
    let current = match current_account_for_auto_switch(app, &snapshot) {
        Some(v) => v,
        None => {
            set_message(app, "未识别到当前账号，等待下一次检测");
            return;
        }
    };
    set_last_check_at(app);
    let refreshed = match refresh_account(current.clone()).await {
        Ok(v) => v,
        Err(_) => {
            set_message(app, "当前账号额度刷新失败");
            return;
        }
    };
    upsert_account(app, refreshed.clone());
    emit_status(
        app,
        "account-refreshed",
        Some(refreshed.clone()),
        refreshed.email.clone(),
        None,
    );
    set_last_switch_email(app, refreshed.email.clone());
    update_idle_state(app, &refreshed, &snapshot.config);
    let decision = quota_decision(&refreshed, &snapshot.config);
    if !decision.has_signal {
        set_message(app, "额度信息不足，跳过本轮自动切号");
        return;
    }
    if !decision.should_switch {
        let idle = runtime_clone(app).map(|v| v.idle).unwrap_or(false);
        set_message(
            app,
            if idle {
                "额度长期未变化，已进入降频检测"
            } else {
                "当前账号额度正常"
            },
        );
        return;
    }
    {
        let state = app.state::<AutoSwitchState>();
        if let Ok(mut runtime) = state.runtime.lock() {
            runtime.idle = false;
            runtime.unchanged_checks = 0;
        }
    }
    try_switch_to_next_available(app, refreshed, snapshot, decision.reason).await;
}

async fn try_switch_to_next_available(
    app: &AppHandle,
    current: AutoSwitchAccount,
    snapshot: AutoSwitchSnapshot,
    reason: String,
) {
    let current_email = normalize_email(current.email.as_deref().unwrap_or(""));
    let current_index = snapshot
        .accounts
        .iter()
        .position(|acc| normalize_email(acc.email.as_deref().unwrap_or("")) == current_email);
    let Some(current_index) = current_index else {
        enter_cooldown(
            app,
            snapshot.config.all_exhausted_cooldown_seconds,
            "没有可自动切换的可用账号",
        );
        return;
    };
    if snapshot.accounts.len() < 2 {
        enter_cooldown(
            app,
            snapshot.config.all_exhausted_cooldown_seconds,
            "没有可自动切换的可用账号",
        );
        return;
    }
    let mut attempts = 0usize;
    let mut has_retryable_failure = false;
    let mut last_failure = String::new();
    for offset in 1..snapshot.accounts.len() {
        let candidate =
            snapshot.accounts[(current_index + offset) % snapshot.accounts.len()].clone();
        let email = normalize_email(candidate.email.as_deref().unwrap_or(""));
        if email.is_empty()
            || email == current_email
            || candidate
                .auth_token
                .as_deref()
                .unwrap_or("")
                .trim()
                .is_empty()
        {
            continue;
        }
        if is_expired(&candidate) || is_free_account(&candidate) {
            continue;
        }
        if failed_until(app, &email) > now_ms() {
            has_retryable_failure = true;
            continue;
        }
        let refreshed = match refresh_account(candidate.clone()).await {
            Ok(v) => v,
            Err(_) => {
                has_retryable_failure = true;
                continue;
            }
        };
        upsert_account(app, refreshed.clone());
        emit_status(
            app,
            "account-refreshed",
            Some(refreshed.clone()),
            refreshed.email.clone(),
            None,
        );
        let candidate_decision = quota_decision(&refreshed, &snapshot.config);
        if !candidate_decision.has_signal {
            has_retryable_failure = true;
            continue;
        }
        if candidate_decision.should_switch {
            continue;
        }
        if attempts >= MAX_ATTEMPTS {
            break;
        }
        attempts += 1;
        let switched =
            match switch_account(refreshed.clone(), snapshot.windsurf_target.clone()).await {
                Ok(v) => v,
                Err(e) => {
                    mark_failure(app, &email);
                    has_retryable_failure = true;
                    last_failure = format!(
                        "{}: {}",
                        refreshed.email.clone().unwrap_or(email.clone()),
                        e
                    );
                    continue;
                }
            };
        upsert_account(app, switched.clone());
        let verified = verify_switch(
            switched.clone(),
            &snapshot.config,
            snapshot.windsurf_target.clone(),
        )
        .await;
        if let Err(e) = verified {
            mark_failure(app, &email);
            has_retryable_failure = true;
            last_failure = format!("{}: {}", switched.email.clone().unwrap_or(email.clone()), e);
            continue;
        }
        mark_success(app, &email);
        set_last_switch_email(app, switched.email.clone());
        set_last_switch_at(app);
        let msg = format!(
            "因 {}，已自动切换到 {}",
            reason,
            switched.email.clone().unwrap_or(email.clone())
        );
        enter_cooldown(app, snapshot.config.cooldown_seconds, &msg);
        emit_status(
            app,
            "switched",
            Some(switched.clone()),
            switched.email.clone(),
            Some(reason),
        );
        return;
    }
    if attempts > 0 {
        let msg = if last_failure.is_empty() {
            format!("自动切号未确认成功，请手动检查：{}", reason)
        } else {
            format!("自动切号未确认成功，请手动检查：{}", last_failure)
        };
        enter_cooldown(app, snapshot.config.all_exhausted_cooldown_seconds, &msg);
    } else if has_retryable_failure {
        enter_cooldown(
            app,
            snapshot.config.cooldown_seconds,
            "候选账号暂时不可用，稍后重试",
        );
    } else {
        enter_cooldown(
            app,
            snapshot.config.all_exhausted_cooldown_seconds,
            "没有可自动切换的可用账号",
        );
    }
}

async fn refresh_account(mut account: AutoSwitchAccount) -> Result<AutoSwitchAccount, String> {
    let auth_token = account.auth_token.clone().unwrap_or_default();
    if auth_token.trim().is_empty() {
        return Err("缺少 auth_token".to_string());
    }
    let devin_auth1_token = auth_token
        .starts_with("auth1_")
        .then_some(auth_token.clone());
    let result = windsurf::windsurf_refresh_credits(
        auth_token,
        account.devin_account_id.clone(),
        devin_auth1_token,
        account.devin_primary_org_id.clone(),
        account.session_token.clone(),
    )
    .await
    .map_err(|e| e.to_string())?;
    if !result.success {
        return Err(result.error.unwrap_or_else(|| "刷新额度失败".to_string()));
    }
    apply_credits(&mut account, result);
    Ok(account)
}

async fn switch_account(
    mut account: AutoSwitchAccount,
    target: Option<WindsurfTarget>,
) -> Result<AutoSwitchAccount, String> {
    let auth_token = account.auth_token.clone().unwrap_or_default();
    let result = windsurf::windsurf_switch_account(
        auth_token,
        None,
        account.devin_account_id.clone(),
        account.devin_primary_org_id.clone(),
        target,
    )
    .await
    .map_err(|e| e.to_string())?;
    if !result.success {
        return Err(result.error.unwrap_or_else(|| "切号失败".to_string()));
    }
    apply_switch_result(&mut account, result);
    Ok(account)
}

async fn verify_switch(
    account: AutoSwitchAccount,
    config: &AutoSwitchConfig,
    target: Option<WindsurfTarget>,
) -> Result<(), String> {
    let expected = normalize_email(account.email.as_deref().unwrap_or(""));
    let deadline = now_ms() + VERIFY_TIMEOUT_MS;
    let mut last_reason = "未等到 Windsurf 完成登录".to_string();
    while now_ms() < deadline {
        sleep(Duration::from_millis(VERIFY_POLL_MS)).await;
        let current = get_current_login_email(target.clone());
        if !current.is_empty() && current != expected {
            last_reason = format!("当前仍识别为 {}", current);
            continue;
        }
        let refreshed = refresh_account(account.clone())
            .await
            .map_err(|_| "切号后刷新额度失败".to_string())?;
        let decision = quota_decision(&refreshed, config);
        if !decision.has_signal {
            return Err("切号后额度信息不足".to_string());
        }
        if decision.should_switch {
            return Err(decision.reason);
        }
        return Ok(());
    }
    Err(last_reason)
}

fn finish_full_check(app: &AppHandle) {
    let snapshot = match snapshot_clone(app) {
        Some(v) => v,
        None => return,
    };
    let now = now_ms();
    {
        let state = app.state::<AutoSwitchState>();
        if let Ok(mut runtime) = state.runtime.lock() {
            runtime.busy = false;
            if snapshot.config.enabled {
                if runtime.cooldown_until > now {
                    runtime.next_due_at =
                        (runtime.cooldown_until).min(now + COOLDOWN_FRONTMOST_PROBE_MS);
                } else if runtime.idle {
                    runtime.last_full_idle_check_at = now;
                    runtime.next_due_at = now
                        + IDLE_FRONTMOST_PROBE_MS
                            .min(snapshot.config.idle_interval_seconds * 1_000);
                } else {
                    runtime.next_due_at = now + snapshot.config.interval_seconds * 1_000;
                }
            }
        }
    }
    emit_status(app, "status", None, None, None);
}

fn probe_windsurf_activity(app: &AppHandle, snapshot: AutoSwitchSnapshot, allow_cooldown: bool) {
    let now = now_ms();
    let status = windsurf::windsurf_get_window_status(snapshot.windsurf_target.clone()).ok();
    let running = status
        .as_ref()
        .and_then(|v| v.get("running"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let frontmost = status
        .as_ref()
        .and_then(|v| v.get("frontmost"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let mut woke = false;
    {
        let state = app.state::<AutoSwitchState>();
        if let Ok(mut runtime) = state.runtime.lock() {
            let in_cooldown = now < runtime.cooldown_until;
            if running && frontmost && (runtime.idle || (allow_cooldown && in_cooldown)) {
                if runtime.next_due_at > now
                    && runtime.next_due_at - now <= ACTIVITY_WAKE_SKIP_WITHIN_MS
                {
                    woke = false;
                } else {
                    runtime.last_activity_wake_at = now;
                    runtime.cooldown_until = 0;
                    runtime.idle = false;
                    runtime.next_due_at = now + ACTIVITY_WAKE_DELAY_MS;
                    runtime.message =
                        "检测到 Windsurf 正在前台使用，已提前安排额度检测".to_string();
                    woke = true;
                }
            }
            if !woke {
                if in_cooldown {
                    runtime.next_due_at = runtime
                        .cooldown_until
                        .min(now + COOLDOWN_FRONTMOST_PROBE_MS);
                } else if runtime.idle {
                    let full_due = runtime.last_full_idle_check_at
                        + snapshot.config.idle_interval_seconds * 1_000;
                    runtime.next_due_at = full_due.min(now + IDLE_FRONTMOST_PROBE_MS);
                }
            }
        }
    }
    emit_status(
        app,
        if woke { "activity-wake" } else { "status" },
        None,
        None,
        None,
    );
}

fn current_account_for_auto_switch(
    app: &AppHandle,
    snapshot: &AutoSwitchSnapshot,
) -> Option<AutoSwitchAccount> {
    let current_email = get_current_login_email(snapshot.windsurf_target.clone());
    if !current_email.is_empty() {
        if let Some(account) = snapshot
            .accounts
            .iter()
            .find(|acc| normalize_email(acc.email.as_deref().unwrap_or("")) == current_email)
        {
            return Some(account.clone());
        }
    }
    let last = snapshot
        .last_switch_email
        .clone()
        .or_else(|| runtime_clone(app).and_then(|v| v.last_switch_email));
    if let Some(last_email) = last {
        let normalized = normalize_email(&last_email);
        if let Some(account) = snapshot
            .accounts
            .iter()
            .find(|acc| normalize_email(acc.email.as_deref().unwrap_or("")) == normalized)
        {
            return Some(account.clone());
        }
    }
    snapshot
        .accounts
        .iter()
        .find(|acc| acc.auth_token.as_deref().unwrap_or("").trim().is_empty() == false)
        .cloned()
}

fn get_current_login_email(target: Option<WindsurfTarget>) -> String {
    match windsurf::windsurf_get_current_login(target) {
        Ok(value) => {
            if value
                .get("success")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                value
                    .get("email")
                    .and_then(|v| v.as_str())
                    .map(normalize_email)
                    .unwrap_or_default()
            } else {
                String::new()
            }
        }
        Err(_) => String::new(),
    }
}

fn apply_credits(account: &mut AutoSwitchAccount, result: WindsurfCreditsResult) {
    account.plan_name = result.plan_name;
    account.teams_tier = result.teams_tier;
    account.teams_tier_name = result.teams_tier_name;
    account.used_credits = result.used_credits;
    account.total_credits = result.total_credits;
    account.used_prompt_credits = result.used_prompt_credits;
    account.available_prompt_credits = result.available_prompt_credits;
    account.used_flow_credits = result.used_flow_credits;
    account.available_flow_credits = result.available_flow_credits;
    account.used_flex_credits = result.used_flex_credits;
    account.available_flex_credits = result.available_flex_credits;
    account.monthly_prompt_credits = result.monthly_prompt_credits;
    account.monthly_flow_credits = result.monthly_flow_credits;
    account.daily_quota_remaining_percent = result.daily_quota_remaining_percent;
    account.weekly_quota_remaining_percent = result.weekly_quota_remaining_percent;
    account.daily_quota_reset_at_unix = result.daily_quota_reset_at_unix;
    account.weekly_quota_reset_at_unix = result.weekly_quota_reset_at_unix;
    account.expires_at = result.expires_at;
    account.plan_start = result.plan_start;
    account.plan_start_unix = result.plan_start_unix;
    account.plan_end_unix = result.plan_end_unix;
    account.credits_updated_at = Some(Utc::now().to_rfc3339());
}

fn apply_switch_result(account: &mut AutoSwitchAccount, result: WindsurfSwitchAccountResult) {
    if result.auth_token.is_some() {
        account.auth_token = result.auth_token;
    }
    if result.session_token.is_some() {
        account.session_token = result.session_token;
    }
    if result.devin_account_id.is_some() {
        account.devin_account_id = result.devin_account_id;
    }
    if result.devin_primary_org_id.is_some() {
        account.devin_primary_org_id = result.devin_primary_org_id;
    }
    if result.provider.is_some() && account.auth_provider.is_none() {
        account.auth_provider = result.provider;
    }
}

fn upsert_account(app: &AppHandle, account: AutoSwitchAccount) {
    let email = normalize_email(account.email.as_deref().unwrap_or(""));
    if email.is_empty() {
        return;
    }
    let state = app.state::<AutoSwitchState>();
    if let Ok(mut snapshot) = state.snapshot.lock() {
        if let Some(existing) = snapshot
            .accounts
            .iter_mut()
            .find(|acc| normalize_email(acc.email.as_deref().unwrap_or("")) == email)
        {
            *existing = account;
        }
    }
}

fn update_idle_state(app: &AppHandle, account: &AutoSwitchAccount, config: &AutoSwitchConfig) {
    let signature = quota_signature(account);
    let state = app.state::<AutoSwitchState>();
    if let Ok(mut runtime) = state.runtime.lock() {
        if !signature.is_empty() && signature == runtime.last_quota_signature {
            runtime.unchanged_checks += 1;
        } else {
            runtime.last_quota_signature = signature;
            runtime.unchanged_checks = 0;
        }
        runtime.idle = runtime.unchanged_checks >= config.idle_after_unchanged_checks;
    }
}

fn enter_cooldown(app: &AppHandle, seconds: i64, message: &str) {
    let state = app.state::<AutoSwitchState>();
    if let Ok(mut runtime) = state.runtime.lock() {
        runtime.cooldown_until = now_ms() + seconds.max(1) * 1_000;
        runtime.message = message.to_string();
    }
    emit_status(app, "status", None, None, None);
}

fn set_message(app: &AppHandle, message: &str) {
    let state = app.state::<AutoSwitchState>();
    if let Ok(mut runtime) = state.runtime.lock() {
        runtime.message = message.to_string();
    }
    emit_status(app, "status", None, None, None);
}

fn set_last_check_at(app: &AppHandle) {
    let state = app.state::<AutoSwitchState>();
    if let Ok(mut runtime) = state.runtime.lock() {
        runtime.last_check_at = Some(Utc::now().to_rfc3339());
    }
}

fn set_last_switch_at(app: &AppHandle) {
    let state = app.state::<AutoSwitchState>();
    if let Ok(mut runtime) = state.runtime.lock() {
        runtime.last_switch_at = Some(Utc::now().to_rfc3339());
    }
}

fn set_last_switch_email(app: &AppHandle, email: Option<String>) {
    let normalized = email.map(|v| normalize_email(&v)).filter(|v| !v.is_empty());
    let state = app.state::<AutoSwitchState>();
    if let Ok(mut runtime) = state.runtime.lock() {
        runtime.last_switch_email = normalized;
    }
}

fn mark_failure(app: &AppHandle, email: &str) {
    let key = normalize_email(email);
    let state = app.state::<AutoSwitchState>();
    if let Ok(mut runtime) = state.runtime.lock() {
        let count = runtime.failed_count.get(&key).copied().unwrap_or(0) + 1;
        runtime.failed_count.insert(key.clone(), count);
        let retry = (FAILURE_BASE_MS * 2_i64.pow((count - 1).min(4) as u32)).min(FAILURE_MAX_MS);
        runtime.failed_until.insert(key, now_ms() + retry);
    }
}

fn mark_success(app: &AppHandle, email: &str) {
    let key = normalize_email(email);
    let state = app.state::<AutoSwitchState>();
    if let Ok(mut runtime) = state.runtime.lock() {
        runtime.failed_count.remove(&key);
        runtime.failed_until.remove(&key);
    }
}

fn failed_until(app: &AppHandle, email: &str) -> i64 {
    let key = normalize_email(email);
    let state = app.state::<AutoSwitchState>();
    state
        .runtime
        .lock()
        .ok()
        .and_then(|runtime| runtime.failed_until.get(&key).copied())
        .unwrap_or(0)
}

fn quota_decision(account: &AutoSwitchAccount, config: &AutoSwitchConfig) -> QuotaDecision {
    let daily = account
        .daily_quota_remaining_percent
        .map(|v| v.clamp(0, 100));
    let weekly = account
        .weekly_quota_remaining_percent
        .map(|v| v.clamp(0, 100));
    if daily.is_none() && weekly.is_none() {
        return QuotaDecision {
            has_signal: false,
            should_switch: false,
            reason: String::new(),
        };
    }
    if let Some(value) = daily {
        if value <= config.daily_threshold_percent {
            return QuotaDecision {
                has_signal: true,
                should_switch: true,
                reason: format!("日额度 {}%", value),
            };
        }
    }
    if let Some(value) = weekly {
        if value <= config.weekly_threshold_percent {
            return QuotaDecision {
                has_signal: true,
                should_switch: true,
                reason: format!("周额度 {}%", value),
            };
        }
    }
    QuotaDecision {
        has_signal: true,
        should_switch: false,
        reason: String::new(),
    }
}

struct QuotaDecision {
    has_signal: bool,
    should_switch: bool,
    reason: String,
}

fn quota_signature(account: &AutoSwitchAccount) -> String {
    format!(
        "{}|{}|{}|{}",
        account
            .daily_quota_remaining_percent
            .map(|v| v.to_string())
            .unwrap_or_default(),
        account
            .weekly_quota_remaining_percent
            .map(|v| v.to_string())
            .unwrap_or_default(),
        account
            .daily_quota_reset_at_unix
            .map(|v| v.to_string())
            .unwrap_or_default(),
        account
            .weekly_quota_reset_at_unix
            .map(|v| v.to_string())
            .unwrap_or_default(),
    )
}

fn is_free_account(account: &AutoSwitchAccount) -> bool {
    let text = [
        account.plan_name.as_deref(),
        account.teams_tier_name.as_deref(),
        account.plan_type.as_deref(),
        account.tier.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" ")
    .to_ascii_lowercase();
    text.split_whitespace().any(|part| part == "free") || text.contains("免费")
}

fn is_expired(account: &AutoSwitchAccount) -> bool {
    let Some(value) = account.expires_at.as_deref() else {
        return false;
    };
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.timestamp_millis() < now_ms())
        .unwrap_or(false)
}

fn snapshot_clone(app: &AppHandle) -> Option<AutoSwitchSnapshot> {
    let state = app.state::<AutoSwitchState>();
    state.snapshot.lock().ok().map(|v| v.clone())
}

fn runtime_clone(app: &AppHandle) -> Option<AutoSwitchRuntime> {
    let state = app.state::<AutoSwitchState>();
    state.runtime.lock().ok().map(|v| v.clone())
}

fn build_status(state: &AutoSwitchState) -> AutoSwitchStatus {
    let snapshot = state
        .snapshot
        .lock()
        .ok()
        .map(|v| v.clone())
        .unwrap_or_default();
    let runtime = state
        .runtime
        .lock()
        .ok()
        .map(|v| v.clone())
        .unwrap_or_default();
    AutoSwitchStatus {
        enabled: snapshot.config.enabled,
        busy: runtime.busy,
        idle: runtime.idle,
        unchanged_checks: runtime.unchanged_checks,
        cooldown_until: runtime.cooldown_until,
        next_due_at: runtime.next_due_at,
        last_check_at: runtime.last_check_at,
        last_switch_at: runtime.last_switch_at,
        last_activity_wake_at: (runtime.last_activity_wake_at > 0)
            .then_some(runtime.last_activity_wake_at),
        message: runtime.message,
    }
}

fn emit_status(
    app: &AppHandle,
    kind: &str,
    account: Option<AutoSwitchAccount>,
    email: Option<String>,
    reason: Option<String>,
) {
    let state = app.state::<AutoSwitchState>();
    let status = build_status(&state);
    let message = status.message.clone();
    let _ = app.emit(
        "windsurf-auto-switch-event",
        AutoSwitchEvent {
            kind: kind.to_string(),
            status,
            account,
            email,
            reason,
            message,
        },
    );
}

fn normalize_email(email: &str) -> String {
    email.trim().to_ascii_lowercase()
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|v| v.as_millis() as i64)
        .unwrap_or(0)
}
