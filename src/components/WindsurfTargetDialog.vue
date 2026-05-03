<template>
  <Teleport to="body">
    <Transition name="modal" appear>
      <div
        v-if="visible"
        class="import-overlay"
        @click="handleOverlayClick"
      >
        <div class="import-dialog windsurf-target-dialog" @click.stop>
          <div class="dialog-header">
            <h3>{{ $t('windsurfTarget.title') }}</h3>
            <button @click="handleClose" class="dialog-close">×</button>
          </div>

          <div class="dialog-body">
            <p class="dialog-message">{{ $t('windsurfTarget.intro') }}</p>

            <div class="target-field">
              <label class="target-label" for="wt-client-type">
                {{ $t('windsurfTarget.clientTypeLabel') }}
              </label>
              <select
                id="wt-client-type"
                v-model="form.clientType"
                class="target-select"
              >
                <option value="windsurf">{{ $t('windsurfTarget.clientStandard') }}</option>
                <option value="windsurf-next">{{ $t('windsurfTarget.clientNext') }}</option>
              </select>
              <p class="target-hint">{{ $t('windsurfTarget.clientTypeHint') }}</p>
            </div>

            <div class="target-field">
              <label class="target-label" for="wt-install-path">
                {{ $t('windsurfTarget.installPathLabel') }}
              </label>
              <div class="target-input-row">
                <input
                  id="wt-install-path"
                  type="text"
                  v-model="form.installPath"
                  class="target-input"
                  :placeholder="$t('windsurfTarget.installPathPlaceholder')"
                  @input="installStatus = 'idle'"
                />
                <button
                  type="button"
                  class="target-btn"
                  :disabled="detecting"
                  @click="handleDetect"
                >
                  <span v-if="detecting" class="sync-spinner"></span>
                  {{ detecting ? $t('windsurfTarget.detecting') : $t('windsurfTarget.detect') }}
                </button>
                <button
                  type="button"
                  class="target-btn"
                  :disabled="validating || !installPathTrimmed"
                  @click="handleValidate"
                >
                  <span v-if="validating" class="sync-spinner"></span>
                  {{ validating ? $t('windsurfTarget.validating') : $t('windsurfTarget.validate') }}
                </button>
              </div>
              <p v-if="installStatusMessage" class="target-status" :class="installStatusClass">
                {{ installStatusMessage }}
              </p>
              <p class="target-hint">{{ $t('windsurfTarget.installPathHint') }}</p>
            </div>

            <div class="target-field">
              <label class="target-label" for="wt-user-data-dir">
                {{ $t('windsurfTarget.userDataDirLabel') }}
              </label>
              <input
                id="wt-user-data-dir"
                type="text"
                v-model="form.userDataDir"
                class="target-input"
                :placeholder="$t('windsurfTarget.userDataDirPlaceholder')"
              />
              <p class="target-hint">{{ $t('windsurfTarget.userDataDirHint') }}</p>
            </div>

            <p v-if="saveStatus" class="target-status" :class="{ success: saveStatus === 'saved' }">
              {{ saveStatus === 'saved' ? $t('windsurfTarget.saved') : '' }}
            </p>
          </div>

          <div class="dialog-footer">
            <button type="button" class="btn-cancel" @click="handleResetDefaults">
              {{ $t('windsurfTarget.resetDefaults') }}
            </button>
            <div class="dialog-footer-right">
              <button type="button" class="btn-cancel" @click="handleClose">
                {{ $t('common.cancel') }}
              </button>
              <button type="button" class="btn-confirm" @click="handleSave">
                {{ $t('windsurfTarget.save') }}
              </button>
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup>
import { reactive, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import {
  DEFAULT_TARGET,
  loadWindsurfTarget,
  saveWindsurfTarget,
} from '../composables/windsurfTarget'
import { useEscToClose } from '../composables/useEscToClose'

const { t } = useI18n()

const props = defineProps({
  visible: {
    type: Boolean,
    default: false,
  },
})

const emit = defineEmits(['update:visible', 'close'])

const form = reactive({ ...DEFAULT_TARGET })
const detecting = ref(false)
const validating = ref(false)
const installStatus = ref('idle') // 'idle' | 'detected' | 'not-found' | 'valid' | 'invalid'
const saveStatus = ref('')

const installPathTrimmed = ref('')
watch(
  () => form.installPath,
  (v) => {
    installPathTrimmed.value = (v || '').trim()
  },
  { immediate: true }
)

const installStatusMessage = ref('')
const installStatusClass = ref('')

const refreshStatusMessage = () => {
  switch (installStatus.value) {
    case 'detected':
      installStatusMessage.value = t('windsurfTarget.detected')
      installStatusClass.value = 'success'
      break
    case 'not-found':
      installStatusMessage.value = t('windsurfTarget.notFound')
      installStatusClass.value = 'error'
      break
    case 'valid':
      installStatusMessage.value = t('windsurfTarget.valid')
      installStatusClass.value = 'success'
      break
    case 'invalid':
      installStatusMessage.value = t('windsurfTarget.invalid')
      installStatusClass.value = 'error'
      break
    default:
      installStatusMessage.value = ''
      installStatusClass.value = ''
  }
}

watch(installStatus, refreshStatusMessage)

const hydrateForm = () => {
  const saved = loadWindsurfTarget()
  form.clientType = saved.clientType
  form.installPath = saved.installPath
  form.userDataDir = saved.userDataDir
  installStatus.value = 'idle'
  saveStatus.value = ''
  refreshStatusMessage()
}

watch(
  () => props.visible,
  (v) => {
    if (v) hydrateForm()
  },
  { immediate: true }
)

const handleClose = () => {
  emit('update:visible', false)
  emit('close')
}

const handleOverlayClick = () => {
  if (detecting.value || validating.value) return
  handleClose()
}

useEscToClose(
  () => props.visible,
  () => {
    if (!detecting.value && !validating.value) handleClose()
  }
)

const handleDetect = async () => {
  detecting.value = true
  try {
    const path = await invoke('detect_windsurf_install_path', {
      clientType: form.clientType,
      client_type: form.clientType,
    })
    if (path && typeof path === 'string' && path.trim()) {
      form.installPath = path
      installStatus.value = 'detected'
    } else {
      installStatus.value = 'not-found'
    }
  } catch (err) {
    console.warn('[windsurfTarget] detect failed', err)
    installStatus.value = 'not-found'
  } finally {
    detecting.value = false
  }
}

const handleValidate = async () => {
  if (!installPathTrimmed.value) return
  validating.value = true
  try {
    const ok = await invoke('validate_windsurf_install_path', {
      path: installPathTrimmed.value,
    })
    installStatus.value = ok ? 'valid' : 'invalid'
  } catch (err) {
    console.warn('[windsurfTarget] validate failed', err)
    installStatus.value = 'invalid'
  } finally {
    validating.value = false
  }
}

const handleResetDefaults = () => {
  form.clientType = DEFAULT_TARGET.clientType
  form.installPath = DEFAULT_TARGET.installPath
  form.userDataDir = DEFAULT_TARGET.userDataDir
  installStatus.value = 'idle'
  saveStatus.value = ''
}

const handleSave = () => {
  saveWindsurfTarget({
    clientType: form.clientType,
    installPath: form.installPath,
    userDataDir: form.userDataDir,
  })
  saveStatus.value = 'saved'
  // 给用户看一眼「已保存」，再关闭
  setTimeout(() => {
    if (saveStatus.value === 'saved') handleClose()
  }, 600)
}
</script>

<style scoped>
/* ========== Dialog 骨架（自包含，不依赖其它组件的 scoped 样式） ========== */

.import-overlay {
  position: fixed;
  inset: 0;
  z-index: 3500;
  background: rgba(0, 0, 0, 0.55);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
}

.import-dialog {
  width: min(720px, 96vw);
  background: var(--color-surface, #ffffff);
  color: var(--color-text-primary, #374151);
  border: 1px solid var(--color-border, #e5e7eb);
  border-radius: 14px;
  overflow: hidden;
  box-shadow: var(--color-shadow-modal, 0 20px 40px rgba(0, 0, 0, 0.22));
}

.dialog-header {
  padding: 14px 16px;
  border-bottom: 1px solid var(--color-border-muted, rgba(0, 0, 0, 0.06));
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.dialog-header h3 {
  margin: 0;
  font-size: 16px;
  color: var(--color-text-heading, #1f2937);
}

.dialog-close {
  border: none;
  background: transparent;
  color: var(--color-text-secondary, #4b5563);
  cursor: pointer;
  font-size: 22px;
  line-height: 1;
  opacity: 0.7;
  width: 32px;
  height: 32px;
  border-radius: 10px;
}

.dialog-close:hover {
  background: var(--color-surface-hover, rgba(0, 0, 0, 0.05));
  opacity: 1;
}

.dialog-body {
  padding: 14px 16px;
}

.dialog-message {
  margin: 0 0 14px 0;
  font-size: 13px;
  color: var(--color-text-secondary, #4b5563);
  line-height: 1.5;
}

.dialog-footer {
  padding: 14px 16px;
  border-top: 1px solid var(--color-border-muted, rgba(0, 0, 0, 0.06));
  display: flex;
  align-items: center;
  gap: 8px;
}

.dialog-footer-right {
  display: inline-flex;
  gap: 8px;
  margin-left: auto;
}

.btn-cancel {
  padding: 8px 16px;
  border-radius: 10px;
  border: 1px solid var(--color-border, #e5e7eb);
  background: transparent;
  color: var(--color-text-primary, #374151);
  cursor: pointer;
  font-size: 13px;
  transition: background 0.15s ease, border-color 0.15s ease;
}

.btn-cancel:hover:not(:disabled) {
  background: var(--color-surface-hover, rgba(0, 0, 0, 0.04));
  border-color: var(--color-border-strong, #d1d5db);
}

.btn-cancel:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.btn-confirm {
  padding: 8px 16px;
  border-radius: 10px;
  border: none;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  background: var(--color-btn-primary-bg, #3b82f6);
  color: var(--color-btn-primary-text, #ffffff);
  font-size: 13px;
  cursor: pointer;
  transition: background 0.15s ease;
}

.btn-confirm:hover:not(:disabled) {
  background: var(--color-btn-primary-bg-hover, #2563eb);
}

.btn-confirm:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.sync-spinner {
  width: 14px;
  height: 14px;
  border: 2px solid rgba(255, 255, 255, 0.4);
  border-top-color: #fff;
  border-radius: 50%;
  animation: wt-spin 0.8s linear infinite;
}

/* 按钮在浅色/暗色主题下 spinner 边框颜色需要跟随 */
.target-btn .sync-spinner {
  border-color: rgba(0, 0, 0, 0.12);
  border-top-color: var(--color-text-primary, #374151);
}

@keyframes wt-spin {
  to {
    transform: rotate(360deg);
  }
}

/* ========== modal 过渡（和其它全局 modal 保持一致） ========== */

.modal-enter-active,
.modal-leave-active {
  transition: opacity 0.18s ease;
}

.modal-enter-active .import-dialog,
.modal-leave-active .import-dialog {
  transition: transform 0.18s ease, opacity 0.18s ease;
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.modal-enter-from .import-dialog,
.modal-leave-to .import-dialog {
  transform: translateY(8px) scale(0.98);
  opacity: 0;
}

/* ========== Windsurf 目标设置表单 ========== */

.windsurf-target-dialog {
  max-width: 580px;
  width: min(580px, calc(100vw - 40px));
}

.target-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 14px;
}

.target-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--color-text-primary, #374151);
}

.target-select,
.target-input {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid var(--color-input-border, #d1d5db);
  border-radius: 8px;
  background: var(--color-input-bg, #ffffff);
  color: var(--color-text-primary, #374151);
  font-size: 13px;
  box-sizing: border-box;
  transition: border-color 0.15s ease, box-shadow 0.15s ease;
}

.target-input::placeholder {
  color: var(--color-input-placeholder, #9ca3af);
}

.target-select:focus,
.target-input:focus {
  border-color: var(--color-input-border-focus, #3b82f6);
  outline: none;
  box-shadow: var(--shadow-focus, 0 0 0 3px rgba(59, 130, 246, 0.15));
}

.target-input-row {
  display: flex;
  gap: 8px;
  align-items: stretch;
}

.target-input-row .target-input {
  flex: 1;
  min-width: 0;
}

.target-btn {
  flex-shrink: 0;
  padding: 0 14px;
  border: 1px solid var(--color-btn-secondary-border, #d1d5db);
  border-radius: 8px;
  background: var(--color-btn-secondary-bg, #f1f5f9);
  color: var(--color-btn-secondary-text, #374151);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  transition: background 0.15s ease, border-color 0.15s ease;
}

.target-btn:hover:not(:disabled) {
  background: var(--color-btn-secondary-bg-hover, #e2e8f0);
  border-color: var(--color-border-strong, #94a3b8);
}

.target-btn:disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.target-hint {
  font-size: 12px;
  color: var(--color-text-muted, #64748b);
  margin: 0;
  line-height: 1.5;
}

.target-status {
  font-size: 12px;
  margin: 2px 0 0;
  line-height: 1.5;
}

.target-status.success {
  color: var(--color-success-bg, #16a34a);
}

.target-status.error {
  color: var(--color-error-bg, #dc2626);
}

/* 暗色主题下状态文字的可读性微调（其余颜色全部走 --color-* 变量自动继承） */
[data-theme='dark'] .target-status.success {
  color: #4ade80;
}

[data-theme='dark'] .target-status.error {
  color: #f87171;
}
</style>
