import { ref } from 'vue'

const STORAGE_KEY = 'windsurf_auto_switch_config_v1'
const LAST_SWITCH_EMAIL_KEY = 'windsurf_last_switch_email_v1'
const CONFIG_VERSION = 3

export const DEFAULT_AUTO_SWITCH_CONFIG = {
  configVersion: CONFIG_VERSION,
  seamlessEnabled: true,
  seamlessAutoApply: true,
  enabled: false,
  intervalSeconds: 60,
  idleIntervalSeconds: 300,
  idleAfterUnchangedChecks: 3,
  dailyThresholdPercent: 15,
  weeklyThresholdPercent: 15,
  cooldownSeconds: 45,
  allExhaustedCooldownSeconds: 600,
  advancedVisible: false,
}

const numberFields = new Set([
  'intervalSeconds',
  'idleIntervalSeconds',
  'idleAfterUnchangedChecks',
  'dailyThresholdPercent',
  'weeklyThresholdPercent',
  'cooldownSeconds',
  'allExhaustedCooldownSeconds',
])

const limits = {
  intervalSeconds: [15, 86400],
  idleIntervalSeconds: [30, 86400],
  idleAfterUnchangedChecks: [1, 100],
  dailyThresholdPercent: [0, 100],
  weeklyThresholdPercent: [0, 100],
  cooldownSeconds: [5, 86400],
  allExhaustedCooldownSeconds: [60, 86400],
}

const state = ref({ ...DEFAULT_AUTO_SWITCH_CONFIG })
let hydrated = false

const clamp = (value, min, max) => {
  const n = Number(value)
  if (!Number.isFinite(n)) return min
  return Math.max(min, Math.min(max, Math.round(n)))
}

export function normalizeAutoSwitchConfig(input = {}) {
  const source = input || {}
  const next = { ...DEFAULT_AUTO_SWITCH_CONFIG, ...source }
  if (!source.configVersion && Number(source.intervalSeconds) === 90) {
    next.intervalSeconds = DEFAULT_AUTO_SWITCH_CONFIG.intervalSeconds
  }
  next.configVersion = CONFIG_VERSION
  next.seamlessEnabled = next.seamlessEnabled !== false
  next.seamlessAutoApply = next.seamlessAutoApply !== false
  next.enabled = next.enabled === true
  if (next.enabled) {
    next.seamlessEnabled = true
    next.seamlessAutoApply = true
  }
  next.advancedVisible = next.advancedVisible === true
  for (const key of numberFields) {
    const [min, max] = limits[key]
    next[key] = clamp(next[key], min, max)
  }
  if (next.idleIntervalSeconds < next.intervalSeconds) {
    next.idleIntervalSeconds = next.intervalSeconds
  }
  return next
}

export function loadAutoSwitchConfig() {
  if (!hydrated) {
    hydrated = true
    try {
      const raw = localStorage.getItem(STORAGE_KEY)
      state.value = normalizeAutoSwitchConfig(raw ? JSON.parse(raw) : {})
    } catch {
      state.value = { ...DEFAULT_AUTO_SWITCH_CONFIG }
    }
  }
  return { ...state.value }
}

export function saveAutoSwitchConfig(next) {
  const normalized = normalizeAutoSwitchConfig(next)
  state.value = normalized
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(normalized))
  } catch {
  }
  return { ...normalized }
}

export function useAutoSwitchConfig() {
  loadAutoSwitchConfig()
  return state
}

export function loadLastSwitchEmail() {
  try {
    return (localStorage.getItem(LAST_SWITCH_EMAIL_KEY) || '').trim().toLowerCase()
  } catch {
    return ''
  }
}

export function saveLastSwitchEmail(email) {
  const normalized = (email || '').trim().toLowerCase()
  try {
    if (normalized) localStorage.setItem(LAST_SWITCH_EMAIL_KEY, normalized)
    else localStorage.removeItem(LAST_SWITCH_EMAIL_KEY)
  } catch {
  }
}
