use crate::http_client::create_http_client;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tokio::time::{Duration, sleep};
use uuid::Uuid;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;

const WINDSURF_PLAN_STATUS_API: &str =
    "https://web-backend.windsurf.com/exa.seat_management_pb.SeatManagementService/GetPlanStatus";
const WINDSURF_POST_AUTH_API: &str = "https://web-backend.windsurf.com/exa.seat_management_pb.SeatManagementService/WindsurfPostAuth";
const WINDSURF_GET_CURRENT_USER_API: &str =
    "https://web-backend.windsurf.com/exa.seat_management_pb.SeatManagementService/GetCurrentUser";
const WINDSURF_GET_ONE_TIME_AUTH_TOKEN_API: &str = "https://web-backend.windsurf.com/exa.seat_management_pb.SeatManagementService/GetOneTimeAuthToken";
const AVW_LATEST_RELEASE_API: &str = "https://api.github.com/repos/liubao-cn/avw/releases/latest";
const AVW_RELEASES_URL: &str = "https://github.com/liubao-cn/avw/releases/latest";

#[cfg(target_os = "windows")]
fn command_hidden(program: &str) -> std::process::Command {
    let mut cmd = std::process::Command::new(program);
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvwUpdateCheckResult {
    pub success: bool,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub release_url: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct GithubLatestRelease {
    tag_name: String,
    html_url: Option<String>,
}

fn normalize_version_tag(version: &str) -> String {
    version
        .trim()
        .trim_start_matches('v')
        .trim_start_matches('V')
        .to_string()
}

fn version_numbers(version: &str) -> Vec<u64> {
    normalize_version_tag(version)
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<u64>().ok())
        .collect()
}

fn is_newer_version(latest: &str, current: &str) -> bool {
    let latest_numbers = version_numbers(latest);
    let current_numbers = version_numbers(current);
    let max_len = latest_numbers.len().max(current_numbers.len()).max(1);

    for idx in 0..max_len {
        let latest_part = latest_numbers.get(idx).copied().unwrap_or(0);
        let current_part = current_numbers.get(idx).copied().unwrap_or(0);
        if latest_part > current_part {
            return true;
        }
        if latest_part < current_part {
            return false;
        }
    }

    false
}

#[tauri::command]
pub async fn check_for_updates() -> Result<AvwUpdateCheckResult, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let fail = |msg: String| AvwUpdateCheckResult {
        success: false,
        current_version: current_version.clone(),
        latest_version: None,
        update_available: false,
        release_url: Some(AVW_RELEASES_URL.to_string()),
        error: Some(msg),
    };

    let client = create_http_client()?;
    let response = match client
        .get(AVW_LATEST_RELEASE_API)
        .header("accept", "application/vnd.github+json")
        .header("user-agent", "AVW")
        .send()
        .await
    {
        Ok(response) => response,
        Err(e) => return Ok(fail(format!("检查更新失败: {}", e))),
    };

    let status = response.status();
    if !status.is_success() {
        return Ok(fail(format!("GitHub Releases 返回状态 {}", status)));
    }

    let release = match response.json::<GithubLatestRelease>().await {
        Ok(release) => release,
        Err(e) => return Ok(fail(format!("解析版本信息失败: {}", e))),
    };
    let latest_version = normalize_version_tag(&release.tag_name);
    let update_available = is_newer_version(&latest_version, &current_version);

    Ok(AvwUpdateCheckResult {
        success: true,
        current_version,
        latest_version: Some(latest_version),
        update_available,
        release_url: release.html_url.or_else(|| Some(AVW_RELEASES_URL.to_string())),
        error: None,
    })
}

#[tauri::command]
pub fn open_release_page() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(AVW_RELEASES_URL)
            .spawn()
            .map_err(|e| format!("打开 Release 页面失败: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        command_hidden("cmd")
            .args(["/C", "start", "", AVW_RELEASES_URL])
            .spawn()
            .map_err(|e| format!("打开 Release 页面失败: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(AVW_RELEASES_URL)
            .spawn()
            .map_err(|e| format!("打开 Release 页面失败: {}", e))?;
    }

    Ok(())
}

// ---------------- Protobuf 最小编解码辅助（仅用于 GetOneTimeAuthToken 的 string field 1） ----------------

fn encode_protobuf_varint(mut value: usize, buf: &mut Vec<u8>) {
    while value >= 0x80 {
        buf.push(((value & 0x7F) | 0x80) as u8);
        value >>= 7;
    }
    buf.push(value as u8);
}

/// 编码单个 `string` 字段到 Protobuf（wire type = 2 length-delimited）。
///
/// 用于构造 `GetOneTimeAuthTokenRequest { auth_token = 1 }` 请求体。
fn encode_protobuf_string_field(field_num: u8, value: &str) -> Vec<u8> {
    let mut buf = Vec::with_capacity(value.len() + 8);
    // tag = (field_num << 3) | wire_type(2)
    buf.push((field_num << 3) | 2);
    encode_protobuf_varint(value.len(), &mut buf);
    buf.extend_from_slice(value.as_bytes());
    buf
}

/// 就地追加单个 `string` 字段到现有 Protobuf buffer（避免多余分配）。
fn append_protobuf_string_field(buf: &mut Vec<u8>, field_num: u8, value: &str) {
    buf.push((field_num << 3) | 2);
    encode_protobuf_varint(value.len(), buf);
    buf.extend_from_slice(value.as_bytes());
}

/// 就地追加嵌套消息字段（wire type = 2 length-delimited）。
///
/// 用于构造 `GetAnalyticsRequest` 中的 `repeated QueryRequest` 列表等嵌套结构。
fn append_protobuf_nested_message(buf: &mut Vec<u8>, field_num: u8, payload: &[u8]) {
    buf.push((field_num << 3) | 2);
    encode_protobuf_varint(payload.len(), buf);
    buf.extend_from_slice(payload);
}

/// 就地追加 `int64` / `uint64` varint 字段（wire type = 0）。
///
/// 用于 `GetAnalyticsRequest.start_timestamp/end_timestamp`（Unix 秒）等。
fn append_protobuf_int64_field(buf: &mut Vec<u8>, field_num: u8, value: i64) {
    buf.push((field_num << 3) | 0);
    // 与 protobuf 对齐：负数用 10 字节补齐；Analytics 场景时间戳为正，直接 u64 即可
    encode_protobuf_varint_u64(value as u64, buf);
}

/// 与 `encode_protobuf_varint` 语义一致，但接受 u64（支持 int64 时间戳等场景）。
fn encode_protobuf_varint_u64(mut value: u64, buf: &mut Vec<u8>) {
    while value >= 0x80 {
        buf.push(((value & 0x7F) | 0x80) as u8);
        value >>= 7;
    }
    buf.push(value as u8);
}

/// 从 `bytes[pos]` 开始解码一个 protobuf varint，返回 `(value, consumed_bytes)`。
///
/// 用于通用 Protobuf 响应解析（length-delimited 长度字段、int64 字段等）。
fn decode_protobuf_varint(bytes: &[u8], start: usize) -> Option<(u64, usize)> {
    let mut value: u64 = 0;
    let mut shift: u32 = 0;
    let mut consumed = 0usize;
    let mut i = start;
    while i < bytes.len() {
        let b = bytes[i];
        i += 1;
        consumed += 1;
        value |= ((b & 0x7F) as u64) << shift;
        if b & 0x80 == 0 {
            return Some((value, consumed));
        }
        shift += 7;
        if shift >= 64 {
            return None;
        }
    }
    None
}

/// 从 Protobuf 消息中提取 `field_number == 1 && wire_type == 2` 的字符串值。
///
/// 用于解析 `GetOneTimeAuthTokenResponse { auth_token = 1 }` 响应体。
/// 其他字段会被跳过（支持 varint wire_type=0，其它类型遇到即退出）。
fn decode_protobuf_string_field_one(data: &[u8]) -> Option<String> {
    let mut pos = 0;
    while pos < data.len() {
        let tag = data[pos];
        pos += 1;
        let wire_type = tag & 0x07;
        let field_number = tag >> 3;

        if wire_type == 2 {
            // 读取 varint length
            let mut length: usize = 0;
            let mut shift = 0;
            while pos < data.len() {
                let b = data[pos];
                pos += 1;
                length |= ((b & 0x7F) as usize) << shift;
                if b & 0x80 == 0 {
                    break;
                }
                shift += 7;
                if shift > 63 {
                    return None;
                }
            }
            if pos + length > data.len() {
                return None;
            }
            if field_number == 1 {
                if let Ok(s) = std::str::from_utf8(&data[pos..pos + length]) {
                    if !s.is_empty() {
                        return Some(s.to_string());
                    }
                }
            }
            pos += length;
        } else if wire_type == 0 {
            // 跳过 varint
            while pos < data.len() {
                let b = data[pos];
                pos += 1;
                if b & 0x80 == 0 {
                    break;
                }
            }
        } else {
            // 其它 wire_type 暂不处理
            break;
        }
    }
    None
}

// ---------------- 统一认证上下文（Devin Auth1 / Session Token） ----------------

/// 统一承载 Windsurf gRPC-Web 接口的鉴权头部集合。
///
/// - **Devin Auth1 账号**：`token` 填 `devin-session-token$...` 形式的 session_token，
///   其余 3 个 `devin_*` 字段按后端返回回填；`with_devin=true` 额外发送
///   `x-devin-session-token`（值同 `token`）
///
/// 字段布局与参考项目 `AuthContext` 对齐，`serde` 字段名保持 snake_case，
/// 方便前端直接把 account 对象里的字段平铺传进来。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WindsurfAuth {
    /// 主 `x-auth-token` 值；Devin 为 session_token
    pub token: String,
    /// Devin `x-devin-auth1-token`（Auth1 一级令牌，格式 `auth1_<52>`）
    #[serde(default)]
    pub devin_auth1_token: Option<String>,
    /// Devin `x-devin-account-id`
    #[serde(default)]
    pub devin_account_id: Option<String>,
    /// Devin `x-devin-primary-org-id`
    #[serde(default)]
    pub devin_primary_org_id: Option<String>,
}

impl WindsurfAuth {
    /// 是否为 Devin 账号：token 形态或任一扩展字段存在都判为 Devin。
    fn is_devin(&self) -> bool {
        self.token.starts_with("devin-session-token$")
            || self.devin_auth1_token.is_some()
            || self.devin_account_id.is_some()
            || self.devin_primary_org_id.is_some()
    }

    /// 把 Devin 的鉴权头部一次性写入到已经构造好的请求上。
    ///
    /// 约定：调用方已经设置了 `accept` / `content-type` 等非鉴权头；
    /// 本函数只在此基础上追加 `x-auth-token` 与可选的 4 个 `x-devin-*` 头。
    fn apply_headers(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let mut r = req.header("x-auth-token", &self.token);
        if self.is_devin() {
            // x-devin-session-token 始终与主 token 同值，对齐官网前端行为
            r = r.header("x-devin-session-token", &self.token);
            if let Some(v) = &self.devin_account_id {
                r = r.header("x-devin-account-id", v);
            }
            if let Some(v) = &self.devin_auth1_token {
                r = r.header("x-devin-auth1-token", v);
            }
            if let Some(v) = &self.devin_primary_org_id {
                r = r.header("x-devin-primary-org-id", v);
            }
        }
        r
    }
}

/// 调用 `GetOneTimeAuthToken` 换取一次性 auth_token。
///
/// 请求体 `field 1 = auth.token`（Devin session_token），
/// 请求头通过 [`WindsurfAuth::apply_headers`] 按账号体系自动分流。
/// 成功返回 Windsurf 后端颁发的一次性 auth_token，可用于 `windsurf://` URL scheme 登录桌面客户端。
async fn windsurf_get_one_time_auth_token(auth: &WindsurfAuth) -> Result<String, String> {
    let http_client = create_http_client()?;

    let body = encode_protobuf_string_field(1, &auth.token);

    let req = http_client
        .post(WINDSURF_GET_ONE_TIME_AUTH_TOKEN_API)
        .header("accept", "*/*")
        .header("connect-protocol-version", "1")
        .header("content-type", "application/proto")
        .header("Referer", "https://windsurf.com/")
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .body(body);

    let response = auth
        .apply_headers(req)
        .send()
        .await
        .map_err(|e| format!("请求 GetOneTimeAuthToken 失败: {}", e))?;

    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("读取 GetOneTimeAuthToken 响应失败: {}", e))?;

    if !status.is_success() {
        let err_text = String::from_utf8_lossy(&bytes).to_string();
        return Err(format!(
            "GetOneTimeAuthToken 失败: HTTP {} {}",
            status, err_text
        ));
    }

    decode_protobuf_string_field_one(&bytes)
        .ok_or_else(|| "解析 GetOneTimeAuthToken 响应失败：未找到 auth_token 字段".to_string())
}

/// 构造 `<scheme>://codeium.windsurf#access_token=...&state=...&token_type=Bearer`
/// 并通过系统默认程序打开，触发 Windsurf 桌面客户端 OAuth 回调登录。
///
/// `target` 决定使用哪个 URL scheme：
/// - `windsurf` → `windsurf://`
/// - `windsurf-next` → `windsurf-next://`
fn open_windsurf_callback_url(
    one_time_token: &str,
    target: &WindsurfTarget,
) -> Result<String, String> {
    let state = Uuid::new_v4().to_string();
    // fragment 用 URL encode，避免 token 里的特殊字符破坏 URL
    let fragment = format!(
        "access_token={}&state={}&token_type=Bearer",
        urlencoding::encode(one_time_token),
        urlencoding::encode(&state)
    );
    let callback_url = format!("{}://codeium.windsurf#{}", target.url_scheme(), fragment);

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&callback_url)
            .spawn()
            .map_err(|e| format!("打开 Windsurf 回调 URL 失败: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        // 使用 PowerShell Start-Process 以便 URL 中的特殊字符（如 &）能被正确解析
        command_hidden("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!("Start-Process '{}'", callback_url),
            ])
            .spawn()
            .map_err(|e| format!("打开 Windsurf 回调 URL 失败: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&callback_url)
            .spawn()
            .map_err(|e| format!("打开 Windsurf 回调 URL 失败: {}", e))?;
    }

    Ok(callback_url)
}

// ==================== Devin Auth1 Token 登录链路 ====================

/// WindsurfPostAuth 响应中的 org 信息（单个组织）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WindsurfOrg {
    pub id: String,
    pub name: String,
}

/// `WindsurfPostAuth` 调用的完整响应（Protobuf 解析后的 Rust 视图）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WindsurfPostAuthResponse {
    pub success: bool,
    pub error: Option<String>,
    /// 带 `devin-session-token$` 前缀的会话 token
    pub session_token: String,
    /// 当前账号可见的组织列表；为空时表示单组织或默认组织
    pub orgs: Vec<WindsurfOrg>,
    /// 服务端可能轮换新的 auth1_token，有则以新值为准
    pub auth1_token: Option<String>,
    /// Devin `x-devin-account-id`
    pub account_id: Option<String>,
    /// Devin `x-devin-primary-org-id`
    pub primary_org_id: Option<String>,
}

/// 解析嵌套 `WindsurfPostAuthOrg` 消息（field 1=id, field 2=name）。
fn parse_windsurf_org(bytes: &[u8]) -> Option<WindsurfOrg> {
    let mut org = WindsurfOrg::default();
    let mut i = 0usize;
    while i < bytes.len() {
        let (tag, consumed) = decode_protobuf_varint(bytes, i)?;
        i += consumed;
        let field_no = (tag >> 3) as u32;
        let wire_type = (tag & 0x7) as u8;
        if wire_type != 2 {
            // 跳过非 length-delimited 字段（org 消息目前只定义了 string 字段）
            match wire_type {
                0 => {
                    let (_, c) = decode_protobuf_varint(bytes, i)?;
                    i += c;
                }
                1 => i += 8,
                5 => i += 4,
                _ => return None,
            }
            continue;
        }
        let (len, consumed_len) = decode_protobuf_varint(bytes, i)?;
        i += consumed_len;
        let end = i + len as usize;
        if end > bytes.len() {
            return None;
        }
        let payload = &bytes[i..end];
        match field_no {
            1 => org.id = String::from_utf8_lossy(payload).into_owned(),
            2 => org.name = String::from_utf8_lossy(payload).into_owned(),
            _ => {}
        }
        i = end;
    }
    if org.id.is_empty() { None } else { Some(org) }
}

/// 解析 `WindsurfPostAuth` Protobuf 响应。
///
/// proto 结构（参考项目对齐）：
/// - field 1: session_token (string)
/// - field 2: repeated WindsurfPostAuthOrg orgs
/// - field 3: optional auth1_token (string)
/// - field 4: optional account_id (string)
/// - field 5: optional primary_org_id (string)
fn parse_windsurf_post_auth_response(bytes: &[u8]) -> Result<WindsurfPostAuthResponse, String> {
    let mut result = WindsurfPostAuthResponse::default();
    let mut i = 0usize;

    while i < bytes.len() {
        let (tag, consumed) = decode_protobuf_varint(bytes, i)
            .ok_or_else(|| "WindsurfPostAuth 响应 tag varint 解码失败".to_string())?;
        i += consumed;

        let field_no = (tag >> 3) as u32;
        let wire_type = (tag & 0x7) as u8;

        if wire_type == 2 {
            let (len, consumed_len) = decode_protobuf_varint(bytes, i)
                .ok_or_else(|| "WindsurfPostAuth 响应 length varint 解码失败".to_string())?;
            i += consumed_len;
            let end = i + len as usize;
            if end > bytes.len() {
                return Err(format!(
                    "WindsurfPostAuth 响应长度越界: field={} len={} i={} total={}",
                    field_no,
                    len,
                    i,
                    bytes.len()
                ));
            }
            let payload = &bytes[i..end];
            match field_no {
                1 => {
                    result.session_token = String::from_utf8_lossy(payload).into_owned();
                }
                2 => {
                    if let Some(org) = parse_windsurf_org(payload) {
                        result.orgs.push(org);
                    }
                }
                3 => {
                    result.auth1_token = Some(String::from_utf8_lossy(payload).into_owned());
                }
                4 => {
                    result.account_id = Some(String::from_utf8_lossy(payload).into_owned());
                }
                5 => {
                    result.primary_org_id = Some(String::from_utf8_lossy(payload).into_owned());
                }
                _ => {}
            }
            i = end;
        } else {
            match wire_type {
                0 => {
                    let (_, c) = decode_protobuf_varint(bytes, i)
                        .ok_or_else(|| "WindsurfPostAuth 响应跳过 varint 失败".to_string())?;
                    i += c;
                }
                1 => i += 8,
                5 => i += 4,
                _ => {
                    return Err(format!(
                        "WindsurfPostAuth 响应出现不支持的 wire_type {}",
                        wire_type
                    ));
                }
            }
        }
    }

    if result.session_token.is_empty() {
        return Err("WindsurfPostAuth 响应缺少 session_token 字段".to_string());
    }
    Ok(result)
}

/// 通过 `WindsurfPostAuth` 接口，用 `auth1_token` 换取 `session_token` 以及 Devin 扩展字段。
///
/// - `auth1_token`：Devin 一级令牌（格式 `auth1_<52>`）
/// - `org_id`：可选的目标组织 ID；传空时由后端按用户 primary org 返回
async fn windsurf_post_auth_internal(
    auth1_token: &str,
    org_id: &str,
) -> Result<WindsurfPostAuthResponse, String> {
    let http_client = create_http_client()?;

    let mut body = Vec::with_capacity(auth1_token.len() + org_id.len() + 4);
    append_protobuf_string_field(&mut body, 1, auth1_token);
    if !org_id.is_empty() {
        append_protobuf_string_field(&mut body, 2, org_id);
    }

    let response = http_client
        .post(WINDSURF_POST_AUTH_API)
        .header("accept", "*/*")
        .header("accept-language", "zh-CN,zh;q=0.9")
        .header("connect-protocol-version", "1")
        .header("content-type", "application/proto")
        .header("origin", "https://windsurf.com")
        .header("referer", "https://windsurf.com/account/login")
        .header("sec-fetch-dest", "empty")
        .header("sec-fetch-mode", "cors")
        .header("sec-fetch-site", "same-site")
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .body(body)
        .send()
        .await
        .map_err(|e| format!("WindsurfPostAuth 请求失败: {}", e))?;

    let status = response.status();
    let resp_content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_default();
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("读取 WindsurfPostAuth 响应失败: {}", e))?;

    if !status.is_success() {
        return Err(format!(
            "WindsurfPostAuth 返回状态 {} (content-type={})",
            status, resp_content_type
        ));
    }

    // 有些网关/代理会把 connect unary 响应按 gRPC-Web 加 5 字节 envelope
    // 前 5 字节 = [flag(1) + length(4, big-endian)]，若符合则剥掉
    let body_to_parse: &[u8] = if bytes.len() > 5 {
        let declared = u32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]) as usize;
        if (bytes[0] & 0x7e) == 0 && declared > 0 && declared + 5 == bytes.len() {
            &bytes[5..]
        } else {
            &bytes[..]
        }
    } else {
        &bytes[..]
    };

    let parsed = parse_windsurf_post_auth_response(body_to_parse)
        .map_err(|e| format!("解析 WindsurfPostAuth 响应失败: {}", e))?;

    Ok(parsed)
}

/// 暴露给前端：手动调 `WindsurfPostAuth` 查询 `auth1_token` 可用组织 / 换取 session。
///
/// 用于前端"多组织选择"分支：先不传 `org_id` 调一次拿 orgs，再让用户点选后二次调用。
#[tauri::command]
pub async fn windsurf_post_auth(
    auth1_token: String,
    org_id: Option<String>,
) -> Result<WindsurfPostAuthResponse, String> {
    let token = auth1_token.trim();
    if !token.starts_with("auth1_") {
        return Ok(WindsurfPostAuthResponse {
            success: false,
            error: Some("auth1_token 必须以 `auth1_` 前缀开头".to_string()),
            ..Default::default()
        });
    }
    let org = org_id.as_deref().map(str::trim).unwrap_or("").to_string();
    match windsurf_post_auth_internal(token, &org).await {
        Ok(mut resp) => {
            resp.success = true;
            Ok(resp)
        }
        Err(e) => Ok(WindsurfPostAuthResponse {
            success: false,
            error: Some(e),
            ..Default::default()
        }),
    }
}

// ==================== GetCurrentUser（带 Devin 5-header 支持）====================

/// 调用 `GetCurrentUser` 拉取账号的 user / team / plan / api_key / 配额等原始数据。
///
/// 请求格式（对齐官网前端 & chaogei/windsurf-account-manager-simple 参考实现）：
/// - `content-type: application/proto`
/// - body: Protobuf `GetCurrentUserRequest { auth_token=1, include_user=2, include_team=3, include_role=4 }`
///   编码为 `0x0a + varint(len) + token_bytes + 0x10 0x01 0x18 0x01 0x20 0x01`
///
/// 响应解析使用通用 [`WindsurfProtoParser`] 得到按字段编号命名的 JSON（`subMessage_N` / `string_N` ...）。
async fn windsurf_get_current_user_internal(
    auth: &WindsurfAuth,
) -> Result<serde_json::Value, String> {
    let http_client = create_http_client()?;

    // 构造 GetCurrentUserRequest Protobuf body
    let token_bytes = auth.token.as_bytes();
    let mut body: Vec<u8> = Vec::with_capacity(token_bytes.len() + 10);
    body.push(0x0a); // field 1, wire=2 (length-delimited string)
    encode_protobuf_varint(token_bytes.len(), &mut body);
    body.extend_from_slice(token_bytes);
    // field 2/3/4 = varint 1 (include_user / include_team / include_role)
    body.extend_from_slice(&[0x10, 0x01, 0x18, 0x01, 0x20, 0x01]);

    let req = http_client
        .post(WINDSURF_GET_CURRENT_USER_API)
        .header("accept", "*/*")
        .header("accept-language", "zh-CN,zh;q=0.9")
        .header("cache-control", "no-cache")
        .header("connect-protocol-version", "1")
        .header("content-type", "application/proto")
        .header("origin", "https://windsurf.com")
        .header("pragma", "no-cache")
        .header("priority", "u=1, i")
        .header("referer", "https://windsurf.com/")
        .header(
            "sec-ch-ua",
            r#""Chromium";v="142", "Google Chrome";v="142", "Not_A Brand";v="99""#,
        )
        .header("sec-ch-ua-mobile", "?0")
        .header("sec-ch-ua-platform", r#""macOS""#)
        .header("sec-fetch-dest", "empty")
        .header("sec-fetch-mode", "cors")
        .header("sec-fetch-site", "same-site")
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .header("x-debug-email", "")
        .header("x-debug-team-name", "")
        .body(body);

    let response = auth
        .apply_headers(req)
        .send()
        .await
        .map_err(|e| format!("GetCurrentUser 请求失败: {}", e))?;

    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("读取 GetCurrentUser 响应失败: {}", e))?;

    if !status.is_success() {
        // 错误响应可能是 JSON（connect unary error envelope）也可能是文本
        let err_text = String::from_utf8_lossy(&bytes).to_string();
        return Err(format!("GetCurrentUser 返回状态 {}: {}", status, err_text));
    }

    // 兼容 gRPC-Web 5 字节 envelope
    let body_to_parse: &[u8] = if bytes.len() > 5 {
        let declared = u32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]) as usize;
        if (bytes[0] & 0x7e) == 0 && declared > 0 && declared + 5 == bytes.len() {
            &bytes[5..]
        } else {
            &bytes[..]
        }
    } else {
        &bytes[..]
    };

    let mut parser = WindsurfProtoParser::new(body_to_parse);
    let parsed = parser
        .parse_message()
        .map_err(|e| format!("解析 GetCurrentUser 响应 Protobuf 失败: {}", e))?;

    Ok(parsed)
}

// ==================== 通过 Auth1 Token 一键添加账号 ====================

/// `windsurf_add_account_by_auth1_token` 的统一响应结构。
///
/// - 单组织（或调用方明确指定 org_id / 开启 auto_select_primary_org）直接返回 `account`
/// - 多组织未决时返回 `requires_org_selection = true` + `orgs` 列表，前端二次选择 org_id 后再调用
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AddAccountByAuth1TokenResult {
    pub success: bool,
    pub error: Option<String>,
    #[serde(default)]
    pub requires_org_selection: bool,
    /// 多组织场景返回，供前端渲染选择列表
    pub orgs: Vec<WindsurfOrg>,
    /// 多组织场景把（可能被服务端轮换过的）auth1_token 返回给前端以便二次调用
    pub auth1_token: Option<String>,
    /// 落盘账号对象（JSON，结构由后端根据 GetCurrentUser 响应填充）
    pub account: Option<serde_json::Value>,
}

/// 通过 Devin Auth1 Token 一键解析账号信息（不持久化，由前端决定是否入库 / 同步云端）。
///
/// 流程：
/// 1. 校验 `auth1_<52>` 格式
/// 2. 调 `WindsurfPostAuth(auth1_token, org_id?)` 拿 session_token / account_id / primary_org_id / orgs
/// 3. 多组织未决 & 未开启自动选 primary → 返回 `requires_org_selection` 让前端决策
/// 4. 构造带 5-header 的 [`WindsurfAuth`]，调 `GetCurrentUser` 拉 email / api_key / 配额 / plan_name
/// 5. 把原始 user_info + Devin 扩展字段组合成账号 JSON 对象返回给前端
#[tauri::command]
pub async fn windsurf_add_account_by_auth1_token(
    auth1_token: String,
    org_id: Option<String>,
    auto_select_primary_org: Option<bool>,
) -> Result<AddAccountByAuth1TokenResult, String> {
    let token_trimmed = auth1_token.trim().to_string();
    if !token_trimmed.starts_with("auth1_") {
        return Ok(AddAccountByAuth1TokenResult {
            success: false,
            error: Some("auth1_token 必须以 `auth1_` 前缀开头".to_string()),
            ..Default::default()
        });
    }
    if token_trimmed.len() < 20 {
        return Ok(AddAccountByAuth1TokenResult {
            success: false,
            error: Some(format!(
                "auth1_token 长度异常（{} 字符），请确认完整粘贴",
                token_trimmed.len()
            )),
            ..Default::default()
        });
    }

    let user_specified_org = org_id.as_deref().map(str::trim).unwrap_or("").to_string();
    let auto_pick = auto_select_primary_org.unwrap_or(false);

    // Step 1: auth1_token → session_token
    let post_auth = match windsurf_post_auth_internal(&token_trimmed, &user_specified_org).await {
        Ok(v) => v,
        Err(e) => {
            return Ok(AddAccountByAuth1TokenResult {
                success: false,
                error: Some(format!("auth1_token 无效或已过期: {}", e)),
                ..Default::default()
            });
        }
    };

    // Step 2: 多组织场景处理
    let effective_auth1_token = post_auth
        .auth1_token
        .clone()
        .unwrap_or_else(|| token_trimmed.clone());

    if user_specified_org.is_empty() && !auto_pick && post_auth.orgs.len() > 1 {
        return Ok(AddAccountByAuth1TokenResult {
            success: false,
            requires_org_selection: true,
            orgs: post_auth.orgs.clone(),
            auth1_token: Some(effective_auth1_token),
            error: Some("检测到多个组织，请选择后重试".to_string()),
            ..Default::default()
        });
    }

    // Step 3: 构造 5-header AuthContext，反查 GetCurrentUser
    let auth = WindsurfAuth {
        token: post_auth.session_token.clone(),
        devin_auth1_token: Some(effective_auth1_token.clone()),
        devin_account_id: post_auth.account_id.clone(),
        devin_primary_org_id: post_auth.primary_org_id.clone().or_else(|| {
            if !user_specified_org.is_empty() {
                Some(user_specified_org.clone())
            } else {
                post_auth.orgs.first().map(|o| o.id.clone())
            }
        }),
    };

    let user_info = match windsurf_get_current_user_internal(&auth).await {
        Ok(v) => v,
        Err(e) => {
            return Ok(AddAccountByAuth1TokenResult {
                success: false,
                error: Some(format!("反查账号信息失败: {}", e)),
                ..Default::default()
            });
        }
    };

    // Step 4: 提取关键字段，映射为前端 account 对象
    // GetCurrentUser 响应为 Protobuf，字段映射：
    //   subMessage_1 = User { string_1=api_key, string_2=name, string_3=email, string_6=id, string_7=team_id }
    //   subMessage_2 = Team (plan 信息)
    //   subMessage_7 = UserRole
    let user_obj = user_info
        .get("subMessage_1")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let team_obj = user_info
        .get("subMessage_2")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    let read_str = |node: &serde_json::Value, key: &str| -> String {
        node.get(key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };

    let email = read_str(&user_obj, "string_3");
    let api_key = read_str(&user_obj, "string_1");
    let name = read_str(&user_obj, "string_2");
    let user_id = read_str(&user_obj, "string_6");
    let team_id_from_user = read_str(&user_obj, "string_7");

    if email.is_empty() {
        return Ok(AddAccountByAuth1TokenResult {
            success: false,
            error: Some(format!(
                "GetCurrentUser 响应未找到 email 字段（user keys: {:?}）",
                user_obj
                    .as_object()
                    .map(|o| o.keys().cloned().collect::<Vec<_>>())
                    .unwrap_or_default()
            )),
            account: Some(user_info),
            ..Default::default()
        });
    }

    let team_id = {
        let t = read_str(&team_obj, "string_1");
        if t.is_empty() { team_id_from_user } else { t }
    };
    // 常见候选：team_obj 里某个 string 字段承载 plan_name
    let plan_name = [
        "string_2", "string_3", "string_4", "string_5", "string_6", "string_7",
    ]
    .iter()
    .find_map(|k| {
        team_obj
            .get(*k)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty() && !s.contains('@'))
            .map(|s| s.to_string())
    })
    .unwrap_or_default();

    let account_json = serde_json::json!({
        "email": email,
        "name": if name.is_empty() { email.split('@').next().unwrap_or("").to_string() } else { name.clone() },
        "auth_token": effective_auth1_token,
        "session_token": post_auth.session_token,
        "devin_account_id": post_auth.account_id,
        "devin_primary_org_id": auth.devin_primary_org_id.clone(),
        "api_key": api_key,
        "user_id": user_id,
        "team_id": team_id,
        "plan_name": plan_name,
        "auth_provider": "auth1",
        "status": "active",
        "raw_user_info": user_info,
    });

    Ok(AddAccountByAuth1TokenResult {
        success: true,
        requires_org_selection: false,
        orgs: post_auth.orgs,
        auth1_token: Some(effective_auth1_token),
        account: Some(account_json),
        ..Default::default()
    })
}

#[tauri::command]
pub async fn windsurf_add_account_by_session_token(
    session_token: String,
) -> Result<AddAccountByAuth1TokenResult, String> {
    let token_trimmed = session_token.trim().to_string();
    if !token_trimmed.starts_with("devin-session-token$") {
        return Ok(AddAccountByAuth1TokenResult {
            success: false,
            error: Some("session_token 必须以 `devin-session-token$` 前缀开头".to_string()),
            ..Default::default()
        });
    }

    let auth = WindsurfAuth {
        token: token_trimmed.clone(),
        ..Default::default()
    };

    let user_info = match windsurf_get_current_user_internal(&auth).await {
        Ok(v) => v,
        Err(e) => {
            return Ok(AddAccountByAuth1TokenResult {
                success: false,
                error: Some(format!("反查账号信息失败: {}", e)),
                ..Default::default()
            });
        }
    };

    let user_obj = user_info
        .get("subMessage_1")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let team_obj = user_info
        .get("subMessage_2")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    let read_str = |node: &serde_json::Value, key: &str| -> String {
        node.get(key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };

    let email = read_str(&user_obj, "string_3");
    let api_key = read_str(&user_obj, "string_1");
    let name = read_str(&user_obj, "string_2");
    let user_id = read_str(&user_obj, "string_6");
    let team_id_from_user = read_str(&user_obj, "string_7");

    if email.is_empty() {
        return Ok(AddAccountByAuth1TokenResult {
            success: false,
            error: Some(format!(
                "GetCurrentUser 响应未找到 email 字段（user keys: {:?}）",
                user_obj
                    .as_object()
                    .map(|o| o.keys().cloned().collect::<Vec<_>>())
                    .unwrap_or_default()
            )),
            account: Some(user_info),
            ..Default::default()
        });
    }

    let team_id = {
        let t = read_str(&team_obj, "string_1");
        if t.is_empty() { team_id_from_user } else { t }
    };
    let plan_name = [
        "string_2", "string_3", "string_4", "string_5", "string_6", "string_7",
    ]
    .iter()
    .find_map(|k| {
        team_obj
            .get(*k)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty() && !s.contains('@'))
            .map(|s| s.to_string())
    })
    .unwrap_or_default();

    let account_json = serde_json::json!({
        "email": email,
        "name": if name.is_empty() { email.split('@').next().unwrap_or("").to_string() } else { name.clone() },
        "auth_token": token_trimmed,
        "session_token": token_trimmed,
        "devin_account_id": serde_json::Value::Null,
        "devin_primary_org_id": serde_json::Value::Null,
        "api_key": api_key,
        "user_id": user_id,
        "team_id": team_id,
        "plan_name": plan_name,
        "auth_provider": "auth1",
        "status": "active",
        "raw_user_info": user_info,
    });

    Ok(AddAccountByAuth1TokenResult {
        success: true,
        requires_org_selection: false,
        account: Some(account_json),
        ..Default::default()
    })
}

// ==================== 使用分析（GetAnalytics）====================

/// 通用 Protobuf 消息 → `serde_json::Value` 解析器。
///
/// 不做业务字段语义映射（不同 RPC 的字段编号各异），仅按 wire_type 生成可读的
/// JSON 结构，字段名按 `int_<N>`、`string_<N>`、`subMessage_<N>`、`double_<N>`、
/// `float_<N>`、`bytes_<N>` 的约定命名，方便前端按编号路径取值。
///
/// 重复字段会自动聚合为数组（第二次出现同名字段时转数组，第三次起 push）。
struct WindsurfProtoParser<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> WindsurfProtoParser<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn parse_message(&mut self) -> Result<serde_json::Value, String> {
        let mut map = serde_json::Map::new();
        while self.pos < self.data.len() {
            let (tag, consumed) = match decode_protobuf_varint(self.data, self.pos) {
                Some(v) => v,
                None => break,
            };
            self.pos += consumed;
            let field_no = (tag >> 3) as u32;
            let wire_type = (tag & 0x07) as u8;
            if field_no == 0 {
                break;
            }

            let (value, name) = match wire_type {
                0 => {
                    let (v, c) = decode_protobuf_varint(self.data, self.pos)
                        .ok_or_else(|| "读取 varint 失败".to_string())?;
                    self.pos += c;
                    (serde_json::json!(v), format!("int_{}", field_no))
                }
                1 => {
                    if self.pos + 8 > self.data.len() {
                        return Err("数据不足以读取 fixed64".to_string());
                    }
                    let mut arr = [0u8; 8];
                    arr.copy_from_slice(&self.data[self.pos..self.pos + 8]);
                    self.pos += 8;
                    (
                        serde_json::json!(f64::from_le_bytes(arr)),
                        format!("double_{}", field_no),
                    )
                }
                2 => {
                    let (len, cl) = decode_protobuf_varint(self.data, self.pos)
                        .ok_or_else(|| "读取 length 失败".to_string())?;
                    self.pos += cl;
                    let len = len as usize;
                    if self.pos + len > self.data.len() {
                        return Err("length-delimited 数据越界".to_string());
                    }
                    let payload = &self.data[self.pos..self.pos + len];
                    self.pos += len;

                    // 优先当嵌套消息解析；解析失败或空结果则回退为字符串/字节
                    let mut sub_parser = WindsurfProtoParser::new(payload);
                    if let Ok(sub) = sub_parser.parse_message() {
                        if sub.as_object().map_or(false, |o| !o.is_empty())
                            && sub_parser.pos == payload.len()
                        {
                            (sub, format!("subMessage_{}", field_no))
                        } else if let Ok(text) = std::str::from_utf8(payload) {
                            if !text.is_empty()
                                && text
                                    .chars()
                                    .all(|c| c.is_ascii_graphic() || c.is_ascii_whitespace())
                            {
                                (serde_json::json!(text), format!("string_{}", field_no))
                            } else {
                                (
                                    serde_json::json!(payload.to_vec()),
                                    format!("bytes_{}", field_no),
                                )
                            }
                        } else {
                            (
                                serde_json::json!(payload.to_vec()),
                                format!("bytes_{}", field_no),
                            )
                        }
                    } else if let Ok(text) = std::str::from_utf8(payload) {
                        if !text.is_empty() {
                            (serde_json::json!(text), format!("string_{}", field_no))
                        } else {
                            (
                                serde_json::json!(payload.to_vec()),
                                format!("bytes_{}", field_no),
                            )
                        }
                    } else {
                        (
                            serde_json::json!(payload.to_vec()),
                            format!("bytes_{}", field_no),
                        )
                    }
                }
                5 => {
                    if self.pos + 4 > self.data.len() {
                        return Err("数据不足以读取 fixed32".to_string());
                    }
                    let mut arr = [0u8; 4];
                    arr.copy_from_slice(&self.data[self.pos..self.pos + 4]);
                    self.pos += 4;
                    (
                        serde_json::json!(f32::from_le_bytes(arr)),
                        format!("float_{}", field_no),
                    )
                }
                _ => {
                    return Err(format!("不支持的 wire_type: {}", wire_type));
                }
            };

            // 重复字段自动聚合为数组
            if map.contains_key(&name) {
                let existing = map.get_mut(&name).unwrap();
                if !existing.is_array() {
                    let clone = existing.clone();
                    *existing = serde_json::json!([clone]);
                }
                if let Some(arr) = existing.as_array_mut() {
                    arr.push(value);
                }
            } else {
                map.insert(name, value);
            }
        }
        Ok(serde_json::Value::Object(map))
    }
}

/// 构造 `GetAnalyticsRequest` Protobuf body。
///
/// 包含 cascade_stats / lines / tool_usage / runs / summary + 完成统计 + 按日分组（Asia/Shanghai 时区）+
/// 可选的 percent_code_written（团队账号）+ 起止 timestamp。字节序列对齐参考项目实现。
fn build_get_analytics_body(start_ts: i64, end_ts: i64, is_team: bool) -> Vec<u8> {
    let mut body: Vec<u8> = Vec::new();

    // Cascade 相关（与官网前端一致）
    body.extend_from_slice(&[0x12, 0x03, 0xA2, 0x01, 0x00]); // cascade_stats (field 20)
    body.extend_from_slice(&[0x12, 0x03, 0xBA, 0x01, 0x00]); // cascade_lines (field 23)
    body.extend_from_slice(&[0x12, 0x03, 0xC2, 0x01, 0x00]); // cascade_tool_usage (field 24)
    body.extend_from_slice(&[0x12, 0x03, 0xCA, 0x01, 0x00]); // cascade_runs (field 25)
    body.extend_from_slice(&[0x12, 0x03, 0xFA, 0x01, 0x00]); // cascade_summary (field 31)

    // 基础统计
    body.extend_from_slice(&[0x12, 0x02, 0x0A, 0x00]); // completion_stats (field 1)
    body.extend_from_slice(&[0x12, 0x02, 0x1A, 0x00]); // completions_by_language (field 3)
    body.extend_from_slice(&[0x12, 0x02, 0x62, 0x00]); // chats_by_model (field 12)

    // 带 time_zone 的按日查询
    let tz = b"Asia/Shanghai";
    let tz_msg_len = (2 + tz.len()) as u8;
    let query_msg_len = (2 + tz_msg_len as usize) as u8;

    // completions_by_day (field 2)
    body.push(0x12);
    body.push(query_msg_len);
    body.push(0x12);
    body.push(tz_msg_len);
    body.push(0x0A);
    body.push(tz.len() as u8);
    body.extend_from_slice(tz);

    // chats_by_day (field 10)
    body.push(0x12);
    body.push(query_msg_len);
    body.push(0x52);
    body.push(tz_msg_len);
    body.push(0x0A);
    body.push(tz.len() as u8);
    body.extend_from_slice(tz);

    if is_team {
        body.extend_from_slice(&[0x12, 0x02, 0x7A, 0x00]); // percent_code_written (field 15)
    }

    // Timestamps：内嵌 google.protobuf.Timestamp 消息（field 1 = seconds）
    let mut ts_start: Vec<u8> = Vec::new();
    append_protobuf_int64_field(&mut ts_start, 1, start_ts);
    append_protobuf_nested_message(&mut body, 3, &ts_start);

    let mut ts_end: Vec<u8> = Vec::new();
    append_protobuf_int64_field(&mut ts_end, 1, end_ts);
    append_protobuf_nested_message(&mut body, 4, &ts_end);

    body
}

/// 调用 `GetAnalytics` 接口，返回原始响应字节（不做额外解析）。
async fn windsurf_get_analytics_internal(
    auth: &WindsurfAuth,
    start_ts: i64,
    end_ts: i64,
    is_team: bool,
) -> Result<Vec<u8>, String> {
    let http_client = create_http_client()?;
    let url =
        "https://web-backend.windsurf.com/exa.user_analytics_pb.UserAnalyticsService/GetAnalytics";

    let body = build_get_analytics_body(start_ts, end_ts, is_team);

    let req = http_client
        .post(url)
        .header("accept", "*/*")
        .header("accept-language", "zh-CN,zh;q=0.9")
        .header("cache-control", "no-cache")
        .header("connect-protocol-version", "1")
        .header("content-type", "application/proto")
        .header("pragma", "no-cache")
        .header("sec-fetch-dest", "empty")
        .header("sec-fetch-mode", "cors")
        .header("sec-fetch-site", "same-site")
        .header("Referer", "https://windsurf.com/")
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .body(body);

    let response = auth
        .apply_headers(req)
        .send()
        .await
        .map_err(|e| format!("GetAnalytics 请求失败: {}", e))?;

    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("读取 GetAnalytics 响应失败: {}", e))?;

    if !status.is_success() {
        return Err(format!(
            "GetAnalytics 失败: HTTP {} {}",
            status,
            String::from_utf8_lossy(&bytes)
        ));
    }

    let body_to_parse: &[u8] = if bytes.len() > 5 {
        let declared = u32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]) as usize;
        if (bytes[0] & 0x7e) == 0 && declared > 0 && declared + 5 == bytes.len() {
            &bytes[5..]
        } else {
            &bytes[..]
        }
    } else {
        &bytes[..]
    };

    Ok(body_to_parse.to_vec())
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WindsurfAnalyticsPoint {
    pub label: String,
    pub value: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WindsurfAnalyticsSeries {
    pub key: String,
    pub label: String,
    pub color: String,
    pub total: i64,
    pub points: Vec<WindsurfAnalyticsPoint>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WindsurfAnalyticsSummary {
    pub total_accepted_lines: i64,
    pub avg_daily_accepted_lines: f64,
    pub total_suggested_lines: i64,
    pub total_tokens: i64,
    pub total_sessions: i64,
    pub primary_model: String,
    pub primary_tool: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WindsurfAnalyticsCompletionStats {
    pub num_acceptances: i64,
    pub num_rejections: i64,
    pub acceptance_rate: f64,
    pub num_lines_accepted: i64,
    pub active_developer_days: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WindsurfAnalyticsChatStats {
    pub chats_sent: i64,
    pub chats_accepted: i64,
    pub chat_code_blocks_used: i64,
    pub function_explain_count: i64,
    pub function_refactor_count: i64,
    pub function_unit_tests_count: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WindsurfAnalyticsDailyCascadeLine {
    pub date: String,
    pub accepted_lines: i64,
    pub suggested_lines: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WindsurfAnalyticsToolUsageItem {
    pub tool_name: String,
    pub count: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WindsurfAnalyticsModelUsageSummaryItem {
    pub model_name: String,
    pub total_count: i64,
    pub percentage: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WindsurfAnalyticsModelUsageDetailItem {
    pub date: String,
    pub model_name: String,
    pub token_usage: i64,
    pub total_count: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WindsurfAnalyticsDayCompletionStats {
    pub num_acceptances: i64,
    pub num_rejections: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WindsurfAnalyticsCompletionsByDayItem {
    pub date: String,
    pub statistics: WindsurfAnalyticsDayCompletionStats,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WindsurfAnalyticsDayChatStats {
    pub chats_sent: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WindsurfAnalyticsChatsByDayItem {
    pub date: String,
    pub stats: WindsurfAnalyticsDayChatStats,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WindsurfAnalyticsLanguageCompletionItem {
    pub language_name: String,
    pub statistics: WindsurfAnalyticsDayCompletionStats,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WindsurfAnalyticsChatsByModelItem {
    pub model_name: String,
    pub stats: WindsurfAnalyticsDayChatStats,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WindsurfAnalyticsData {
    pub summary: WindsurfAnalyticsSummary,
    pub completion_stats: WindsurfAnalyticsCompletionStats,
    pub chat_stats: WindsurfAnalyticsChatStats,
    pub daily_cascade_lines: Vec<WindsurfAnalyticsDailyCascadeLine>,
    pub tool_usage: Vec<WindsurfAnalyticsToolUsageItem>,
    pub model_usage_summary: Vec<WindsurfAnalyticsModelUsageSummaryItem>,
    pub model_usage_details: Vec<WindsurfAnalyticsModelUsageDetailItem>,
    pub completions_by_language: Vec<WindsurfAnalyticsLanguageCompletionItem>,
    pub chats_by_model: Vec<WindsurfAnalyticsChatsByModelItem>,
    pub completions_by_day: Vec<WindsurfAnalyticsCompletionsByDayItem>,
    pub chats_by_day: Vec<WindsurfAnalyticsChatsByDayItem>,
}

fn pick_analytics_point_value(
    obj: &serde_json::Map<String, serde_json::Value>,
    prefer_flat_value: bool,
) -> Option<i64> {
    let preferred_keys = if prefer_flat_value {
        ["int_2", "int_1"]
    } else {
        ["int_1", "int_2"]
    };

    preferred_keys
        .iter()
        .find_map(|k| obj.get(*k).and_then(|v| v.as_i64()))
        .or_else(|| {
            let mut ints = obj
                .iter()
                .filter_map(|(k, v)| {
                    if !k.starts_with("int_") {
                        return None;
                    }
                    let order = k
                        .trim_start_matches("int_")
                        .parse::<u32>()
                        .unwrap_or(u32::MAX);
                    v.as_i64().map(|value| (order, value))
                })
                .collect::<Vec<_>>();
            ints.sort_by_key(|(order, _)| *order);
            ints.first().map(|(_, value)| *value)
        })
}

fn collect_analytics_points(
    node: &serde_json::Value,
    inherited_label: Option<String>,
    points: &mut Vec<WindsurfAnalyticsPoint>,
) {
    match node {
        serde_json::Value::Array(arr) => {
            for item in arr {
                collect_analytics_points(item, inherited_label.clone(), points);
            }
        }
        serde_json::Value::Object(obj) => {
            let local_label = ["string_1", "string_2", "string_3"]
                .iter()
                .find_map(|k| obj.get(*k).and_then(|v| v.as_str()))
                .map(|s| s.to_string())
                .unwrap_or_default();
            let effective_label = if !local_label.is_empty() {
                Some(local_label.clone())
            } else {
                inherited_label.clone()
            };

            let value = pick_analytics_point_value(obj, !local_label.is_empty());

            if let (Some(label), Some(value)) = (effective_label.clone(), value) {
                points.push(WindsurfAnalyticsPoint { label, value });
                return;
            }

            for value in obj.values() {
                collect_analytics_points(value, effective_label.clone(), points);
            }
        }
        _ => {}
    }
}

fn normalize_completions_by_day(
    parsed: &serde_json::Value,
) -> Vec<WindsurfAnalyticsCompletionsByDayItem> {
    if let Some(value) = wrapped_item_by_key(parsed, "subMessage_2") {
        if let Some(items) = value.get("subMessage_1").and_then(|value| value.as_array()) {
            let mut rows = items
                .iter()
                .filter_map(|item| {
                    let obj = item.as_object()?;
                    let timestamp = find_nested_timestamp(item)?;
                    let metrics = extract_numbered_int_metrics(obj);
                    if metrics.is_empty() {
                        return None;
                    }

                    Some(WindsurfAnalyticsCompletionsByDayItem {
                        date: format_analytics_day_label(timestamp),
                        statistics: WindsurfAnalyticsDayCompletionStats {
                            num_acceptances: metric_from_orders(&metrics, &[2, 1], 0),
                            num_rejections: metric_from_orders(&metrics, &[3], 1),
                        },
                    })
                })
                .collect::<Vec<_>>();

            rows.sort_by(|a, b| a.date.cmp(&b.date));
            if !rows.is_empty() {
                return rows;
            }
        }
    }

    normalize_analytics_daily_series(parsed)
        .into_iter()
        .find(|series| series.key == "subMessage_2" || series.key == "completions_by_day")
        .map(|series| {
            series
                .points
                .into_iter()
                .map(|point| WindsurfAnalyticsCompletionsByDayItem {
                    date: point.label,
                    statistics: WindsurfAnalyticsDayCompletionStats {
                        num_acceptances: point.value,
                        num_rejections: 0,
                    },
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn normalize_chats_by_day(parsed: &serde_json::Value) -> Vec<WindsurfAnalyticsChatsByDayItem> {
    if let Some(value) = wrapped_item_by_key(parsed, "subMessage_10") {
        if let Some(items) = value.get("subMessage_1").and_then(|value| value.as_array()) {
            let mut rows = items
                .iter()
                .filter_map(|item| {
                    let obj = item.as_object()?;
                    let timestamp = find_nested_timestamp(item)?;
                    let metrics = extract_numbered_int_metrics(obj);
                    if metrics.is_empty() {
                        return None;
                    }

                    Some(WindsurfAnalyticsChatsByDayItem {
                        date: format_analytics_day_label(timestamp),
                        stats: WindsurfAnalyticsDayChatStats {
                            chats_sent: metric_from_orders(&metrics, &[2, 1], 0),
                        },
                    })
                })
                .collect::<Vec<_>>();

            rows.sort_by(|a, b| a.date.cmp(&b.date));
            if !rows.is_empty() {
                return rows;
            }
        }
    }

    normalize_analytics_daily_series(parsed)
        .into_iter()
        .find(|series| series.key == "subMessage_10" || series.key == "chats_by_day")
        .map(|series| {
            series
                .points
                .into_iter()
                .map(|point| WindsurfAnalyticsChatsByDayItem {
                    date: point.label,
                    stats: WindsurfAnalyticsDayChatStats {
                        chats_sent: point.value,
                    },
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn normalize_completions_by_language(
    parsed: &serde_json::Value,
) -> Vec<WindsurfAnalyticsLanguageCompletionItem> {
    let mut aggregate = std::collections::BTreeMap::<String, (i64, i64)>::new();

    if let Some(value) = wrapped_item_by_key(parsed, "subMessage_3") {
        let mut rows = Vec::new();
        collect_named_metric_rows(value, &mut rows);

        for (language_name, metrics) in rows {
            let entry = aggregate.entry(language_name).or_insert((0, 0));
            entry.0 += metric_from_orders(&metrics, &[2, 1], 0);
            entry.1 += metric_from_orders(&metrics, &[3], 1);
        }
    }

    let mut items = aggregate
        .into_iter()
        .map(|(language_name, (num_acceptances, num_rejections))| {
            WindsurfAnalyticsLanguageCompletionItem {
                language_name,
                statistics: WindsurfAnalyticsDayCompletionStats {
                    num_acceptances,
                    num_rejections,
                },
            }
        })
        .collect::<Vec<_>>();

    items.sort_by(|a, b| {
        b.statistics
            .num_acceptances
            .cmp(&a.statistics.num_acceptances)
            .then_with(|| a.language_name.cmp(&b.language_name))
    });
    items
}

fn normalize_chats_by_model(parsed: &serde_json::Value) -> Vec<WindsurfAnalyticsChatsByModelItem> {
    let mut counts = std::collections::BTreeMap::<String, i64>::new();

    if let Some(value) = wrapped_item_by_key(parsed, "subMessage_12") {
        let mut rows = Vec::new();
        collect_named_metric_rows(value, &mut rows);

        for (model_name, metrics) in rows {
            if !looks_like_model_name(&model_name) {
                continue;
            }
            *counts.entry(model_name).or_insert(0) += metric_from_orders(&metrics, &[2, 1, 3], 0);
        }
    }

    let mut items = counts
        .into_iter()
        .map(
            |(model_name, chats_sent)| WindsurfAnalyticsChatsByModelItem {
                model_name,
                stats: WindsurfAnalyticsDayChatStats { chats_sent },
            },
        )
        .collect::<Vec<_>>();
    items.sort_by(|a, b| {
        b.stats
            .chats_sent
            .cmp(&a.stats.chats_sent)
            .then_with(|| a.model_name.cmp(&b.model_name))
    });
    items
}

fn normalize_completion_stats(
    completions_by_day: &[WindsurfAnalyticsCompletionsByDayItem],
    daily_cascade_lines: &[WindsurfAnalyticsDailyCascadeLine],
) -> WindsurfAnalyticsCompletionStats {
    let num_acceptances = completions_by_day
        .iter()
        .map(|item| item.statistics.num_acceptances)
        .sum::<i64>();
    let num_rejections = completions_by_day
        .iter()
        .map(|item| item.statistics.num_rejections)
        .sum::<i64>();
    let total_events = num_acceptances + num_rejections;
    let active_developer_days = completions_by_day
        .iter()
        .filter(|item| item.statistics.num_acceptances > 0 || item.statistics.num_rejections > 0)
        .count() as i64;

    WindsurfAnalyticsCompletionStats {
        num_acceptances,
        num_rejections,
        acceptance_rate: if total_events > 0 {
            (num_acceptances as f64 / total_events as f64) * 100.0
        } else {
            0.0
        },
        num_lines_accepted: daily_cascade_lines
            .iter()
            .map(|item| item.accepted_lines)
            .sum(),
        active_developer_days: if active_developer_days > 0 {
            active_developer_days
        } else {
            daily_cascade_lines.len() as i64
        },
    }
}

fn normalize_chat_stats(
    chats_by_day: &[WindsurfAnalyticsChatsByDayItem],
    chats_by_model: &[WindsurfAnalyticsChatsByModelItem],
) -> WindsurfAnalyticsChatStats {
    let chats_sent = if !chats_by_day.is_empty() {
        chats_by_day
            .iter()
            .map(|item| item.stats.chats_sent)
            .sum::<i64>()
    } else {
        chats_by_model
            .iter()
            .map(|item| item.stats.chats_sent)
            .sum::<i64>()
    };

    WindsurfAnalyticsChatStats {
        chats_sent,
        ..Default::default()
    }
}

fn normalize_analytics_points(node: &serde_json::Value) -> Vec<WindsurfAnalyticsPoint> {
    let mut points = Vec::new();
    collect_analytics_points(node, None, &mut points);
    points.sort_by(|a, b| a.label.cmp(&b.label));
    points.dedup_by(|a, b| a.label == b.label && a.value == b.value);
    points
}

fn format_analytics_day_label(timestamp: i64) -> String {
    chrono::DateTime::from_timestamp(timestamp, 0)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| timestamp.to_string())
}

fn find_nested_timestamp(node: &serde_json::Value) -> Option<i64> {
    match node {
        serde_json::Value::Array(arr) => arr.iter().find_map(find_nested_timestamp),
        serde_json::Value::Object(obj) => {
            if obj.len() == 1 {
                if let Some(ts) = obj.get("int_1").and_then(|v| v.as_i64()) {
                    return Some(ts);
                }
            }
            obj.values().find_map(find_nested_timestamp)
        }
        _ => None,
    }
}

fn extract_time_series_metrics(
    obj: &serde_json::Map<String, serde_json::Value>,
) -> Vec<(String, i64)> {
    let mut metrics = obj
        .iter()
        .filter_map(|(key, value)| {
            if !key.starts_with("int_") {
                return None;
            }
            value.as_i64().map(|num| (key.clone(), num))
        })
        .collect::<Vec<_>>();
    metrics.sort_by(|a, b| a.0.cmp(&b.0));
    metrics
}

fn extract_numbered_int_metrics(
    obj: &serde_json::Map<String, serde_json::Value>,
) -> Vec<(u32, i64)> {
    let mut metrics = obj
        .iter()
        .filter_map(|(key, value)| {
            if !key.starts_with("int_") {
                return None;
            }
            let order = key.trim_start_matches("int_").parse::<u32>().ok()?;
            value.as_i64().map(|num| (order, num))
        })
        .collect::<Vec<_>>();
    metrics.sort_by_key(|(order, _)| *order);
    metrics
}

fn metric_from_orders(
    metrics: &[(u32, i64)],
    preferred_orders: &[u32],
    fallback_index: usize,
) -> i64 {
    preferred_orders
        .iter()
        .find_map(|order| {
            metrics
                .iter()
                .find(|(current_order, _)| current_order == order)
                .map(|(_, value)| *value)
        })
        .or_else(|| metrics.get(fallback_index).map(|(_, value)| *value))
        .unwrap_or(0)
}

fn collect_named_metric_rows(node: &serde_json::Value, rows: &mut Vec<(String, Vec<(u32, i64)>)>) {
    match node {
        serde_json::Value::Array(arr) => {
            for item in arr {
                collect_named_metric_rows(item, rows);
            }
        }
        serde_json::Value::Object(obj) => {
            let label = value_as_label(obj);
            let metrics = extract_numbered_int_metrics(obj);

            if let Some(label) = label {
                if !metrics.is_empty() {
                    rows.push((label, metrics));
                    return;
                }
            }

            for value in obj.values() {
                collect_named_metric_rows(value, rows);
            }
        }
        _ => {}
    }
}

fn map_wrapped_series_meta(path: &str, metric_key: &str) -> (String, String, String) {
    if path.contains("subMessage_18") {
        return match metric_key {
            "int_2" => (
                "daily_accepted_lines".to_string(),
                "每日接受代码行数".to_string(),
                "#3b82f6".to_string(),
            ),
            "int_3" => (
                "daily_suggested_lines".to_string(),
                "每日建议代码行数".to_string(),
                "#10b981".to_string(),
            ),
            _ => (
                format!("{}_{}", path.replace('.', "_"), metric_key),
                format!("{} {}", path, metric_key),
                "#6366f1".to_string(),
            ),
        };
    }

    let palette = [
        "#3b82f6", "#10b981", "#f59e0b", "#8b5cf6", "#ef4444", "#14b8a6",
    ];
    let seed = path
        .bytes()
        .fold(0usize, |acc, byte| acc.wrapping_add(byte as usize))
        + metric_key
            .bytes()
            .fold(0usize, |acc, byte| acc.wrapping_add(byte as usize));
    (
        format!("{}_{}", path.replace('.', "_"), metric_key),
        format!("{} {}", path, metric_key),
        palette[seed % palette.len()].to_string(),
    )
}

fn build_wrapped_time_series(
    arr: &[serde_json::Value],
    path: &str,
) -> Option<Vec<WindsurfAnalyticsSeries>> {
    if arr.len() < 2 {
        return None;
    }

    let mut series_map = std::collections::BTreeMap::<String, Vec<(i64, i64)>>::new();
    let mut meta_map = std::collections::BTreeMap::<String, (String, String)>::new();

    for item in arr {
        let serde_json::Value::Object(obj) = item else {
            return None;
        };
        let Some(timestamp) = find_nested_timestamp(item) else {
            return None;
        };
        let metrics = extract_time_series_metrics(obj);
        if metrics.is_empty() {
            return None;
        }
        for (metric_key, value) in metrics {
            let (series_key, label, color) = map_wrapped_series_meta(path, &metric_key);
            meta_map.insert(series_key.clone(), (label, color));
            series_map
                .entry(series_key)
                .or_default()
                .push((timestamp, value));
        }
    }

    let series = series_map
        .into_iter()
        .filter_map(|(series_key, mut entries)| {
            if entries.len() < 2 {
                return None;
            }
            entries.sort_by_key(|(timestamp, _)| *timestamp);
            let points = entries
                .into_iter()
                .map(|(timestamp, value)| WindsurfAnalyticsPoint {
                    label: format_analytics_day_label(timestamp),
                    value,
                })
                .collect::<Vec<_>>();
            let total = points.iter().map(|point| point.value).sum();
            let (label, color) = meta_map
                .remove(&series_key)
                .unwrap_or_else(|| (series_key.clone(), "#3b82f6".to_string()));
            Some(WindsurfAnalyticsSeries {
                key: series_key,
                label,
                color,
                total,
                points,
            })
        })
        .collect::<Vec<_>>();

    if series.is_empty() {
        None
    } else {
        Some(series)
    }
}

fn collect_wrapped_time_series(
    node: &serde_json::Value,
    path: &str,
    series: &mut Vec<WindsurfAnalyticsSeries>,
) {
    match node {
        serde_json::Value::Array(arr) => {
            if let Some(found) = build_wrapped_time_series(arr, path) {
                series.extend(found);
                return;
            }
            for item in arr {
                collect_wrapped_time_series(item, path, series);
            }
        }
        serde_json::Value::Object(obj) => {
            for (key, value) in obj {
                let next_path = if path.is_empty() {
                    key.to_string()
                } else {
                    format!("{}.{}", path, key)
                };
                collect_wrapped_time_series(value, &next_path, series);
            }
        }
        _ => {}
    }
}

fn normalize_analytics_daily_series(parsed: &serde_json::Value) -> Vec<WindsurfAnalyticsSeries> {
    if let Some(wrapped) = parsed.get("subMessage_1") {
        let mut wrapped_series = Vec::new();
        collect_wrapped_time_series(wrapped, "subMessage_1", &mut wrapped_series);
        if !wrapped_series.is_empty() {
            return wrapped_series;
        }
    }

    let Some(obj) = parsed.as_object() else {
        return Vec::new();
    };

    let configs = [
        ("subMessage_2", "completions_by_day", "#3b82f6"),
        ("subMessage_10", "chats_by_day", "#10b981"),
    ];

    let mut series = Vec::new();
    for (key, label, color) in configs {
        let Some(node) = obj.get(key) else {
            continue;
        };
        let points = normalize_analytics_points(node);
        if points.len() < 2 {
            continue;
        }
        let total = points.iter().map(|p| p.value).sum();
        series.push(WindsurfAnalyticsSeries {
            key: key.to_string(),
            label: label.to_string(),
            color: color.to_string(),
            total,
            points,
        });
    }
    series
}

fn analytics_wrapped_items<'a>(
    parsed: &'a serde_json::Value,
) -> Vec<&'a serde_json::Map<String, serde_json::Value>> {
    parsed
        .get("subMessage_1")
        .and_then(|value| value.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item.as_object())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn wrapped_item_by_key<'a>(
    parsed: &'a serde_json::Value,
    key: &str,
) -> Option<&'a serde_json::Value> {
    analytics_wrapped_items(parsed)
        .into_iter()
        .find_map(|obj| obj.get(key))
}

fn value_as_label(obj: &serde_json::Map<String, serde_json::Value>) -> Option<String> {
    ["string_1", "string_2", "string_3"]
        .iter()
        .find_map(|key| obj.get(*key).and_then(|value| value.as_str()))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn value_as_count(obj: &serde_json::Map<String, serde_json::Value>) -> Option<i64> {
    ["int_3", "int_2", "int_1", "int_4"]
        .iter()
        .find_map(|key| obj.get(*key).and_then(|value| value.as_i64()))
}

fn looks_like_tool_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
}

fn looks_like_model_name(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "claude", "gpt", "gemini", "deepseek", "llama", "qwen", "sonnet", "opus", "o1", "o3",
        "haiku",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
        || (lower.contains('-') && !looks_like_tool_name(value))
}

fn collect_named_count_rows(node: &serde_json::Value, rows: &mut Vec<(String, i64)>) {
    match node {
        serde_json::Value::Array(arr) => {
            for item in arr {
                collect_named_count_rows(item, rows);
            }
        }
        serde_json::Value::Object(obj) => {
            if let (Some(label), Some(count)) = (value_as_label(obj), value_as_count(obj)) {
                rows.push((label, count));
            }
            for value in obj.values() {
                collect_named_count_rows(value, rows);
            }
        }
        _ => {}
    }
}

fn collect_tool_usage_rows(
    node: &serde_json::Value,
    rows: &mut Vec<WindsurfAnalyticsToolUsageItem>,
) {
    match node {
        serde_json::Value::Array(arr) => {
            for item in arr {
                collect_tool_usage_rows(item, rows);
            }
        }
        serde_json::Value::Object(obj) => {
            let tool_name = ["string_2", "string_1", "string_3"]
                .iter()
                .find_map(|key| obj.get(*key).and_then(|value| value.as_str()))
                .map(|value| value.trim().to_string())
                .filter(|value| looks_like_tool_name(value));
            let count = obj
                .get("int_3")
                .and_then(|value| value.as_i64())
                .unwrap_or(0);

            if let Some(tool_name) = tool_name {
                if count > 0 {
                    rows.push(WindsurfAnalyticsToolUsageItem { tool_name, count });
                }
                return;
            }

            for value in obj.values() {
                collect_tool_usage_rows(value, rows);
            }
        }
        _ => {}
    }
}

fn normalize_daily_cascade_lines(
    parsed: &serde_json::Value,
) -> Vec<WindsurfAnalyticsDailyCascadeLine> {
    if let Some(value) = wrapped_item_by_key(parsed, "subMessage_18") {
        if let Some(items) = value.get("subMessage_1").and_then(|value| value.as_array()) {
            let mut lines = items
                .iter()
                .filter_map(|item| {
                    let obj = item.as_object()?;
                    let timestamp = find_nested_timestamp(item)?;
                    Some(WindsurfAnalyticsDailyCascadeLine {
                        date: format_analytics_day_label(timestamp),
                        accepted_lines: obj
                            .get("int_2")
                            .and_then(|value| value.as_i64())
                            .unwrap_or(0),
                        suggested_lines: obj
                            .get("int_3")
                            .and_then(|value| value.as_i64())
                            .unwrap_or(0),
                    })
                })
                .collect::<Vec<_>>();
            lines.sort_by(|a, b| a.date.cmp(&b.date));
            return lines;
        }
    }

    normalize_analytics_daily_series(parsed)
        .into_iter()
        .fold(
            std::collections::BTreeMap::<String, WindsurfAnalyticsDailyCascadeLine>::new(),
            |mut acc, series| {
                if series.key != "daily_accepted_lines" && series.key != "daily_suggested_lines" {
                    return acc;
                }
                for point in series.points {
                    let entry = acc.entry(point.label.clone()).or_insert_with(|| {
                        WindsurfAnalyticsDailyCascadeLine {
                            date: point.label.clone(),
                            ..Default::default()
                        }
                    });
                    if series.key == "daily_accepted_lines" {
                        entry.accepted_lines = point.value;
                    } else {
                        entry.suggested_lines = point.value;
                    }
                }
                acc
            },
        )
        .into_values()
        .collect()
}

fn normalize_tool_usage(parsed: &serde_json::Value) -> Vec<WindsurfAnalyticsToolUsageItem> {
    let mut counts = std::collections::BTreeMap::<String, i64>::new();
    let mut rows = Vec::new();
    if let Some(value) = wrapped_item_by_key(parsed, "subMessage_19") {
        collect_tool_usage_rows(value, &mut rows);
    }

    for row in rows {
        *counts.entry(row.tool_name).or_insert(0) += row.count;
    }

    let mut items = counts
        .into_iter()
        .map(|(tool_name, count)| WindsurfAnalyticsToolUsageItem { tool_name, count })
        .collect::<Vec<_>>();
    items.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.tool_name.cmp(&b.tool_name))
    });
    items
}

fn normalize_model_usage_summary(
    parsed: &serde_json::Value,
    chats_by_model: &[WindsurfAnalyticsChatsByModelItem],
) -> Vec<WindsurfAnalyticsModelUsageSummaryItem> {
    if !chats_by_model.is_empty() {
        let total = chats_by_model
            .iter()
            .map(|item| item.stats.chats_sent)
            .sum::<i64>()
            .max(1);

        let mut items = chats_by_model
            .iter()
            .map(|item| WindsurfAnalyticsModelUsageSummaryItem {
                model_name: item.model_name.clone(),
                total_count: item.stats.chats_sent,
                percentage: (item.stats.chats_sent as f64 / total as f64) * 100.0,
            })
            .collect::<Vec<_>>();

        items.sort_by(|a, b| {
            b.total_count
                .cmp(&a.total_count)
                .then_with(|| a.model_name.cmp(&b.model_name))
        });
        return items;
    }

    let mut rows = Vec::new();
    collect_named_count_rows(parsed, &mut rows);

    let mut counts = std::collections::BTreeMap::<String, i64>::new();
    for (label, count) in rows {
        if !looks_like_model_name(&label) {
            continue;
        }
        *counts.entry(label).or_insert(0) += count;
    }

    let total = counts.values().sum::<i64>().max(1);
    let mut items = counts
        .into_iter()
        .map(
            |(model_name, total_count)| WindsurfAnalyticsModelUsageSummaryItem {
                model_name,
                total_count,
                percentage: (total_count as f64 / total as f64) * 100.0,
            },
        )
        .collect::<Vec<_>>();
    items.sort_by(|a, b| {
        b.total_count
            .cmp(&a.total_count)
            .then_with(|| a.model_name.cmp(&b.model_name))
    });
    items
}

fn normalize_summary(
    daily_cascade_lines: &[WindsurfAnalyticsDailyCascadeLine],
    tool_usage: &[WindsurfAnalyticsToolUsageItem],
    model_usage_summary: &[WindsurfAnalyticsModelUsageSummaryItem],
    model_usage_details: &[WindsurfAnalyticsModelUsageDetailItem],
    chat_stats: &WindsurfAnalyticsChatStats,
) -> WindsurfAnalyticsSummary {
    let total_accepted_lines = daily_cascade_lines
        .iter()
        .map(|item| item.accepted_lines)
        .sum::<i64>();
    let total_suggested_lines = daily_cascade_lines
        .iter()
        .map(|item| item.suggested_lines)
        .sum::<i64>();
    let active_days = daily_cascade_lines.len() as f64;

    WindsurfAnalyticsSummary {
        total_accepted_lines,
        avg_daily_accepted_lines: if active_days > 0.0 {
            total_accepted_lines as f64 / active_days
        } else {
            0.0
        },
        total_suggested_lines,
        total_tokens: model_usage_details
            .iter()
            .map(|item| item.token_usage)
            .sum(),
        total_sessions: chat_stats.chats_sent,
        primary_model: model_usage_summary
            .first()
            .map(|item| item.model_name.clone())
            .unwrap_or_default(),
        primary_tool: tool_usage
            .first()
            .map(|item| item.tool_name.clone())
            .unwrap_or_default(),
    }
}

fn normalize_analytics_data(parsed: &serde_json::Value) -> WindsurfAnalyticsData {
    let daily_cascade_lines = normalize_daily_cascade_lines(parsed);
    let tool_usage = normalize_tool_usage(parsed);
    let completions_by_day = normalize_completions_by_day(parsed);
    let chats_by_day = normalize_chats_by_day(parsed);
    let completions_by_language = normalize_completions_by_language(parsed);
    let chats_by_model = normalize_chats_by_model(parsed);
    let model_usage_summary = normalize_model_usage_summary(parsed, &chats_by_model);
    let model_usage_details = Vec::new();
    let completion_stats = normalize_completion_stats(&completions_by_day, &daily_cascade_lines);
    let chat_stats = normalize_chat_stats(&chats_by_day, &chats_by_model);
    let summary = normalize_summary(
        &daily_cascade_lines,
        &tool_usage,
        &model_usage_summary,
        &model_usage_details,
        &chat_stats,
    );

    WindsurfAnalyticsData {
        summary,
        completion_stats,
        chat_stats,
        daily_cascade_lines,
        tool_usage,
        model_usage_summary: model_usage_summary.clone(),
        model_usage_details,
        completions_by_language,
        chats_by_model,
        completions_by_day,
        chats_by_day,
    }
}

/// `windsurf_get_analytics` 返回结构。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WindsurfAnalyticsResult {
    pub success: bool,
    pub error: Option<String>,
    /// 本次实际使用的 token 路径（`auth1`）。便于前端排查
    pub provider: Option<String>,
    /// 起止 Unix 秒，方便前端校对时间窗
    pub start_timestamp: i64,
    pub end_timestamp: i64,
    pub analytics_data: Option<WindsurfAnalyticsData>,
    pub daily_series: Vec<WindsurfAnalyticsSeries>,
}

/// 获取 Windsurf 账号使用分析数据（cascade / completion / chat 等指标）。
///
/// 鉴权自适应：
/// - `auth_token` 以 `auth1_` 开头 → 先调 `WindsurfPostAuth` 换取 session_token，再带 5-header 调用
/// - `auth_token` 以 `devin-session-token$` 开头 → 直接按 Session Token 调用
///
/// 时间窗：`start_timestamp` / `end_timestamp` 为 Unix 秒；建议前端传最近 30 天范围。
#[tauri::command]
pub async fn windsurf_get_analytics(
    auth_token: String,
    devin_account_id: Option<String>,
    devin_auth1_token: Option<String>,
    devin_primary_org_id: Option<String>,
    session_token: Option<String>,
    start_timestamp: i64,
    end_timestamp: i64,
    is_team: Option<bool>,
) -> Result<WindsurfAnalyticsResult, String> {
    let trimmed = auth_token.trim();
    if trimmed.is_empty() {
        return Ok(WindsurfAnalyticsResult {
            success: false,
            error: Some("缺少 auth_token".to_string()),
            start_timestamp,
            end_timestamp,
            ..Default::default()
        });
    }

    let (auth, provider) = if trimmed.starts_with("auth1_") {
        if let Some(st) = session_token
            .as_deref()
            .map(str::trim)
            .filter(|s| s.starts_with("devin-session-token$"))
        {
            (
                WindsurfAuth {
                    token: st.to_string(),
                    devin_auth1_token: Some(trimmed.to_string()),
                    devin_account_id: devin_account_id.clone(),
                    devin_primary_org_id: devin_primary_org_id.clone(),
                },
                "auth1",
            )
        } else {
            let primary_org = devin_primary_org_id
                .as_deref()
                .map(str::trim)
                .unwrap_or("")
                .to_string();
            let post_auth = match windsurf_post_auth_internal(trimmed, &primary_org).await {
                Ok(v) => v,
                Err(e) => {
                    return Ok(WindsurfAnalyticsResult {
                        success: false,
                        provider: Some("auth1".to_string()),
                        error: Some(format!("WindsurfPostAuth 失败: {}", e)),
                        start_timestamp,
                        end_timestamp,
                        ..Default::default()
                    });
                }
            };
            let effective = post_auth
                .auth1_token
                .clone()
                .unwrap_or_else(|| trimmed.to_string());
            (
                WindsurfAuth {
                    token: post_auth.session_token,
                    devin_auth1_token: Some(effective),
                    devin_account_id: post_auth.account_id.or_else(|| devin_account_id.clone()),
                    devin_primary_org_id: post_auth
                        .primary_org_id
                        .or_else(|| Some(primary_org).filter(|s| !s.is_empty()))
                        .or_else(|| post_auth.orgs.first().map(|o| o.id.clone())),
                },
                "auth1",
            )
        }
    } else if trimmed.starts_with("devin-session-token$") {
        (
            WindsurfAuth {
                token: trimmed.to_string(),
                devin_auth1_token: devin_auth1_token
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string()),
                devin_account_id: devin_account_id.clone(),
                devin_primary_org_id: devin_primary_org_id.clone(),
            },
            "auth1",
        )
    } else if let Some(auth1) = devin_auth1_token
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        (
            WindsurfAuth {
                token: trimmed.to_string(),
                devin_auth1_token: Some(auth1.to_string()),
                devin_account_id: devin_account_id.clone(),
                devin_primary_org_id: devin_primary_org_id.clone(),
            },
            "auth1",
        )
    } else {
        return Ok(WindsurfAnalyticsResult {
            success: false,
            error: Some("仅支持 auth1_ 或 devin-session-token$ 开头的 Token".to_string()),
            start_timestamp,
            end_timestamp,
            ..Default::default()
        });
    };

    match windsurf_get_analytics_internal(
        &auth,
        start_timestamp,
        end_timestamp,
        is_team.unwrap_or(false),
    )
    .await
    {
        Ok(bytes) => {
            let parsed = WindsurfProtoParser::new(&bytes).parse_message().ok();
            let analytics_data = parsed.as_ref().map(normalize_analytics_data);
            let daily_series = parsed
                .as_ref()
                .map(normalize_analytics_daily_series)
                .unwrap_or_default();
            Ok(WindsurfAnalyticsResult {
                success: true,
                provider: Some(provider.to_string()),
                start_timestamp,
                end_timestamp,
                analytics_data,
                daily_series,
                ..Default::default()
            })
        }
        Err(e) => Ok(WindsurfAnalyticsResult {
            success: false,
            provider: Some(provider.to_string()),
            error: Some(e),
            start_timestamp,
            end_timestamp,
            ..Default::default()
        }),
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WindsurfCreditsResult {
    pub success: bool,
    pub plan_name: Option<String>,
    pub teams_tier: Option<i64>,
    pub teams_tier_name: Option<String>,
    /// 历史字段：兼容旧 UI；含义 = available_prompt_credits / 100
    pub used_credits: Option<i64>,
    pub total_credits: Option<i64>,
    // 细分 credit 字段
    pub used_prompt_credits: Option<i64>,
    pub available_prompt_credits: Option<i64>,
    pub used_flow_credits: Option<i64>,
    pub available_flow_credits: Option<i64>,
    pub used_flex_credits: Option<i64>,
    pub available_flex_credits: Option<i64>,
    pub monthly_prompt_credits: Option<i64>,
    pub monthly_flow_credits: Option<i64>,
    // 日/周配额
    pub daily_quota_remaining_percent: Option<i64>,
    pub weekly_quota_remaining_percent: Option<i64>,
    pub daily_quota_reset_at_unix: Option<i64>,
    pub weekly_quota_reset_at_unix: Option<i64>,
    // 周期
    pub expires_at: Option<String>,
    pub plan_start: Option<String>,
    pub plan_start_unix: Option<i64>,
    pub plan_end_unix: Option<i64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetWindsurfMachineIdResponse {
    pub success: bool,
    pub machine_id: String,
    pub mac_machine_id: Option<String>,
    pub sqm_id: String,
    pub dev_device_id: String,
    pub error: Option<String>,
}

fn generate_uuid() -> String {
    Uuid::new_v4().to_string()
}

fn generate_random_hex(byte_len: usize) -> String {
    let mut bytes = vec![0u8; byte_len];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

// ==================== Windsurf 客户端目标（客户端类型 / 自定义安装路径 / 自定义用户数据目录） ====================

/// 前端可选传入的 Windsurf 客户端定位信息。
///
/// 所有字段均可省略：
/// - `client_type` 缺省等价于 `"windsurf"`（标准版），合法值为 `"windsurf"` / `"windsurf-next"`；
/// - `install_path` 用于补丁 / 路径识别（当前未直接用于 OAuth 登录）；
/// - `user_data_dir` 用于覆盖 `storage.json` 所在的用户数据根目录，便于支持
///   便携模式 / `--user-data-dir` 等自定义场景。
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindsurfTarget {
    #[serde(default)]
    pub client_type: Option<String>,
    /// 仅为将来补丁 / 路径识别场景预留；当前 OAuth 登录流程不直接使用此字段。
    #[serde(default)]
    #[allow(dead_code)]
    pub install_path: Option<String>,
    #[serde(default)]
    pub user_data_dir: Option<String>,
}

impl WindsurfTarget {
    /// 规范化客户端类型字符串，未知值归并为 `"windsurf"`。
    fn normalized_client_type(&self) -> &str {
        match self.client_type.as_deref().map(str::trim) {
            Some("windsurf-next") => "windsurf-next",
            _ => "windsurf",
        }
    }

    /// 对应 OAuth 回调使用的 URL scheme。
    fn url_scheme(&self) -> &'static str {
        match self.normalized_client_type() {
            "windsurf-next" => "windsurf-next",
            _ => "windsurf",
        }
    }

    /// 默认用户数据目录名（`%APPDATA%` / `~/Library/Application Support` / `$XDG_CONFIG_HOME` 下的子目录名）。
    fn data_dir_name(&self) -> &'static str {
        match self.normalized_client_type() {
            "windsurf-next" => "Windsurf - Next",
            _ => "Windsurf",
        }
    }
}

/// 解析用户数据目录：优先使用显式传入的 `user_data_dir`，否则按 OS + 客户端类型拼默认位置。
fn resolve_windsurf_user_data_dir(target: &WindsurfTarget) -> Result<PathBuf, String> {
    if let Some(custom) = target
        .user_data_dir
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return Ok(PathBuf::from(custom));
    }

    let name = target.data_dir_name();

    #[cfg(target_os = "windows")]
    {
        let base = dirs::data_dir().ok_or("无法获取系统数据目录")?;
        Ok(base.join(name))
    }

    #[cfg(target_os = "macos")]
    {
        let home = dirs::home_dir().ok_or("无法获取用户目录")?;
        Ok(home.join("Library").join("Application Support").join(name))
    }

    #[cfg(target_os = "linux")]
    {
        let base = dirs::config_dir().ok_or("无法获取系统配置目录")?;
        Ok(base.join(name))
    }
}

/// 解析 `storage.json`（机器 ID 重置写入目标）路径。
fn resolve_storage_json_path(target: &WindsurfTarget) -> Result<PathBuf, String> {
    Ok(resolve_windsurf_user_data_dir(target)?
        .join("User")
        .join("globalStorage")
        .join("storage.json"))
}

/// Windsurf 安装目录内 `extension.js` 的相对路径。
/// 通过它的存在性来判定某个目录是否确实是 Windsurf 安装目录。
fn extension_js_relative_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        PathBuf::from("Contents")
            .join("Resources")
            .join("app")
            .join("extensions")
            .join("windsurf")
            .join("dist")
            .join("extension.js")
    }
    #[cfg(not(target_os = "macos"))]
    {
        PathBuf::from("resources")
            .join("app")
            .join("extensions")
            .join("windsurf")
            .join("dist")
            .join("extension.js")
    }
}

/// 通过 `extension.js` 存在性校验某路径是否为有效 Windsurf 安装目录。
fn is_valid_windsurf_install_dir(path: &PathBuf) -> bool {
    path.join(extension_js_relative_path()).exists()
}

/// 在 Windows 上通过 PowerShell COM 调用解析 `.lnk` 快捷方式的 TargetPath。
#[cfg(target_os = "windows")]
fn resolve_lnk_target(lnk_path: &PathBuf) -> Option<PathBuf> {
    let script = format!(
        "$sh = New-Object -ComObject WScript.Shell; $sh.CreateShortcut('{}').TargetPath",
        lnk_path.display()
    );
    let out = command_hidden("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let target = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if target.is_empty() {
        None
    } else {
        Some(PathBuf::from(target))
    }
}

/// 按常见路径探测指定客户端类型的 Windsurf 安装目录。命中即返回。
fn detect_windsurf_install_path_impl(client_type: &str) -> Option<String> {
    let target = WindsurfTarget {
        client_type: Some(client_type.to_string()),
        ..Default::default()
    };
    let dir_name = target.data_dir_name(); // "Windsurf" / "Windsurf - Next"

    #[cfg(target_os = "windows")]
    {
        // 1) 开始菜单快捷方式 → TargetPath → parent
        if let Ok(appdata) = std::env::var("APPDATA") {
            let start_menu = PathBuf::from(appdata)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs")
                .join(dir_name);
            if let Ok(entries) = fs::read_dir(&start_menu) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("lnk") {
                        if let Some(exe) = resolve_lnk_target(&path) {
                            if let Some(parent) = exe.parent() {
                                let root = parent.to_path_buf();
                                if is_valid_windsurf_install_dir(&root) {
                                    return Some(root.to_string_lossy().to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2) 常见安装路径
        let mut candidates: Vec<PathBuf> = Vec::new();
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            candidates.push(PathBuf::from(local).join("Programs").join(dir_name));
        }
        candidates.push(PathBuf::from("C:\\Program Files").join(dir_name));
        candidates.push(PathBuf::from("C:\\Program Files (x86)").join(dir_name));

        for c in candidates {
            if is_valid_windsurf_install_dir(&c) {
                return Some(c.to_string_lossy().to_string());
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let app_name = format!("{}.app", dir_name);
        let mut candidates: Vec<PathBuf> =
            vec![PathBuf::from("/Applications").join(&app_name)];
        if let Some(home) = dirs::home_dir() {
            candidates.push(home.join("Applications").join(&app_name));
        }
        for c in candidates {
            if is_valid_windsurf_install_dir(&c) {
                return Some(c.to_string_lossy().to_string());
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let mut candidates: Vec<PathBuf> = vec![
            PathBuf::from("/opt").join(dir_name),
            PathBuf::from("/usr/share").join(dir_name.to_lowercase()),
        ];
        if let Some(home) = dirs::home_dir() {
            candidates.push(home.join(".local").join("share").join(dir_name));
        }
        for c in candidates {
            if is_valid_windsurf_install_dir(&c) {
                return Some(c.to_string_lossy().to_string());
            }
        }
    }

    None
}

/// 重置 Windsurf 的 `telemetry.machineId` / `sqmId` / `devDeviceId` / `macMachineId`。
///
/// `windsurf_target` 可选：指定客户端类型（标准版 / Next）与自定义用户数据目录，
/// 覆盖默认的 `~/Library/Application Support/Windsurf/...` 等路径，用于支持便携模式
/// 或 `--user-data-dir` 启动。
#[tauri::command]
pub async fn reset_windsurf_machine_id(
    windsurf_target: Option<WindsurfTarget>,
) -> Result<ResetWindsurfMachineIdResponse, String> {
    let target = windsurf_target.unwrap_or_default();
    let storage_path = resolve_storage_json_path(&target)?;

    let fail = |msg: String| ResetWindsurfMachineIdResponse {
        success: false,
        machine_id: String::new(),
        mac_machine_id: None,
        sqm_id: String::new(),
        dev_device_id: String::new(),
        error: Some(msg),
    };

    let parent = storage_path
        .parent()
        .ok_or_else(|| "无法解析 storage.json 目录".to_string())?;

    if !parent.exists() {
        return Ok(fail(format!(
            "Windsurf 用户数据目录不存在，请确认已安装并至少启动过一次，或在设置中指定正确的用户数据目录: {}",
            parent.display()
        )));
    }

    let mut storage: serde_json::Map<String, serde_json::Value> = if storage_path.exists() {
        let content = match fs::read_to_string(&storage_path) {
            Ok(c) => c,
            Err(e) => {
                return Ok(fail(format!(
                    "读取 storage.json 失败: {} (path: {})",
                    e,
                    storage_path.display()
                )));
            }
        };

        match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(e) => {
                return Ok(fail(format!(
                    "解析 storage.json 失败: {} (path: {})",
                    e,
                    storage_path.display()
                )));
            }
        }
    } else {
        serde_json::Map::new()
    };

    let new_machine_id = generate_random_hex(32);
    let new_mac_machine_id = generate_random_hex(16);
    let new_sqm_id = generate_uuid().to_uppercase();
    let new_dev_device_id = generate_uuid().to_lowercase();

    storage.insert(
        "telemetry.machineId".to_string(),
        serde_json::Value::String(new_machine_id.clone()),
    );
    storage.insert(
        "telemetry.sqmId".to_string(),
        serde_json::Value::String(new_sqm_id.clone()),
    );
    storage.insert(
        "telemetry.devDeviceId".to_string(),
        serde_json::Value::String(new_dev_device_id.clone()),
    );
    storage.insert(
        "telemetry.macMachineId".to_string(),
        serde_json::Value::String(new_mac_machine_id.clone()),
    );

    let new_content = match serde_json::to_string_pretty(&storage) {
        Ok(s) => s,
        Err(e) => {
            return Ok(fail(format!(
                "序列化 storage.json 失败: {} (path: {})",
                e,
                storage_path.display()
            )));
        }
    };

    if storage_path.exists() {
        if let Ok(meta) = fs::metadata(&storage_path) {
            let mut perms = meta.permissions();
            if perms.readonly() {
                perms.set_readonly(false);
                let _ = fs::set_permissions(&storage_path, perms);
            }
        }
    }

    let mut last_err: Option<String> = None;
    for i in 0..5 {
        match fs::write(&storage_path, &new_content) {
            Ok(_) => {
                last_err = None;
                break;
            }
            Err(e) => {
                last_err = Some(format!("{}", e));
                if i < 4 {
                    sleep(Duration::from_millis(250 * (i as u64 + 1))).await;
                }
            }
        }
    }

    if let Some(e) = last_err {
        return Ok(fail(format!(
            "写入 storage.json 失败: {} (path: {})。请关闭 Windsurf 后重试，或检查文件权限。",
            e,
            storage_path.display()
        )));
    }

    Ok(ResetWindsurfMachineIdResponse {
        success: true,
        machine_id: new_machine_id,
        mac_machine_id: Some(new_mac_machine_id),
        sqm_id: new_sqm_id,
        dev_device_id: new_dev_device_id,
        error: None,
    })
}

/// 获取 Windsurf 订阅 / 用量信息（GetPlanStatus）。
///
/// 请求格式对齐官网前端 & chaogei/windsurf-account-manager-simple：
/// `content-type: application/proto` + body `0x0a + varint(len) + token_bytes`
/// （即 Protobuf `GetPlanStatusRequest { auth_token = 1 }`）。响应为 Protobuf
/// `GetPlanStatusResponse { PlanStatus plan_status = 1 }`，按字段编号解析。
///
/// 鉴权自适应：
/// - `auth_token` 是 `auth1_*` / `devin-session-token$*` 或带了 devin_auth1_token →
///   先换 session_token 再组 5-header（x-auth-token + x-devin-*）
#[tauri::command]
pub async fn windsurf_refresh_credits(
    auth_token: String,
    devin_account_id: Option<String>,
    devin_auth1_token: Option<String>,
    devin_primary_org_id: Option<String>,
    session_token: Option<String>,
) -> Result<WindsurfCreditsResult, String> {
    let http_client = create_http_client()?;
    let trimmed = auth_token.trim().to_string();

    let fail = |msg: String| WindsurfCreditsResult {
        success: false,
        error: Some(msg),
        ..Default::default()
    };

    let explicit_auth1 = devin_auth1_token
        .as_deref()
        .map(str::trim)
        .filter(|s| s.starts_with("auth1_"))
        .map(|s| s.to_string());
    let use_auth1 =
        trimmed.starts_with("auth1_") || trimmed.starts_with("devin-session-token$") || explicit_auth1.is_some();
    if !use_auth1 {
        return Ok(fail("仅支持 auth1_ 或 devin-session-token$ 开头的 Token".to_string()));
    }

    // 构造鉴权上下文
    let primary_org = devin_primary_org_id
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .to_string();
    let auth1_for_post = if trimmed.starts_with("auth1_") {
        trimmed.clone()
    } else {
        explicit_auth1.clone().unwrap_or_default()
    };
    let session_candidate = session_token
        .as_deref()
        .map(str::trim)
        .filter(|s| s.starts_with("devin-session-token$"))
        .map(|s| s.to_string())
        .or_else(|| {
            if trimmed.starts_with("devin-session-token$") {
                Some(trimmed.clone())
            } else {
                None
            }
        });

    let auth = if let Some(st) = session_candidate {
        WindsurfAuth {
            token: st,
            devin_auth1_token: if auth1_for_post.is_empty() {
                None
            } else {
                Some(auth1_for_post.clone())
            },
            devin_account_id: devin_account_id.clone(),
            devin_primary_org_id: devin_primary_org_id.clone(),
        }
    } else if !auth1_for_post.is_empty() {
        let post_auth = match windsurf_post_auth_internal(&auth1_for_post, &primary_org).await {
            Ok(v) => v,
            Err(e) => return Ok(fail(format!("WindsurfPostAuth 失败: {}", e))),
        };
        let effective_auth1 = post_auth
            .auth1_token
            .clone()
            .unwrap_or_else(|| auth1_for_post.clone());
        WindsurfAuth {
            token: post_auth.session_token,
            devin_auth1_token: Some(effective_auth1),
            devin_account_id: post_auth.account_id.or(devin_account_id.clone()),
            devin_primary_org_id: post_auth
                .primary_org_id
                .or_else(|| Some(primary_org).filter(|s| !s.is_empty()))
                .or_else(|| post_auth.orgs.first().map(|o| o.id.clone())),
        }
    } else {
        return Ok(fail(
            "Auth1 路径缺少 session_token 且未提供 auth1_token".to_string(),
        ));
    };

    // 构造 Protobuf body: field 1 (string) = auth.token
    let token_bytes = auth.token.as_bytes();
    let mut body: Vec<u8> = Vec::with_capacity(token_bytes.len() + 4);
    body.push(0x0a);
    encode_protobuf_varint(token_bytes.len(), &mut body);
    body.extend_from_slice(token_bytes);

    let req = http_client
        .post(WINDSURF_PLAN_STATUS_API)
        .header("accept", "*/*")
        .header("accept-language", "zh-CN,zh;q=0.9")
        .header("cache-control", "no-cache")
        .header("connect-protocol-version", "1")
        .header("content-type", "application/proto")
        .header("origin", "https://windsurf.com")
        .header("pragma", "no-cache")
        .header("priority", "u=1, i")
        .header("referer", "https://windsurf.com/")
        .header("sec-fetch-dest", "empty")
        .header("sec-fetch-mode", "cors")
        .header("sec-fetch-site", "same-site")
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .header("x-debug-email", "")
        .header("x-debug-team-name", "")
        .body(body);

    let response = auth
        .apply_headers(req)
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))?;

    if !status.is_success() {
        return Ok(fail(format!(
            "GetPlanStatus HTTP {}: {}",
            status,
            String::from_utf8_lossy(&bytes)
        )));
    }

    // 兼容 gRPC-Web 5 字节 envelope
    let body_to_parse: &[u8] = if bytes.len() > 5 {
        let declared = u32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]) as usize;
        if (bytes[0] & 0x7e) == 0 && declared > 0 && declared + 5 == bytes.len() {
            &bytes[5..]
        } else {
            &bytes[..]
        }
    } else {
        &bytes[..]
    };

    let mut parser = WindsurfProtoParser::new(body_to_parse);
    let parsed = parser
        .parse_message()
        .map_err(|e| format!("解析 GetPlanStatus 响应失败: {}", e))?;

    // PlanStatus 位于 subMessage_1
    let plan_status = parsed
        .get("subMessage_1")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let plan_info = plan_status
        .get("subMessage_1")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    let read_i64 = |node: &serde_json::Value, key: &str| -> Option<i64> {
        node.get(key).and_then(|v| v.as_i64())
    };
    let read_str = |node: &serde_json::Value, key: &str| -> Option<String> {
        node.get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    };
    let read_nested_i64 =
        |node: &serde_json::Value, sub_key: &str, inner_key: &str| -> Option<i64> {
            node.get(sub_key)
                .and_then(|v| v.get(inner_key))
                .and_then(|v| v.as_i64())
        };

    // PlanInfo
    let teams_tier = read_i64(&plan_info, "int_1");
    let teams_tier_name = teams_tier.map(|t| {
        match t {
            0 => "UNSPECIFIED",
            1 => "TEAMS",
            2 => "PRO",
            3 => "ENTERPRISE_SAAS",
            4 => "HYBRID",
            5 => "ENTERPRISE_SELF_HOSTED",
            6 => "WAITLIST_PRO",
            7 => "TEAMS_ULTIMATE",
            8 => "PRO_ULTIMATE",
            9 => "TRIAL",
            10 => "ENTERPRISE_SELF_SERVE",
            _ => "UNKNOWN",
        }
        .to_string()
    });
    let plan_name = read_str(&plan_info, "string_2").or_else(|| Some("Free".to_string()));
    let monthly_prompt_credits = read_i64(&plan_info, "int_12");
    let monthly_flow_credits = read_i64(&plan_info, "int_13");

    // PlanStatus 本体
    let available_flex_credits = read_i64(&plan_status, "int_4");
    let used_flow_credits = read_i64(&plan_status, "int_5");
    let used_prompt_credits = read_i64(&plan_status, "int_6");
    let used_flex_credits = read_i64(&plan_status, "int_7");
    let available_prompt_credits = read_i64(&plan_status, "int_8");
    let available_flow_credits = read_i64(&plan_status, "int_9");

    let daily_quota_remaining_percent = read_i64(&plan_status, "int_14");
    let weekly_quota_remaining_percent = read_i64(&plan_status, "int_15");
    let daily_quota_reset_at_unix = read_i64(&plan_status, "int_17");
    let weekly_quota_reset_at_unix = read_i64(&plan_status, "int_18");

    // 周期 Timestamp 内嵌：subMessage_2.int_1 = plan_start（秒），subMessage_3.int_1 = plan_end（秒）
    let plan_start_unix = read_nested_i64(&plan_status, "subMessage_2", "int_1");
    let plan_end_unix = read_nested_i64(&plan_status, "subMessage_3", "int_1");
    let plan_start = plan_start_unix.map(|n| {
        chrono::DateTime::from_timestamp(n, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| n.to_string())
    });
    let expires_at = plan_end_unix.map(|n| {
        chrono::DateTime::from_timestamp(n, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| n.to_string())
    });

    // 兼容旧 UI 字段：used_credits = used_prompt / 100; total_credits = available_prompt / 100
    let (used_credits, total_credits) = match (used_prompt_credits, available_prompt_credits) {
        (Some(u), Some(a)) => (Some(u / 100), Some(a / 100)),
        (Some(u), None) => (Some(u / 100), None),
        (None, Some(a)) => (None, Some(a / 100)),
        _ => (None, None),
    };

    Ok(WindsurfCreditsResult {
        success: true,
        plan_name,
        teams_tier,
        teams_tier_name,
        used_credits,
        total_credits,
        used_prompt_credits,
        available_prompt_credits,
        used_flow_credits,
        available_flow_credits,
        used_flex_credits,
        available_flex_credits,
        monthly_prompt_credits,
        monthly_flow_credits,
        daily_quota_remaining_percent,
        weekly_quota_remaining_percent,
        daily_quota_reset_at_unix,
        weekly_quota_reset_at_unix,
        expires_at,
        plan_start,
        plan_start_unix,
        plan_end_unix,
        error: None,
    })
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WindsurfSwitchAccountResult {
    pub success: bool,
    pub error: Option<String>,
    /// 本次实际采用的登录路径："auth1" (Devin)
    pub provider: Option<String>,
    /// Windsurf 后端颁发的一次性 auth_token（仅用于 URL scheme 登录，成功后立即失效）
    pub one_time_auth_token: Option<String>,
    /// 实际触发的 `windsurf://` 回调 URL，便于前端调试或在 URL scheme 不可用时引导用户手动打开
    pub callback_url: Option<String>,
    /// 若服务端轮换过 auth1_token，将新值回写给前端
    pub auth_token: Option<String>,
    /// Devin 路径新换取的 session_token，前端可缓存供后续 API 复用（过期可再次 post_auth）
    pub session_token: Option<String>,
    /// Devin 路径解析出的 account_id（Devin 账号体系专用）
    pub devin_account_id: Option<String>,
    /// Devin 路径解析出的 primary_org_id（Devin 账号体系专用）
    pub devin_primary_org_id: Option<String>,
    pub machine_id_reset: Option<bool>,
    pub machine_id_reset_error: Option<String>,
}

/// 通过 `windsurf://` URL scheme 完成 Windsurf 桌面客户端的 OAuth 回调登录。
///
/// 统一入口，按 `auth_token` 前缀自动分流：
/// - **Devin Auth1 路径**（`auth_token` 以 `auth1_` 开头）：
///   1. 调 `WindsurfPostAuth(auth_token, devin_primary_org_id?)` 换 session_token + 扩展字段
///   2. 组装 5-header [`WindsurfAuth`] 调 `GetOneTimeAuthToken` 拿一次性 token
///   3. 打开 `windsurf://codeium.windsurf#access_token=<one_time>&state=<uuid>&token_type=Bearer`
///   4. 返回新 session_token / account_id / primary_org_id 供前端回写
/// - **Devin Session Token 路径**（`auth_token` 以 `devin-session-token$` 开头）
#[tauri::command]
pub async fn windsurf_switch_account(
    auth_token: String,
    _refresh_token: Option<String>,
    devin_account_id: Option<String>,
    devin_primary_org_id: Option<String>,
    windsurf_target: Option<WindsurfTarget>,
) -> Result<WindsurfSwitchAccountResult, String> {
    let trimmed = auth_token.trim();
    if trimmed.is_empty() {
        return Ok(WindsurfSwitchAccountResult {
            success: false,
            error: Some("缺少 auth_token，无法发起 Windsurf 登录".to_string()),
            ..Default::default()
        });
    }

    if !trimmed.starts_with("auth1_") && !trimmed.starts_with("devin-session-token$") {
        return Ok(WindsurfSwitchAccountResult {
            success: false,
            error: Some("仅支持 auth1_ 或 devin-session-token$ 开头的 Token".to_string()),
            ..Default::default()
        });
    }

    let target = windsurf_target.unwrap_or_default();

    let reset_result = reset_windsurf_machine_id(Some(target.clone())).await?;
    let machine_id_reset = reset_result.success;
    let machine_id_reset_error = if reset_result.success {
        None
    } else {
        reset_result.error
    };

    if trimmed.starts_with("auth1_") {
        let mut result = switch_account_via_auth1(
            trimmed,
            devin_account_id,
            devin_primary_org_id,
            &target,
        )
        .await?;
        result.machine_id_reset = Some(machine_id_reset);
        result.machine_id_reset_error = machine_id_reset_error;
        Ok(result)
    } else {
        let mut result = switch_account_via_session(
            trimmed,
            devin_account_id,
            devin_primary_org_id,
            &target,
        )
        .await?;
        result.machine_id_reset = Some(machine_id_reset);
        result.machine_id_reset_error = machine_id_reset_error;
        Ok(result)
    }
}

async fn switch_account_via_session(
    session_token: &str,
    devin_account_id_hint: Option<String>,
    devin_primary_org_id_hint: Option<String>,
    target: &WindsurfTarget,
) -> Result<WindsurfSwitchAccountResult, String> {
    let auth = WindsurfAuth {
        token: session_token.to_string(),
        devin_account_id: devin_account_id_hint.clone(),
        devin_primary_org_id: devin_primary_org_id_hint.clone(),
        ..Default::default()
    };

    let one_time_token = match windsurf_get_one_time_auth_token(&auth).await {
        Ok(t) => t,
        Err(e) => {
            return Ok(WindsurfSwitchAccountResult {
                success: false,
                provider: Some("auth1".to_string()),
                error: Some(format!("获取一次性 auth_token 失败: {}", e)),
                auth_token: Some(session_token.to_string()),
                session_token: Some(session_token.to_string()),
                devin_account_id: devin_account_id_hint,
                devin_primary_org_id: devin_primary_org_id_hint,
                ..Default::default()
            });
        }
    };

    let callback_url = match open_windsurf_callback_url(&one_time_token, target) {
        Ok(url) => url,
        Err(e) => {
            return Ok(WindsurfSwitchAccountResult {
                success: false,
                provider: Some("auth1".to_string()),
                error: Some(e),
                one_time_auth_token: Some(one_time_token),
                auth_token: Some(session_token.to_string()),
                session_token: Some(session_token.to_string()),
                devin_account_id: devin_account_id_hint,
                devin_primary_org_id: devin_primary_org_id_hint,
                ..Default::default()
            });
        }
    };

    Ok(WindsurfSwitchAccountResult {
        success: true,
        provider: Some("auth1".to_string()),
        one_time_auth_token: Some(one_time_token),
        callback_url: Some(callback_url),
        auth_token: Some(session_token.to_string()),
        session_token: Some(session_token.to_string()),
        devin_account_id: devin_account_id_hint,
        devin_primary_org_id: devin_primary_org_id_hint,
        ..Default::default()
    })
}

/// Devin Auth1 Token 路径：auth1_token → WindsurfPostAuth → session_token → 5-header GetOneTimeAuthToken → windsurf://
async fn switch_account_via_auth1(
    auth1_token: &str,
    devin_account_id_hint: Option<String>,
    devin_primary_org_id_hint: Option<String>,
    target: &WindsurfTarget,
) -> Result<WindsurfSwitchAccountResult, String> {
    // Step 1: WindsurfPostAuth 换 session_token
    let primary_org_hint = devin_primary_org_id_hint
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .to_string();

    let post_auth = match windsurf_post_auth_internal(auth1_token, &primary_org_hint).await {
        Ok(v) => v,
        Err(e) => {
            return Ok(WindsurfSwitchAccountResult {
                success: false,
                provider: Some("auth1".to_string()),
                error: Some(format!("WindsurfPostAuth 失败: {}", e)),
                ..Default::default()
            });
        }
    };

    let effective_auth1 = post_auth
        .auth1_token
        .clone()
        .unwrap_or_else(|| auth1_token.to_string());
    let account_id = post_auth
        .account_id
        .clone()
        .or_else(|| devin_account_id_hint.clone());
    let primary_org_id = post_auth.primary_org_id.clone().or_else(|| {
        if !primary_org_hint.is_empty() {
            Some(primary_org_hint.clone())
        } else {
            post_auth.orgs.first().map(|o| o.id.clone())
        }
    });

    // Step 2: 组装 5-header WindsurfAuth，调 GetOneTimeAuthToken
    let auth = WindsurfAuth {
        token: post_auth.session_token.clone(),
        devin_auth1_token: Some(effective_auth1.clone()),
        devin_account_id: account_id.clone(),
        devin_primary_org_id: primary_org_id.clone(),
    };

    let one_time_token = match windsurf_get_one_time_auth_token(&auth).await {
        Ok(t) => t,
        Err(e) => {
            return Ok(WindsurfSwitchAccountResult {
                success: false,
                provider: Some("auth1".to_string()),
                error: Some(format!("获取一次性 auth_token 失败: {}", e)),
                auth_token: Some(effective_auth1),
                session_token: Some(post_auth.session_token),
                devin_account_id: account_id,
                devin_primary_org_id: primary_org_id,
                ..Default::default()
            });
        }
    };

    // Step 3: 打开 <scheme>:// URL
    let callback_url = match open_windsurf_callback_url(&one_time_token, target) {
        Ok(url) => url,
        Err(e) => {
            return Ok(WindsurfSwitchAccountResult {
                success: false,
                provider: Some("auth1".to_string()),
                error: Some(e),
                one_time_auth_token: Some(one_time_token),
                auth_token: Some(effective_auth1),
                session_token: Some(post_auth.session_token),
                devin_account_id: account_id,
                devin_primary_org_id: primary_org_id,
                ..Default::default()
            });
        }
    };

    Ok(WindsurfSwitchAccountResult {
        success: true,
        provider: Some("auth1".to_string()),
        one_time_auth_token: Some(one_time_token),
        callback_url: Some(callback_url),
        auth_token: Some(effective_auth1),
        session_token: Some(post_auth.session_token),
        devin_account_id: account_id,
        devin_primary_org_id: primary_org_id,
        ..Default::default()
    })
}

// ==================== Windsurf 安装路径探测 / 校验 Tauri 命令 ====================

/// 探测指定客户端类型的 Windsurf 安装路径。
///
/// 参数：
/// - `client_type`：`"windsurf"` / `"windsurf-next"`，省略视为 `"windsurf"`。
///
/// 返回：
/// - 命中 → `Some(<绝对路径>)`，路径满足 `<path>/.../extension.js` 存在；
/// - 未命中 → `None`，前端应引导用户手动指定。
#[tauri::command]
pub fn detect_windsurf_install_path(client_type: Option<String>) -> Option<String> {
    let ct = client_type
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("windsurf");
    detect_windsurf_install_path_impl(ct)
}

/// 校验用户输入的 Windsurf 安装路径是否有效（是否包含 `extension.js`）。
#[tauri::command]
pub fn validate_windsurf_install_path(path: String) -> bool {
    let path = path.trim();
    if path.is_empty() {
        return false;
    }
    is_valid_windsurf_install_dir(&PathBuf::from(path))
}
