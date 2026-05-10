<template>
  <div class="account-card" :class="statusClass">
    <div class="card-header">
      <div class="card-title">
        <div class="email-row">
          <span class="account-icon" aria-hidden="true">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
              <path d="M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z"/>
            </svg>
          </span>
          <span class="email-text" :title="account.email">{{ maskedEmail }}</span>
          <button class="copy-btn" type="button" @click.stop="copyEmail" :title="$t('windsurf.copyEmail')">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
              <path d="M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z"/>
            </svg>
          </button>
        </div>
        <div class="name-row">
          <span v-if="fullName" class="name-text">{{ fullName }}</span>
        </div>
      </div>
      <div class="status-badges">
        <span v-if="isCurrentLogin" class="status-chip current-login">● {{ $t('windsurf.currentLogin') }}</span>
        <span class="status-chip" :class="statusChipClass">{{ statusText }}</span>
      </div>
    </div>

    <div class="card-body">
      <div class="meta-grid">
        <div class="meta-item">
          <span class="meta-label">{{ $t('windsurf.expiresAt') }}</span>
          <span class="meta-value">
            {{ expireInfo.date }}
            <span
              v-if="expireInfo.remaining"
              class="expire-remaining"
              :style="{ color: expireInfo.color }"
            >
              · {{ expireInfo.remaining }}
            </span>
          </span>
        </div>
        <div class="meta-item">
          <span class="meta-label">{{ $t('windsurf.token') }}</span>
          <span class="meta-value">{{ tokenStatusText }}</span>
        </div>
      </div>

      <div v-if="hasQuotaSection" class="quota-section">
        <div class="quota-head">
          <div class="quota-label">{{ $t('windsurf.credits') }}</div>
          <span v-if="planNameText" class="plan-name">{{ planNameText }}</span>
        </div>
        <div v-if="hasDailyQuota || hasWeeklyQuota" class="sub-quota-grid">
          <div v-if="hasDailyQuota" class="sub-quota-item">
            <div class="sub-quota-head">
              <span class="sub-quota-title">{{ $t('windsurf.dailyQuota') }}</span>
              <span class="sub-quota-percent" :style="{ color: quotaColorFor(dailyQuotaPercent) }">
                {{ dailyQuotaPercent }}%
              </span>
            </div>
            <div class="sub-quota-bar-bg">
              <div class="sub-quota-bar-fill" :style="{ width: dailyQuotaPercent + '%', backgroundColor: quotaColorFor(dailyQuotaPercent) }"></div>
            </div>
            <div v-if="dailyResetText" class="sub-quota-reset">
              {{ $t('windsurf.resetAt') }}: {{ dailyResetText }}
            </div>
          </div>

          <div v-if="hasWeeklyQuota" class="sub-quota-item">
            <div class="sub-quota-head">
              <span class="sub-quota-title">{{ $t('windsurf.weeklyQuota') }}</span>
              <span class="sub-quota-percent" :style="{ color: quotaColorFor(weeklyQuotaPercent) }">
                {{ weeklyQuotaPercent }}%
              </span>
            </div>
            <div class="sub-quota-bar-bg">
              <div class="sub-quota-bar-fill" :style="{ width: weeklyQuotaPercent + '%', backgroundColor: quotaColorFor(weeklyQuotaPercent) }"></div>
            </div>
            <div v-if="weeklyResetText" class="sub-quota-reset">
              {{ $t('windsurf.resetAt') }}: {{ weeklyResetText }}
            </div>
          </div>
        </div>

        <div v-if="creditsUpdatedAtText" class="credits-updated">
          {{ $t('windsurf.lastUpdated') }}: {{ creditsUpdatedAtText }}
        </div>
      </div>

      <div class="actions">
        <button class="btn small primary" type="button" :disabled="account.__updating" @click.stop="emit('refresh-account', account)">
          <span v-if="account.__updating" class="sync-spinner"></span>
          {{ account.__updating ? $t('windsurf.updating') : $t('windsurf.update') }}
        </button>
        <button
          class="btn small success switch-btn"
          :class="{ loading: account.__loggingIn }"
          type="button"
          :disabled="account.__updating || account.__loggingIn || !account.auth_token"
          @click.stop="emit('switch-account', account)"
        >
          <span v-if="account.__loggingIn" class="sync-spinner"></span>
          <span class="btn-label">{{ account.__loggingIn ? $t('windsurf.loggingIn') : $t('windsurf.switchAccount') }}</span>
        </button>
        <button
          class="btn small info"
          type="button"
          :disabled="account.__updating || account.__loggingIn || !account.auth_token"
          :title="!account.auth_token ? $t('windsurf.noToken') : ''"
          @click.stop="emit('show-analytics', account)"
        >
          {{ $t('windsurf.analyticsAction') }}
        </button>
        <button class="btn small secondary" type="button" :disabled="account.__updating || account.__loggingIn" @click.stop="emit('delete-local', account)">
          {{ $t('windsurf.deleteLocal') }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const props = defineProps({
  account: {
    type: Object,
    required: true
  },
  isCurrentLogin: {
    type: Boolean,
    default: false
  },
  highlightCurrentLogin: {
    type: Boolean,
    default: false
  }
})

const emit = defineEmits(['refresh-account', 'switch-account', 'delete-local', 'show-toast', 'show-analytics'])

const maskedEmail = computed(() => {
  const email = props.account.email || ''
  const [name, domain] = email.split('@')
  if (!name || !domain) return email
  if (name.length <= 2) return `${name}***@${domain}`
  return `${name.slice(0, 2)}***@${domain}`
})

const fullName = computed(() => {
  const first = props.account.first_name || ''
  const last = props.account.last_name || ''
  const name = `${first} ${last}`.trim()
  return name || ''
})

const expireInfo = computed(() => {
  const raw = props.account.expires_at
  if (!raw) return { date: '-', remaining: '', color: '' }
  const d = new Date(raw)
  if (Number.isNaN(d.getTime())) return { date: String(raw), remaining: '', color: '' }

  const dateStr = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
  const startOfDay = (dt) => new Date(dt.getFullYear(), dt.getMonth(), dt.getDate()).getTime()
  const diffDays = Math.round((startOfDay(d) - startOfDay(new Date())) / 86400000)

  let remaining
  let color
  if (diffDays < 0) {
    remaining = t('windsurf.alreadyExpired')
    color = '#dc2626'
  } else if (diffDays === 0) {
    remaining = t('windsurf.expireToday')
    color = '#dc2626'
  } else if (diffDays <= 3) {
    remaining = t('windsurf.expireInDays', { days: diffDays })
    color = '#dc2626'
  } else if (diffDays <= 10) {
    remaining = t('windsurf.expireInDays', { days: diffDays })
    color = '#f59e0b'
  } else {
    remaining = t('windsurf.expireInDays', { days: diffDays })
    color = '#10b981'
  }

  return { date: dateStr, remaining, color }
})

const statusText = computed(() => {
  const s = (props.account.status || '').toLowerCase()
  if (s === 'active') return t('windsurf.statusActive')
  if (s === 'inactive') return t('windsurf.statusInactive')
  if (s === 'banned') return t('windsurf.statusBanned')
  return props.account.status || t('windsurf.statusUnknown')
})

const statusChipClass = computed(() => {
  const s = (props.account.status || '').toLowerCase()
  if (s === 'active') return 'ok'
  if (s === 'inactive') return 'warn'
  if (s === 'banned') return 'bad'
  return 'neutral'
})

const isFreeAccount = computed(() => {
  const text = [
    props.account.plan_name,
    props.account.teams_tier_name,
    props.account.plan_type,
    props.account.tier
  ].filter(Boolean).join(' ').toLowerCase()
  return /\bfree\b|免费/.test(text)
})

const statusClass = computed(() => {
  const s = (props.account.status || '').toLowerCase()
  const classes = []
  if (s === 'active') classes.push('status-ok')
  else if (s === 'inactive') classes.push('status-warn')
  else if (s === 'banned') classes.push('status-bad')
  else classes.push('status-neutral')
  if (isFreeAccount.value) classes.push('free-account')
  if (props.isCurrentLogin) classes.push('current-login-account')
  if (props.highlightCurrentLogin) classes.push('current-login-highlight')
  return classes
})

const tokenStatusText = computed(() => {
  return props.account.auth_token ? t('windsurf.tokenPresent') : t('windsurf.tokenMissing')
})

const planNameText = computed(() => {
  return props.account.plan_name || ''
})

const creditsUpdatedAtText = computed(() => {
  if (!props.account.credits_updated_at) return ''
  const d = new Date(props.account.credits_updated_at)
  if (Number.isNaN(d.getTime())) return String(props.account.credits_updated_at)
  return d.toLocaleString()
})

const quotaPercentage = computed(() => {
  const total = Number(props.account.total_credits)
  const used = Number(props.account.used_credits)
  if (!Number.isFinite(total) || total <= 0) return 0
  if (!Number.isFinite(used) || used < 0) return 0
  const remaining = Math.max(0, total - used)
  const pct = Math.min(100, Math.max(0, (remaining / total) * 100))
  return Math.round(pct)
})

const quotaColorFor = (percent) => {
  const pct = Number(percent)
  if (!Number.isFinite(pct)) return '#9ca3af'
  if (pct <= 10) return '#ef4444'
  if (pct <= 30) return '#f59e0b'
  if (pct <= 60) return '#3b82f6'
  return '#10b981'
}

const normalizePercent = (value) => {
  const pct = Number(value)
  if (!Number.isFinite(pct)) return null
  return Math.max(0, Math.min(100, Math.round(pct)))
}

const dailyQuotaPercent = computed(() => normalizePercent(props.account.daily_quota_remaining_percent))
const weeklyQuotaPercent = computed(() => normalizePercent(props.account.weekly_quota_remaining_percent))

const hasDailyQuota = computed(() => {
  return dailyQuotaPercent.value !== null
})

const hasWeeklyQuota = computed(() => {
  return weeklyQuotaPercent.value !== null
})

const hasQuotaSection = computed(() => {
  return Boolean(planNameText.value || hasDailyQuota.value || hasWeeklyQuota.value || creditsUpdatedAtText.value)
})

const formatUnixSec = (sec) => {
  if (!sec || !Number.isFinite(Number(sec))) return ''
  const d = new Date(Number(sec) * 1000)
  if (Number.isNaN(d.getTime())) return ''
  return d.toLocaleString()
}

const dailyResetText = computed(() => formatUnixSec(props.account.daily_quota_reset_at_unix))
const weeklyResetText = computed(() => formatUnixSec(props.account.weekly_quota_reset_at_unix))

const copyEmail = async () => {
  try {
    await navigator.clipboard.writeText(props.account.email || '')
    emit('show-toast', t('windsurf.emailCopied'), 'success')
  } catch {
    emit('show-toast', t('windsurf.copyFailed'), 'error')
  }
}
</script>

<style scoped>
.account-card {
  background: var(--card-bg, #fff);
  border: 1px solid rgba(0, 0, 0, 0.06);
  border-radius: 12px;
  padding: 14px;
  transition: transform 0.15s ease, box-shadow 0.15s ease, border-color 0.15s ease;
}

.account-card:hover {
  transform: translateY(-1px);
  box-shadow: 0 10px 24px rgba(0, 0, 0, 0.08);
}

.account-card.free-account {
  border-style: dashed;
  border-color: rgba(245, 158, 11, 0.75);
  background:
    linear-gradient(135deg, rgba(245, 158, 11, 0.08), rgba(255, 255, 255, 0.92));
}

.account-card.current-login-account {
  border-color: #10b981;
  border-style: solid;
  box-shadow: 0 0 0 1px #10b981 inset;
  background:
    linear-gradient(
      180deg,
      rgba(16, 185, 129, 0.08) 0%,
      var(--card-bg, #fff) 60%
    );
}

.account-card.current-login-highlight {
  animation: currentLoginPulse 1.2s ease-in-out 4;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 10px;
  margin-bottom: 10px;
  flex-wrap: nowrap;
}

.card-title {
  flex: 1 1 auto;
  min-width: 0;
}

.email-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.account-icon {
  width: 22px;
  height: 22px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.05);
  color: rgba(0, 0, 0, 0.55);
  flex: 0 0 auto;
}

.email-text {
  font-weight: 700;
  font-size: 16px;
  color: var(--text-primary, #111827);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1 1 auto;
  min-width: 0;
}

.copy-btn {
  border: none;
  background: rgba(0, 0, 0, 0.05);
  color: rgba(0, 0, 0, 0.55);
  border-radius: 8px;
  width: 28px;
  height: 28px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
}

.copy-btn:hover {
  background: rgba(0, 0, 0, 0.08);
  color: rgba(0, 0, 0, 0.75);
}

.name-row {
  margin-top: 4px;
  color: rgba(0, 0, 0, 0.55);
  font-size: 12px;
}

.status-badges {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: nowrap;
  flex-shrink: 0;
  justify-content: flex-end;
}

.meta-chip {
  padding: 4px 10px;
  border-radius: 999px;
  font-size: 12px;
  font-weight: 600;
  user-select: none;
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.meta-chip.cloud-sync {
  background: rgba(59, 130, 246, 0.14);
  color: #2563eb;
}

.meta-chip.user {
  background: rgba(16, 185, 129, 0.12);
  color: #059669;
}

.meta-chip.sold {
  background: rgba(245, 158, 11, 0.14);
  color: #b45309;
}

.status-chip {
  padding: 4px 10px;
  border-radius: 999px;
  font-size: 12px;
  white-space: nowrap;
  font-weight: 600;
  user-select: none;
}

.expire-remaining {
  font-weight: 600;
  white-space: nowrap;
}

.status-chip.ok {
  background: rgba(16, 185, 129, 0.12);
  color: #059669;
}

.status-chip.warn {
  background: rgba(245, 158, 11, 0.14);
  color: #b45309;
}

.status-chip.bad {
  background: rgba(239, 68, 68, 0.14);
  color: #b91c1c;
}

.status-chip.neutral {
  background: rgba(107, 114, 128, 0.14);
  color: #374151;
}

.status-chip.current-login {
  background: #10b981;
  color: #fff;
  letter-spacing: 0.2px;
}

.card-body {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.meta-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
  gap: 10px;
}

.meta-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.meta-label {
  font-size: 12px;
  color: rgba(0, 0, 0, 0.45);
}

.meta-value {
  font-size: 12px;
  color: rgba(0, 0, 0, 0.8);
}

.quota-section {
  padding-top: 8px;
  border-top: 1px solid rgba(0, 0, 0, 0.06);
}

.quota-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 8px;
}

.quota-label {
  font-size: 12px;
  color: rgba(0, 0, 0, 0.5);
}

.plan-name {
  padding: 2px 8px;
  border-radius: 999px;
  background: rgba(59, 130, 246, 0.12);
  color: #2563eb;
  font-weight: 600;
}

.sub-quota-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
  gap: 8px;
  margin-top: 8px;
}

.sub-quota-item {
  padding: 8px 10px;
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.03);
  border: 1px solid rgba(0, 0, 0, 0.05);
}

.sub-quota-head {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  margin-bottom: 4px;
}

.sub-quota-title {
  font-size: 11px;
  color: rgba(0, 0, 0, 0.6);
  font-weight: 600;
}

.sub-quota-percent {
  font-size: 13px;
  font-weight: 700;
}

.sub-quota-bar-bg {
  height: 4px;
  background: rgba(0, 0, 0, 0.08);
  border-radius: 2px;
  overflow: hidden;
}

.sub-quota-bar-fill {
  height: 100%;
  transition: width 0.3s;
}

.sub-quota-reset {
  margin-top: 4px;
  font-size: 10px;
  color: rgba(0, 0, 0, 0.45);
}

.credits-updated {
  margin-top: 6px;
  font-size: 12px;
  color: rgba(0, 0, 0, 0.55);
}

.actions {
  display: flex;
  flex-wrap: nowrap;
  gap: 6px;
  margin-top: 10px;
}

.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 8px 16px;
  border-radius: 6px;
  border: none;
  cursor: pointer;
  font-size: 14px;
  font-weight: 500;
  transition: all 0.2s;
  margin: 0;
}

.actions .btn {
  flex: 1 1 0;
  min-width: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.btn.small {
  padding: 5px 8px;
  font-size: 12px;
  line-height: 1.25;
}

.btn.primary {
  background: var(--primary-color, #3b82f6);
  color: white;
}

.btn.primary:hover {
  background: var(--primary-hover, #2563eb);
}

.btn.success {
  background: var(--success-color, #10b981);
  color: white;
}

.btn.success:hover {
  background: var(--success-hover, #059669);
}

.switch-btn {
  position: relative;
  transition: background 0.2s ease, transform 0.2s ease, opacity 0.2s ease, box-shadow 0.2s ease;
}

.switch-btn:not(:disabled):hover {
  transform: translateY(-1px);
  box-shadow: 0 6px 14px rgba(16, 185, 129, 0.24);
}

.switch-btn.loading {
  background: linear-gradient(90deg, #10b981, #059669, #10b981);
  background-size: 200% 100%;
  animation: switchLoading 1.2s ease-in-out infinite;
}

.btn-label {
  transition: opacity 0.18s ease, transform 0.18s ease;
}

.btn.info {
  background: var(--info-color, #06b6d4);
  color: white;
}

.btn.info:hover {
  background: var(--info-hover, #0891b2);
}

.btn.secondary {
  background: rgba(0, 0, 0, 0.08);
  color: rgba(0, 0, 0, 0.72);
}

.btn.secondary:hover {
  background: rgba(0, 0, 0, 0.14);
}

.btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
  transform: none;
}

.sync-spinner {
  width: 12px;
  height: 12px;
  border: 2px solid rgba(255, 255, 255, 0.5);
  border-top-color: rgba(255, 255, 255, 1);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

@keyframes switchLoading {
  0% {
    background-position: 0% 50%;
  }
  100% {
    background-position: 200% 50%;
  }
}

.dialog-actions {
  display: flex;
  gap: 12px;
}

@keyframes currentLoginPulse {
  0%, 100% {
    transform: translateY(0);
    box-shadow: 0 0 0 1px #10b981 inset;
  }
  50% {
    transform: translateY(-2px);
    box-shadow: 0 0 0 1px #10b981 inset, 0 0 0 3px rgba(16, 185, 129, 0.22), 0 16px 32px rgba(16, 185, 129, 0.16);
  }
}

/* ============================== */
/*  暗色主题统一覆盖               */

[data-theme='dark'] .account-card {
  background: var(--color-surface, #1e293b);
  border-color: rgba(148, 163, 184, 0.2);
  color: var(--color-text-primary, #e2e8f0);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.35);
}

[data-theme='dark'] .account-card:hover {
  box-shadow: 0 10px 24px rgba(0, 0, 0, 0.45);
}

[data-theme='dark'] .account-card.free-account {
  border-color: rgba(251, 191, 36, 0.75);
  background:
    linear-gradient(135deg, rgba(251, 191, 36, 0.12), rgba(30, 41, 59, 0.94));
}

[data-theme='dark'] .account-card.current-login-account {
  border-color: #10b981;
  box-shadow: 0 0 0 1px #10b981 inset;
  background:
    linear-gradient(
      180deg,
      rgba(16, 185, 129, 0.12) 0%,
      var(--color-surface, #1e293b) 60%
    );
}

[data-theme='dark'] .email-text {
  color: var(--color-text-strong, #f8fafc);
}

[data-theme='dark'] .copy-btn {
  background: rgba(148, 163, 184, 0.14);
  color: var(--color-text-secondary, #cbd5f5);
}

[data-theme='dark'] .copy-btn:hover {
  background: rgba(148, 163, 184, 0.22);
  color: var(--color-text-strong, #f8fafc);
}

[data-theme='dark'] .name-row {
  color: var(--color-text-muted, #94a3b8);
}

[data-theme='dark'] .meta-label {
  color: var(--color-text-muted, #94a3b8);
}

[data-theme='dark'] .meta-value {
  color: var(--color-text-primary, #e2e8f0);
}

[data-theme='dark'] .quota-section {
  border-top-color: rgba(148, 163, 184, 0.18);
}

[data-theme='dark'] .quota-label {
  color: var(--color-text-muted, #94a3b8);
}

[data-theme='dark'] .plan-name {
  background: rgba(96, 165, 250, 0.18);
  color: #93c5fd;
}

[data-theme='dark'] .sub-quota-item {
  background: rgba(148, 163, 184, 0.08);
  border-color: rgba(148, 163, 184, 0.18);
}

[data-theme='dark'] .sub-quota-title {
  color: var(--color-text-secondary, #cbd5f5);
}

[data-theme='dark'] .sub-quota-bar-bg {
  background: rgba(148, 163, 184, 0.2);
}

[data-theme='dark'] .sub-quota-reset {
  color: var(--color-text-muted, #94a3b8);
}

[data-theme='dark'] .credits-updated {
  color: var(--color-text-muted, #94a3b8);
}

[data-theme='dark'] .btn.secondary {
  background: rgba(148, 163, 184, 0.16);
  color: var(--color-text-primary, #e2e8f0);
}

[data-theme='dark'] .btn.secondary:hover {
  background: rgba(148, 163, 184, 0.26);
}

[data-theme='dark'] .meta-chip.cloud-sync {
  background: rgba(96, 165, 250, 0.2);
  color: #93c5fd;
}

[data-theme='dark'] .meta-chip.user {
  background: rgba(45, 212, 191, 0.18);
  color: #5eead4;
}

[data-theme='dark'] .meta-chip.sold {
  background: rgba(250, 204, 21, 0.18);
  color: #fde68a;
}

[data-theme='dark'] .status-chip.ok {
  background: rgba(45, 212, 191, 0.18);
  color: #5eead4;
}

[data-theme='dark'] .status-chip.warn {
  background: rgba(250, 204, 21, 0.18);
  color: #fde68a;
}

[data-theme='dark'] .status-chip.bad {
  background: rgba(248, 113, 113, 0.2);
  color: #fca5a5;
}

[data-theme='dark'] .status-chip.neutral {
  background: rgba(148, 163, 184, 0.2);
  color: var(--color-text-primary, #e2e8f0);
}

[data-theme='dark'] .status-chip.current-login {
  background: #10b981;
  color: #fff;
}
</style>
