<template>
  <div class="windsurf-shell">
    <aside class="app-sidebar" :class="{ collapsed: sidebarCollapsed }">
      <div class="sidebar-header">
        <span class="sidebar-logo">AVW</span>
        <button
          class="sidebar-collapse-btn"
          @click="sidebarCollapsed = !sidebarCollapsed"
          :title="sidebarCollapsed ? $t('app.appHome') : $t('app.windsurf')"
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline v-if="sidebarCollapsed" points="9 18 15 12 9 6" />
            <polyline v-else points="15 18 9 12 15 6" />
          </svg>
        </button>
      </div>

      <nav class="sidebar-nav">
        <button class="sidebar-item active" type="button">
          <span class="sidebar-icon">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
              <path d="M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z" />
            </svg>
          </span>
          <span v-if="!sidebarCollapsed" class="sidebar-label">{{ $t('app.windsurf') }}</span>
        </button>
      </nav>

      <div class="sidebar-footer"></div>
    </aside>

    <div class="app-main-area">
      <div v-if="updateInfo?.update_available" class="update-banner">
        <div class="update-copy">
          {{ $t('app.updateAvailable', { version: updateInfo.latest_version }) }}
        </div>
        <div class="update-actions">
          <button
            class="update-link"
            type="button"
            @click="openReleasePage"
          >
            {{ $t('app.updateNow') }}
          </button>
          <button type="button" class="update-dismiss" :aria-label="$t('app.dismissUpdate')" @click="dismissUpdate">
            ×
          </button>
        </div>
      </div>
      <main class="main-content">
        <WindsurfAccountList :embedded="true" />
      </main>

      <div class="fixed-controls">
        <button
          type="button"
          class="control-btn language-toggle"
          @click="toggleLanguage"
          :aria-label="languageToggleLabel"
          :title="languageToggleLabel"
        >
          {{ currentLocale === 'zh-CN' ? 'EN' : '中' }}
        </button>
        <button
          type="button"
          class="control-btn theme-toggle"
          @click="toggleTheme"
          :aria-pressed="isDarkTheme"
          :aria-label="themeToggleLabel"
          :title="themeToggleLabel"
        >
          <svg v-if="isDarkTheme" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
          </svg>
          <svg v-else width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="5" />
            <path d="m12 1 0 2" />
            <path d="m12 21 0 2" />
            <path d="m4.22 4.22 1.42 1.42" />
            <path d="m18.36 18.36 1.42 1.42" />
            <path d="m1 12 2 0" />
            <path d="m21 12 2 0" />
            <path d="m4.22 19.78 1.42-1.42" />
            <path d="m18.36 5.64 1.42-1.42" />
          </svg>
        </button>
      </div>

    </div>
  </div>
</template>

<script setup>
import { computed, inject, onBeforeUnmount, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import WindsurfAccountList from './components/WindsurfAccountList.vue'

const { t, locale } = useI18n()

const SIDEBAR_AUTO_COLLAPSE_BREAKPOINT = 960

const sidebarCollapsed = ref(false)
const currentLocale = ref(locale.value)
const updateInfo = ref(null)

const themeManager = inject('themeManager', null)
const themeStorageKey = themeManager?.storageKey ?? 'avw-theme'
const currentTheme = ref(themeManager?.initialTheme ?? document.documentElement.dataset.theme ?? 'light')
const isDarkTheme = computed(() => currentTheme.value === 'dark')

const changeLanguage = () => {
  locale.value = currentLocale.value
  localStorage.setItem('preferred-language', currentLocale.value)
}

const toggleLanguage = () => {
  currentLocale.value = currentLocale.value === 'zh-CN' ? 'en-US' : 'zh-CN'
  changeLanguage()
}

const languageToggleLabel = computed(() => {
  return currentLocale.value === 'zh-CN' ? t('app.switchToEnglish') : t('app.switchToChinese')
})

const applyThemeFallback = (theme) => {
  const normalized = theme === 'dark' ? 'dark' : 'light'
  const root = document.documentElement
  root.dataset.theme = normalized
  root.style.colorScheme = normalized
}

const setTheme = (theme) => {
  const normalized = theme === 'dark' ? 'dark' : 'light'
  currentTheme.value = normalized

  if (themeManager?.applyTheme) {
    themeManager.applyTheme(normalized)
  } else {
    applyThemeFallback(normalized)
  }

  localStorage.setItem(themeStorageKey, normalized)
}

const toggleTheme = () => {
  setTheme(isDarkTheme.value ? 'light' : 'dark')
}

const themeToggleLabel = computed(() => {
  return isDarkTheme.value ? t('app.switchToLight') : t('app.switchToDark')
})

const syncSidebarWithViewport = () => {
  if (typeof window === 'undefined') return
  sidebarCollapsed.value = window.innerWidth < SIDEBAR_AUTO_COLLAPSE_BREAKPOINT
}

const checkUpdates = async () => {
  try {
    const resp = await invoke('check_for_updates')
    if (resp?.success && resp.update_available) {
      updateInfo.value = resp
    }
  } catch {
    updateInfo.value = null
  }
}

const dismissUpdate = () => {
  updateInfo.value = null
}

const openReleasePage = async () => {
  try {
    await invoke('open_release_page')
  } catch {
  }
}

onMounted(() => {
  const savedLanguage = localStorage.getItem('preferred-language')
  if (savedLanguage === 'zh-CN' || savedLanguage === 'en-US') {
    currentLocale.value = savedLanguage
    locale.value = savedLanguage
  }

  localStorage.removeItem('user')
  localStorage.removeItem('token')
  syncSidebarWithViewport()
  checkUpdates()
  window.addEventListener('resize', syncSidebarWithViewport)
})

onBeforeUnmount(() => {
  window.removeEventListener('resize', syncSidebarWithViewport)
})
</script>

<style scoped>
.windsurf-shell {
  height: 100vh;
  background: var(--color-surface-muted, #f8f9fa);
  overflow: hidden;
  display: flex;
}

.app-sidebar {
  width: clamp(192px, 18vw, 224px);
  background: var(--color-surface, #ffffff);
  border-right: 1px solid var(--color-divider, #e1e5e9);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  transition: width 0.2s ease;
  /* 外层 windsurf-shell 已经 overflow:hidden 负责整体裁剪，
     sidebar 内保持 visible，collapsed 时账号浮窗菜单才不会被截断。 */
  overflow: visible;
  position: relative;
  z-index: 1;
}

.app-sidebar.collapsed {
  width: 56px;
}

.sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 12px;
  border-bottom: 1px solid var(--color-divider, #e1e5e9);
  gap: 8px;
}

.sidebar-logo {
  font-size: 18px;
  font-weight: 800;
  color: var(--color-accent, #3b82f6);
  user-select: none;
  white-space: nowrap;
}

.app-sidebar.collapsed .sidebar-logo {
  display: none;
}

.sidebar-collapse-btn {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  background: transparent;
  color: var(--color-text-muted, #9ca3af);
  border-radius: 6px;
  cursor: pointer;
  flex-shrink: 0;
  transition: all 0.15s;
}

.sidebar-collapse-btn:hover {
  background: var(--color-surface-hover, #f3f4f6);
  color: var(--color-text-primary, #374151);
}

.sidebar-nav {
  display: flex;
  flex-direction: column;
  padding: 8px;
  gap: 2px;
}

.sidebar-footer {
  margin-top: auto;
  padding: 12px 8px 14px;
  border-top: 1px solid var(--color-divider, #e1e5e9);
  position: relative;
}

.sidebar-account-wrap {
  position: relative;
}

.sidebar-account-trigger,
.sidebar-login-btn {
  width: 100%;
  min-height: 40px;
  display: flex;
  align-items: center;
  gap: 10px;
  border: none;
  border-radius: 12px;
  padding: 8px 10px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s ease;
}

.sidebar-account-trigger {
  background: rgba(59, 130, 246, 0.08);
  color: var(--color-text-primary, #374151);
}

.sidebar-account-trigger:hover,
.sidebar-account-trigger.open {
  background: rgba(59, 130, 246, 0.14);
}

.sidebar-account-trigger.collapsed,
.sidebar-login-btn.collapsed {
  justify-content: center;
  padding: 8px;
}

.sidebar-avatar {
  width: 28px;
  height: 28px;
  border-radius: 999px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: #3b82f6;
  color: #fff;
  font-size: 12px;
  font-weight: 700;
  flex-shrink: 0;
}

.sidebar-account-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  text-align: left;
}

.sidebar-account-chevron {
  flex-shrink: 0;
  color: var(--color-text-muted, #6b7280);
  transition: transform 0.15s ease;
}

.sidebar-account-chevron.open {
  transform: rotate(180deg);
}

.sidebar-account-menu {
  position: absolute;
  left: 0;
  right: 0;
  bottom: calc(100% + 8px);
  padding: 8px;
  border-radius: 12px;
  border: 1px solid var(--color-border, #e5e7eb);
  background: var(--color-surface, #ffffff);
  box-shadow: 0 12px 30px rgba(15, 23, 42, 0.18);
  z-index: 50;
}

.sidebar-account-wrap.collapsed .sidebar-account-menu {
  left: calc(100% + 8px);
  right: auto;
  bottom: 0;
  width: 180px;
}

.sidebar-account-menu-header {
  padding: 4px 6px 8px;
  font-size: 12px;
  font-weight: 600;
  color: var(--color-text-muted, #6b7280);
  word-break: break-all;
}

.sidebar-account-menu-item {
  width: 100%;
  border: none;
  border-radius: 10px;
  background: transparent;
  color: var(--color-text-primary, #374151);
  padding: 10px 12px;
  font-size: 13px;
  font-weight: 600;
  text-align: left;
  cursor: pointer;
  transition: background 0.15s ease;
}

.sidebar-account-menu-item:hover {
  background: rgba(239, 68, 68, 0.1);
  color: #dc2626;
}

.sidebar-login-btn {
  justify-content: center;
  background: #3b82f6;
  color: #fff;
}

.sidebar-login-btn:hover {
  background: #2563eb;
}

.sidebar-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 10px;
  border: none;
  background: transparent;
  border-radius: 8px;
  color: var(--color-text-secondary, #6b7280);
  font-size: 13px;
  font-weight: 500;
  text-align: left;
  white-space: nowrap;
}

.sidebar-item.active {
  background: rgba(59, 130, 246, 0.12);
  color: #3b82f6;
  font-weight: 600;
}

.sidebar-icon {
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.sidebar-label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.app-main-area {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.update-banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  padding: 10px 18px;
  border-bottom: 1px solid rgba(37, 99, 235, 0.18);
  background: linear-gradient(90deg, rgba(59, 130, 246, 0.12), rgba(16, 185, 129, 0.12));
  color: var(--color-text-primary, #1f2937);
  font-size: 13px;
  font-weight: 600;
}

.update-copy {
  min-width: 0;
}

.update-actions {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.update-link {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-height: 28px;
  padding: 0 12px;
  border: none;
  border-radius: 999px;
  background: #2563eb;
  color: #ffffff;
  cursor: pointer;
  font: inherit;
  text-decoration: none;
  transition: transform 0.18s ease, box-shadow 0.18s ease, background 0.18s ease;
}

.update-link:hover {
  background: #1d4ed8;
  box-shadow: 0 8px 18px rgba(37, 99, 235, 0.24);
  transform: translateY(-1px);
}

.update-dismiss {
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 999px;
  background: rgba(15, 23, 42, 0.08);
  color: var(--color-text-secondary, #475569);
  cursor: pointer;
  font-size: 18px;
  line-height: 1;
  transition: background 0.18s ease, color 0.18s ease;
}

.update-dismiss:hover {
  background: rgba(15, 23, 42, 0.14);
  color: var(--color-text-primary, #1f2937);
}

.main-content {
  display: flex;
  flex: 1;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}

.fixed-controls {
  position: fixed;
  right: 20px;
  bottom: 20px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  z-index: 1000;
}

.control-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  border-radius: 50%;
  border: 1px solid var(--color-border, #e5e7eb);
  background: var(--color-surface, #ffffff);
  color: var(--color-text-primary, #374151);
  cursor: pointer;
  transition: all 0.2s ease;
  box-shadow: 0 3px 10px rgba(0, 0, 0, 0.15);
}

.control-btn:hover {
  transform: translateY(-2px);
  box-shadow: 0 5px 15px rgba(0, 0, 0, 0.2);
}

.control-btn.language-toggle {
  font-weight: 700;
  font-size: 12px;
}

[data-theme='dark'] .app-sidebar {
  background: var(--color-surface, #1e293b);
  border-color: rgba(148, 163, 184, 0.15);
}

[data-theme='dark'] .sidebar-item.active {
  background: rgba(59, 130, 246, 0.2);
  color: #60a5fa;
}

[data-theme='dark'] .sidebar-footer {
  border-color: rgba(148, 163, 184, 0.15);
}

[data-theme='dark'] .sidebar-collapse-btn:hover {
  background: rgba(148, 163, 184, 0.1);
}

[data-theme='dark'] .sidebar-account-trigger {
  background: rgba(96, 165, 250, 0.16);
  color: #e2e8f0;
}

[data-theme='dark'] .sidebar-account-trigger:hover,
[data-theme='dark'] .sidebar-account-trigger.open {
  background: rgba(96, 165, 250, 0.22);
}

[data-theme='dark'] .sidebar-account-menu {
  background: var(--color-surface, #1e293b);
  border-color: rgba(148, 163, 184, 0.2);
}

[data-theme='dark'] .sidebar-account-menu-header {
  color: #94a3b8;
}

[data-theme='dark'] .sidebar-account-menu-item {
  color: #e2e8f0;
}

[data-theme='dark'] .sidebar-account-menu-item:hover {
  background: rgba(248, 113, 113, 0.14);
  color: #fca5a5;
}

[data-theme='dark'] .update-banner {
  border-color: rgba(96, 165, 250, 0.22);
  background: linear-gradient(90deg, rgba(37, 99, 235, 0.24), rgba(5, 150, 105, 0.2));
  color: #e2e8f0;
}

[data-theme='dark'] .update-dismiss {
  background: rgba(226, 232, 240, 0.1);
  color: #cbd5e1;
}

[data-theme='dark'] .update-dismiss:hover {
  background: rgba(226, 232, 240, 0.16);
  color: #f8fafc;
}

/* 窄屏交由 JS 自动 collapsed，不再在 CSS 里强制压缩以免 sidebar 宽度不伦不类 */
</style>
