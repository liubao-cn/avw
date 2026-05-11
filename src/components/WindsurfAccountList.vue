<template>
  <div :class="props.embedded ? 'embedded-container' : 'windsurf-account-list-modal'">
    <div :class="props.embedded ? 'embedded-inner' : 'modal-overlay'">
      <div :class="props.embedded ? 'embedded-panel' : 'modal-content'" @click.stop>
        <div class="modal-header">
          <div class="header-title">
            <h2>{{ $t('windsurf.title') }}</h2>
            <div class="status-badge info">
              <span class="status-dot info"></span>
              <span class="status-text">{{ accounts.length }} {{ $t('windsurf.accounts') }}</span>
            </div>
          </div>
          <div class="header-actions">
            <button @click="handleAddAuth1" class="btn primary small">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
                <path d="M12 1L3 5v6c0 5.55 3.84 10.74 9 12 5.16-1.26 9-6.45 9-12V5l-9-4zm-2 16l-4-4 1.41-1.41L10 14.17l6.59-6.59L18 9l-8 8z"/>
              </svg>
              {{ $t('windsurf.addAuth1') }}
            </button>
            <button @click="handleAddSession" class="btn success small">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
                <path d="M17 1H7C5.9 1 5 1.9 5 3v18l7-3 7 3V3c0-1.1-.9-2-2-2z"/>
              </svg>
              {{ $t('windsurf.addSession') }}
            </button>
            <button @click="handleRefreshAll" class="btn info small" :disabled="isRefreshing || accounts.length === 0">
              <span v-if="isRefreshing" class="sync-spinner"></span>
              {{ isRefreshing ? $t('windsurf.refreshing') : $t('windsurf.refreshAll') }}
            </button>
            <button @click="showAutoSwitchDialog = true" class="btn small" :class="autoSwitchConfig.enabled ? 'success' : 'secondary'">
              {{ $t('windsurf.autoSwitchTitle') }}
            </button>
            <button v-if="!props.embedded" class="close-btn" @click="handleClose">×</button>
          </div>
        </div>

        <div class="modal-body">
          <div class="auto-switch-banner" :class="{ enabled: autoSwitchConfig.enabled, busy: autoSwitchRuntime.busy }">
            <div class="auto-switch-main">
              <span class="auto-switch-dot"></span>
              <span>{{ autoSwitchRuntime.message || $t('windsurf.autoSwitchIdle') }}</span>
            </div>
            <button class="select-btn" type="button" :disabled="autoSwitchRuntime.busy" @click="requestAutoSwitchCheck">
              {{ $t('windsurf.autoSwitchCheckNow') }}
            </button>
          </div>

          <div v-if="isLoading" class="loading-state">
            <div class="spinner"></div>
            <p>{{ $t('windsurf.loading') }}</p>
          </div>

          <div v-else-if="accounts.length === 0" class="empty-state">
            <div class="empty-icon">
              <svg width="64" height="64" viewBox="0 0 24 24" fill="currentColor">
                <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 15l-5-5 1.41-1.41L10 14.17l7.59-7.59L19 8l-9 9z" />
              </svg>
            </div>
            <h3>{{ $t('windsurf.empty') }}</h3>
            <p>{{ $t('windsurf.emptyHint') }}</p>
            <div class="empty-actions">
              <button class="btn primary" @click="handleAddAuth1">{{ $t('windsurf.addAuth1') }}</button>
              <button class="btn success" @click="handleAddSession">{{ $t('windsurf.addSession') }}</button>
            </div>
          </div>

          <div v-else class="account-list">
            <div class="list-toolbar">
              <div class="search-box">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" class="search-icon">
                  <path d="M15.5 14h-.79l-.28-.27C15.41 12.59 16 11.11 16 9.5 16 5.91 13.09 3 9.5 3S3 5.91 3 9.5 5.91 16 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z"/>
                </svg>
                <input type="text" v-model="searchQuery" :placeholder="$t('windsurf.searchPlaceholder')" class="search-input" />
                <button v-if="searchQuery.trim()" @click="searchQuery = ''" class="clear-search-btn">
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
                    <path d="M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z"/>
                  </svg>
                </button>
              </div>

              <div class="sort-wrapper">
                <button class="btn sort-btn small" @click="showSortMenu = !showSortMenu">
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
                    <path d="M3 18h6v-2H3v2zM3 6v2h18V6H3zm0 7h12v-2H3v2z"/>
                  </svg>
                  {{ $t('windsurf.sort') }}
                </button>
                <Transition name="dropdown">
                  <div v-if="showSortMenu" class="sort-menu" @click.stop>
                    <button :class="['sort-option', { active: sortType === 'expiry' && sortOrder === 'desc' }]" @click="setSortType('expiry', 'desc')">
                      <span>{{ $t('windsurf.sortByExpiryDesc') }}</span>
                      <svg v-if="sortType === 'expiry' && sortOrder === 'desc'" width="16" height="16" viewBox="0 0 24 24" fill="currentColor" class="check-icon">
                        <path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z"/>
                      </svg>
                    </button>
                    <button :class="['sort-option', { active: sortType === 'expiry' && sortOrder === 'asc' }]" @click="setSortType('expiry', 'asc')">
                      <span>{{ $t('windsurf.sortByExpiryAsc') }}</span>
                      <svg v-if="sortType === 'expiry' && sortOrder === 'asc'" width="16" height="16" viewBox="0 0 24 24" fill="currentColor" class="check-icon">
                        <path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z"/>
                      </svg>
                    </button>
                    <div class="sort-divider"></div>
                    <button :class="['sort-option', { active: sortType === 'credits' && sortOrder === 'desc' }]" @click="setSortType('credits', 'desc')">
                      <span>{{ $t('windsurf.sortByCreditsDesc') }}</span>
                      <svg v-if="sortType === 'credits' && sortOrder === 'desc'" width="16" height="16" viewBox="0 0 24 24" fill="currentColor" class="check-icon">
                        <path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z"/>
                      </svg>
                    </button>
                    <button :class="['sort-option', { active: sortType === 'credits' && sortOrder === 'asc' }]" @click="setSortType('credits', 'asc')">
                      <span>{{ $t('windsurf.sortByCreditsAsc') }}</span>
                      <svg v-if="sortType === 'credits' && sortOrder === 'asc'" width="16" height="16" viewBox="0 0 24 24" fill="currentColor" class="check-icon">
                        <path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z"/>
                      </svg>
                    </button>
                  </div>
                </Transition>
              </div>

              <button
                @click="locateCurrentLoginAccount"
                class="btn locate-account-btn small"
                :disabled="locatingCurrentAccount || accounts.length === 0"
                :title="$t('windsurf.locateCurrentAccount')"
              >
                <span v-if="locatingCurrentAccount" class="sync-spinner"></span>
                <svg v-else width="14" height="14" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
                  <path d="M12 2a8 8 0 0 0-8 8c0 5.4 7 11.5 7.3 11.8a1 1 0 0 0 1.4 0C13 21.5 20 15.4 20 10a8 8 0 0 0-8-8zm0 11a3 3 0 1 1 0-6 3 3 0 0 1 0 6z"/>
                </svg>
                {{ locatingCurrentAccount ? $t('windsurf.locatingCurrentAccount') : $t('windsurf.locateCurrentAccount') }}
              </button>

              <button @click="showBatchDeleteDialog = true" class="batch-delete-icon-btn" :disabled="accounts.length === 0" :title="$t('windsurf.batchDelete')">
                <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/>
                </svg>
              </button>

              <span class="account-count">{{ filteredAccounts.length }} / {{ accounts.length }}</span>
            </div>

            <div class="account-grid-scroll">
              <div ref="accountGridRef" class="account-grid" :style="gridStyle">
                <WindsurfAccountCard
                  v-for="account in filteredAccounts"
                  :key="account.id"
                  :account="account"
                  :is-current-login="isCurrentLoginAccount(account)"
                  :highlight-current-login="isHighlightedCurrentLoginAccount(account)"
                  :data-account-email="normalizeEmail(account.email)"
                  @refresh-account="handleRefreshAccount"
                  @switch-account="handleSwitchAccount"
                  @delete-local="handleDeleteLocal"
                  @show-analytics="handleShowAnalytics"
                  @show-toast="showToast"
                />
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <Teleport to="body">
      <Transition name="toast">
        <div v-if="toast.show" class="toast" :class="toast.type">
          <span class="toast-message">{{ toast.message }}</span>
        </div>
      </Transition>
    </Teleport>

    <Teleport to="body">
      <Transition name="modal" appear>
        <div v-if="showAuth1Dialog" class="import-overlay" @click="showAuth1Dialog = false">
          <div class="import-dialog" @click.stop style="max-width: 560px;">
            <div class="dialog-header">
              <h3>{{ tokenImportTitle }}</h3>
              <button @click="showAuth1Dialog = false" class="dialog-close">×</button>
            </div>

            <div class="dialog-body">
              <p class="dialog-message">{{ tokenImportHint }}</p>
              <textarea
                v-model="auth1Text"
                rows="8"
                class="import-textarea"
                :placeholder="tokenImportPlaceholder"
              ></textarea>
              <label v-if="tokenImportMode === 'auth1'" class="condition-item" style="margin-top: 10px;">
                <input type="checkbox" v-model="auth1AutoPickPrimary" />
                <span>{{ $t('windsurf.auth1AutoPickPrimary') }}</span>
              </label>
              <p v-if="auth1Progress" class="dialog-message" style="margin-top: 10px; opacity: 0.8;">
                {{ auth1Progress }}
              </p>
              <div v-if="auth1LastErrors.length > 0" class="auth1-error-list">
                <div class="auth1-error-title">{{ $t('windsurf.auth1ErrorsTitle', { count: auth1LastErrors.length }) }}</div>
                <div v-for="(err, idx) in auth1LastErrors" :key="idx" class="auth1-error-item">
                  <div class="auth1-error-label">#{{ idx + 1 }} {{ err.label }}<span v-if="err.token" class="auth1-error-token">{{ err.token }}</span></div>
                  <div class="auth1-error-reason">{{ err.reason }}</div>
                </div>
              </div>
            </div>

            <div class="dialog-footer">
              <button @click="showAuth1Dialog = false" class="btn-cancel" :disabled="isAuth1Importing">
                {{ $t('common.cancel') }}
              </button>
              <button @click="executeAddAuth1" class="btn-confirm" :disabled="isAuth1Importing || !canAddAuth1">
                <span v-if="isAuth1Importing" class="sync-spinner"></span>
                {{ isAuth1Importing ? $t('windsurf.addingAuth1') : tokenImportConfirmText }}
              </button>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>

    <Teleport to="body">
      <Transition name="modal" appear>
        <div v-if="showOrgPickerDialog" class="import-overlay" @click="cancelOrgPicker">
          <div class="import-dialog" @click.stop style="max-width: 480px;">
            <div class="dialog-header">
              <h3>{{ $t('windsurf.orgPickerTitle') }}</h3>
              <button @click="cancelOrgPicker" class="dialog-close">×</button>
            </div>

            <div class="dialog-body">
              <p class="dialog-message">{{ $t('windsurf.orgPickerHint') }}</p>
              <div class="org-list">
                <label v-for="org in orgPickerOrgs" :key="org.id" class="org-item">
                  <input type="radio" v-model="orgPickerSelectedId" :value="org.id" />
                  <span class="org-name">{{ org.name || org.id }}</span>
                  <span class="org-id">{{ org.id }}</span>
                </label>
              </div>
            </div>

            <div class="dialog-footer">
              <button @click="cancelOrgPicker" class="btn-cancel" :disabled="isOrgPickerConfirming">
                {{ $t('common.cancel') }}
              </button>
              <button @click="confirmOrgPicker" class="btn-confirm" :disabled="isOrgPickerConfirming || !orgPickerSelectedId">
                <span v-if="isOrgPickerConfirming" class="sync-spinner"></span>
                {{ isOrgPickerConfirming ? $t('windsurf.addingAuth1') : $t('windsurf.orgPickerConfirm') }}
              </button>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>

    <Teleport to="body">
      <Transition name="modal" appear>
        <div v-if="showDeleteConfirm" class="import-overlay" @click="showDeleteConfirm = false">
          <div class="import-dialog" @click.stop style="max-width: 380px;">
            <div class="dialog-header">
              <h3>{{ $t('windsurf.deleteLocal') }}</h3>
              <button @click="showDeleteConfirm = false" class="dialog-close">×</button>
            </div>

            <div class="dialog-body">
              <p class="dialog-message">{{ $t('windsurf.deleteLocalConfirm') }}</p>
            </div>

            <div class="dialog-footer">
              <button @click="showDeleteConfirm = false" class="btn-cancel" :disabled="isDeleting">
                {{ $t('common.cancel') }}
              </button>
              <button @click="confirmDelete" class="btn-confirm" style="background: #ef4444;" :disabled="isDeleting">
                <span v-if="isDeleting" class="sync-spinner"></span>
                {{ isDeleting ? $t('windsurf.deleting') : $t('common.confirm') }}
              </button>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>

    <Teleport to="body">
      <Transition name="modal" appear>
        <div v-if="showBatchDeleteDialog" class="import-overlay" @click="closeBatchDeleteDialog">
          <div class="batch-delete-dialog" @click.stop>
            <div class="dialog-header">
              <h3>{{ $t('windsurf.batchDeleteTitle') }}</h3>
              <button @click="closeBatchDeleteDialog" class="dialog-close">×</button>
            </div>
            <div class="dialog-body">
              <p class="dialog-message">{{ $t('windsurf.batchDeleteMessage') }}</p>
              
              <div class="delete-mode-tabs">
                <button :class="['mode-tab', { active: batchDeleteMode === 'condition' }]" @click="batchDeleteMode = 'condition'">
                  {{ $t('windsurf.deleteByCondition') }}
                </button>
                <button :class="['mode-tab', { active: batchDeleteMode === 'manual' }]" @click="batchDeleteMode = 'manual'">
                  {{ $t('windsurf.deleteManual') }}
                </button>
              </div>

              <div v-if="batchDeleteMode === 'condition'" class="delete-conditions">
                <label class="condition-item">
                  <input type="checkbox" v-model="deleteNoCredits" />
                  <span>{{ $t('windsurf.deleteNoCreditsAccounts') }}</span>
                  <span class="condition-count">({{ noCreditsAccountsCount }})</span>
                </label>
                <label class="condition-item">
                  <input type="checkbox" v-model="deleteExpiredAccounts" />
                  <span>{{ $t('windsurf.deleteExpiredAccounts') }}</span>
                  <span class="condition-count">({{ expiredAccountsCount }})</span>
                </label>
                <label class="condition-item">
                  <input type="checkbox" v-model="deleteFreeAccounts" />
                  <span>{{ $t('windsurf.deleteFreeAccounts') }}</span>
                  <span class="condition-count">({{ freeAccountsCount }})</span>
                </label>
              </div>

              <div v-else class="manual-select">
                <div class="select-actions">
                  <button @click="selectAllForDelete" class="select-btn">{{ $t('windsurf.selectAll') }}</button>
                  <button @click="deselectAllForDelete" class="select-btn">{{ $t('windsurf.deselectAll') }}</button>
                </div>
                <div class="manual-list">
                  <label v-for="account in accounts" :key="account.id || account.email" class="manual-item">
                    <input type="checkbox" :value="account.email" v-model="selectedForBatchDelete" />
                    <span class="account-email">{{ account.email }}</span>
                    <span v-if="isFreeAccount(account)" class="tag free">{{ $t('windsurf.freeAccount') }}</span>
                    <span v-if="isNoCredits(account)" class="tag no-credits">{{ $t('windsurf.noCredits') }}</span>
                    <span v-if="isExpired(account)" class="tag expired">{{ $t('windsurf.expired') }}</span>
                  </label>
                </div>
              </div>

              <div class="delete-stats">
                <div class="stat-item total">
                  <span class="stat-label">{{ $t('windsurf.toDeleteCount') }}:</span>
                  <span class="stat-value">{{ batchToDeleteCount }} {{ $t('windsurf.items') }}</span>
                </div>
              </div>
              <p class="dialog-warning">{{ $t('windsurf.cannotUndo') }}</p>
            </div>
            <div class="dialog-footer">
              <button @click="closeBatchDeleteDialog" class="btn-cancel" :disabled="isBatchDeleting">
                {{ $t('common.cancel') }}
              </button>
              <button @click="executeBatchDelete" class="btn-confirm" style="background: #ef4444;" :disabled="isBatchDeleting || batchToDeleteCount === 0">
                <span v-if="isBatchDeleting" class="sync-spinner"></span>
                {{ isBatchDeleting ? $t('windsurf.deleting') : $t('windsurf.confirmBatchDelete') }} ({{ batchToDeleteCount }})
              </button>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>

    <Teleport to="body">
      <Transition name="modal" appear>
        <div v-if="showAutoSwitchDialog" class="import-overlay" @click="closeAutoSwitchDialog">
          <div class="auto-switch-dialog" @click.stop>
            <div class="dialog-header">
              <h3>{{ $t('windsurf.autoSwitchTitle') }}</h3>
              <button @click="closeAutoSwitchDialog" class="dialog-close">×</button>
            </div>
            <div class="dialog-body">
              <p class="dialog-message">{{ $t('windsurf.autoSwitchIntro') }}</p>
              <div class="auto-switch-form">
                <label class="condition-item">
                  <input type="checkbox" v-model="autoSwitchDraft.seamlessEnabled" :disabled="autoSwitchDraft.enabled" />
                  <span>{{ $t('windsurf.seamlessEnabled') }}</span>
                </label>
                <label class="condition-item">
                  <input type="checkbox" v-model="autoSwitchDraft.seamlessAutoApply" :disabled="autoSwitchDraft.enabled" />
                  <span>{{ $t('windsurf.seamlessAutoApply') }}</span>
                </label>
                <label class="condition-item">
                  <input type="checkbox" v-model="autoSwitchDraft.enabled" @change="handleAutoSwitchEnabledChange" />
                  <span>{{ $t('windsurf.autoSwitchEnabled') }}</span>
                </label>
                <div class="auto-switch-grid">
                  <label>
                    <span>{{ $t('windsurf.autoSwitchDailyThreshold') }}</span>
                    <input v-model.number="autoSwitchDraft.dailyThresholdPercent" type="number" min="0" max="100" />
                  </label>
                  <label>
                    <span>{{ $t('windsurf.autoSwitchWeeklyThreshold') }}</span>
                    <input v-model.number="autoSwitchDraft.weeklyThresholdPercent" type="number" min="0" max="100" />
                  </label>
                </div>
                <button
                  type="button"
                  class="auto-switch-advanced-toggle"
                  @click="autoSwitchDraft.advancedVisible = !autoSwitchDraft.advancedVisible"
                >
                  {{ autoSwitchDraft.advancedVisible ? $t('windsurf.autoSwitchHideAdvanced') : $t('windsurf.autoSwitchShowAdvanced') }}
                </button>
                <div v-if="autoSwitchDraft.advancedVisible" class="auto-switch-grid">
                  <label>
                    <span>{{ $t('windsurf.autoSwitchInterval') }}</span>
                    <input v-model.number="autoSwitchDraft.intervalSeconds" type="number" min="15" />
                  </label>
                  <label>
                    <span>{{ $t('windsurf.autoSwitchIdleInterval') }}</span>
                    <input v-model.number="autoSwitchDraft.idleIntervalSeconds" type="number" min="30" />
                  </label>
                  <label>
                    <span>{{ $t('windsurf.autoSwitchIdleChecks') }}</span>
                    <input v-model.number="autoSwitchDraft.idleAfterUnchangedChecks" type="number" min="1" />
                  </label>
                  <label>
                    <span>{{ $t('windsurf.autoSwitchCooldown') }}</span>
                    <input v-model.number="autoSwitchDraft.cooldownSeconds" type="number" min="5" />
                  </label>
                  <label>
                    <span>{{ $t('windsurf.autoSwitchAllExhaustedCooldown') }}</span>
                    <input v-model.number="autoSwitchDraft.allExhaustedCooldownSeconds" type="number" min="60" />
                  </label>
                </div>
              </div>
              <p v-if="autoSwitchRuntime.message" class="dialog-message auto-switch-status-text">
                {{ autoSwitchRuntime.message }}
              </p>
            </div>
            <div class="dialog-footer">
              <button @click="closeAutoSwitchDialog" class="btn-cancel" :disabled="autoSwitchRuntime.busy">
                {{ $t('common.cancel') }}
              </button>
              <button @click="saveAutoSwitchSettings" class="btn-confirm" :disabled="autoSwitchRuntime.busy">
                <span v-if="autoSwitchRuntime.busy" class="sync-spinner"></span>
                {{ $t('windsurfTarget.save') }}
              </button>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>

    <Teleport to="body">
      <Transition name="modal" appear>
        <WindsurfAnalyticsDialog
          v-if="showAnalyticsDialog && analyticsAccount"
          :account="analyticsAccount"
          @close="closeAnalyticsDialog"
          @show-toast="showToast"
        />
      </Transition>
    </Teleport>
  </div>
</template>

<script setup>
import { ref, computed, nextTick, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useI18n } from 'vue-i18n'
import WindsurfAccountCard from './WindsurfAccountCard.vue'
import WindsurfAnalyticsDialog from './WindsurfAnalyticsDialog.vue'
import { useEscToClose } from '../composables/useEscToClose'
import { buildInvokeTarget } from '../composables/windsurfTarget'
import {
  loadAutoSwitchConfig,
  loadLastSwitchEmail,
  normalizeAutoSwitchConfig,
  saveAutoSwitchConfig,
  saveLastSwitchEmail,
} from '../composables/windsurfAutoSwitch'

const props = defineProps({
  embedded: {
    type: Boolean,
    default: false
  }
})

const emit = defineEmits(['close'])
const { t } = useI18n()

const accounts = ref([])
const isLoading = ref(false)
const isRefreshing = ref(false)
const searchQuery = ref('')
const accountGridRef = ref(null)
const locatingCurrentAccount = ref(false)
const currentLoginEmail = ref('')
const highlightedLoginEmail = ref('')
let currentLoginHighlightTimer = null
let unlistenAutoSwitchEvent = null
let autoSwitchSnapshotSyncTimer = null

// Auth1 Token 手动添加
const showAuth1Dialog = ref(false)
const auth1Text = ref('')
const tokenImportMode = ref('auth1')
const auth1AutoPickPrimary = ref(true)
const isAuth1Importing = ref(false)
const auth1Progress = ref('')
const auth1LastErrors = ref([])

// 多组织选择子对话框
const showOrgPickerDialog = ref(false)
const orgPickerOrgs = ref([])
const orgPickerSelectedId = ref('')
const isOrgPickerConfirming = ref(false)
// 记录当前在多组织选择中的 token & 额外元数据（用于确认后继续流程）
const pendingAuth1 = ref(null)
// 多组织选择的 Promise resolver（由 executeAddAuth1 内部等待）
const orgPickerResolver = ref(null)

const showDeleteConfirm = ref(false)
const deleteTargetAccount = ref(null)
const isDeleting = ref(false)

const sortType = ref('expiry')
const sortOrder = ref('asc')
const showSortMenu = ref(false)

const showAnalyticsDialog = ref(false)
const analyticsAccount = ref(null)

const showBatchDeleteDialog = ref(false)
const isBatchDeleting = ref(false)
const batchDeleteMode = ref('condition')
const deleteNoCredits = ref(false)
const deleteExpiredAccounts = ref(false)
const deleteFreeAccounts = ref(false)
const selectedForBatchDelete = ref([])

const showAutoSwitchDialog = ref(false)
const autoSwitchConfig = ref(loadAutoSwitchConfig())
const autoSwitchDraft = ref({ ...autoSwitchConfig.value })
const autoSwitchTimer = ref(null)
const autoSwitchTimerMeta = ref({ dueAt: 0, lightProbe: false })
const autoSwitchRuntime = ref({
  busy: false,
  idle: false,
  unchangedChecks: 0,
  lastQuotaSignature: '',
  cooldownUntil: 0,
  message: '',
  lastCheckAt: null,
  lastSwitchAt: null,
  lastActivityWakeAt: null,
  lastFullIdleCheckAt: 0,
  nextDueAt: 0,
})

const CACHE_KEY = 'windsurf_accounts_cache_v1'
const AUTO_SWITCH_VERIFY_TIMEOUT_MS = 15000
const AUTO_SWITCH_VERIFY_POLL_MS = 1000
const AUTO_SWITCH_MAX_ATTEMPTS = 3
const AUTO_SWITCH_FAILURE_BASE_MS = 60000
const AUTO_SWITCH_FAILURE_MAX_MS = 900000
const AUTO_SWITCH_ACTIVITY_WAKE_THROTTLE_MS = 60000
const AUTO_SWITCH_ACTIVITY_WAKE_DELAY_SECONDS = 5
const AUTO_SWITCH_ACTIVITY_WAKE_SKIP_WITHIN_SECONDS = 15
const AUTO_SWITCH_IDLE_FRONTMOST_PROBE_SECONDS = 30
const AUTO_SWITCH_COOLDOWN_FRONTMOST_PROBE_SECONDS = 15

const toast = ref({ show: false, message: '', type: 'success' })
const autoSwitchFailedUntil = new Map()
const autoSwitchFailedCount = new Map()

const showToast = (message, type = 'success') => {
  toast.value = { show: true, message, type }
  setTimeout(() => {
    toast.value.show = false
  }, 3000)
}

const handleClose = () => {
  emit('close')
}

const handleAddAuth1 = () => {
  tokenImportMode.value = 'auth1'
  auth1Text.value = ''
  auth1AutoPickPrimary.value = true
  auth1Progress.value = ''
  auth1LastErrors.value = []
  showAuth1Dialog.value = true
}

const handleAddSession = () => {
  tokenImportMode.value = 'session'
  auth1Text.value = ''
  auth1AutoPickPrimary.value = false
  auth1Progress.value = ''
  auth1LastErrors.value = []
  showAuth1Dialog.value = true
}

const canAddAuth1 = computed(() => {
  return Boolean(auth1Text.value && auth1Text.value.trim())
})

const tokenImportTitle = computed(() => {
  return tokenImportMode.value === 'session' ? t('windsurf.addSessionTitle') : t('windsurf.addAuth1Title')
})

const tokenImportHint = computed(() => {
  return tokenImportMode.value === 'session' ? t('windsurf.addSessionHint') : t('windsurf.addAuth1Hint')
})

const tokenImportPlaceholder = computed(() => {
  return tokenImportMode.value === 'session' ? t('windsurf.addSessionPlaceholder') : t('windsurf.addAuth1Placeholder')
})

const tokenImportConfirmText = computed(() => {
  return tokenImportMode.value === 'session' ? t('windsurf.confirmAddSession') : t('windsurf.confirmAddAuth1')
})

const isNoCredits = (account) => {
  const total = account.total_credits ?? 0
  const used = account.used_credits ?? 0
  return (total - used) <= 0
}

const isFreeAccount = (account) => {
  const text = [
    account?.plan_name,
    account?.teams_tier_name,
    account?.plan_type,
    account?.tier
  ].filter(Boolean).join(' ').toLowerCase()
  return /\bfree\b|免费/.test(text)
}

const getExpireTimestamp = (account) => {
  if (!account?.expires_at) return Number.POSITIVE_INFINITY
  const timestamp = new Date(account.expires_at).getTime()
  return Number.isFinite(timestamp) ? timestamp : Number.POSITIVE_INFINITY
}

const isExpired = (account) => {
  return getExpireTimestamp(account) < Date.now()
}

const getAccountSearchText = (account) => {
  return [
    account?.email,
    account?.auth_token,
    account?.session_token,
    account?.plan_name,
    account?.teams_tier,
    account?.teams_tier_name,
    account?.plan_type,
    account?.tier
  ].filter(Boolean).join(' ').toLowerCase()
}

const noCreditsAccountsCount = computed(() => {
  return accounts.value.filter(a => isNoCredits(a)).length
})

const expiredAccountsCount = computed(() => {
  return accounts.value.filter(a => isExpired(a)).length
})

const freeAccountsCount = computed(() => {
  return accounts.value.filter(a => isFreeAccount(a)).length
})

const batchToDeleteCount = computed(() => {
  if (batchDeleteMode.value === 'manual') {
    return selectedForBatchDelete.value.length
  }
  const toDelete = new Set()
  if (deleteNoCredits.value) {
    accounts.value.filter(a => isNoCredits(a)).forEach(a => toDelete.add(a.email))
  }
  if (deleteExpiredAccounts.value) {
    accounts.value.filter(a => isExpired(a)).forEach(a => toDelete.add(a.email))
  }
  if (deleteFreeAccounts.value) {
    accounts.value.filter(a => isFreeAccount(a)).forEach(a => toDelete.add(a.email))
  }
  return toDelete.size
})

const selectAllForDelete = () => {
  selectedForBatchDelete.value = accounts.value.map(a => a.email)
}

const deselectAllForDelete = () => {
  selectedForBatchDelete.value = []
}

const closeBatchDeleteDialog = () => {
  showBatchDeleteDialog.value = false
  batchDeleteMode.value = 'condition'
  deleteNoCredits.value = false
  deleteExpiredAccounts.value = false
  deleteFreeAccounts.value = false
  selectedForBatchDelete.value = []
}

const executeBatchDelete = async () => {
  let emailsToDelete = []
  
  if (batchDeleteMode.value === 'manual') {
    emailsToDelete = [...selectedForBatchDelete.value]
  } else {
    const toDelete = new Set()
    if (deleteNoCredits.value) {
      accounts.value.filter(a => isNoCredits(a)).forEach(a => toDelete.add(a.email))
    }
    if (deleteExpiredAccounts.value) {
      accounts.value.filter(a => isExpired(a)).forEach(a => toDelete.add(a.email))
    }
    if (deleteFreeAccounts.value) {
      accounts.value.filter(a => isFreeAccount(a)).forEach(a => toDelete.add(a.email))
    }
    emailsToDelete = Array.from(toDelete)
  }

  if (emailsToDelete.length === 0) return

  isBatchDeleting.value = true
  try {
    let successCount = 0
    let failCount = 0

    for (const email of emailsToDelete) {
      const account = accounts.value.find(a => a.email === email)
      if (!account) continue

      try {
        const keyEmail = (email || '').toLowerCase().trim()
        accounts.value = accounts.value.filter(a => (a.email || '').toLowerCase().trim() !== keyEmail)
        successCount++
      } catch {
        failCount++
      }
    }

    saveCache()
    closeBatchDeleteDialog()

    if (failCount === 0) {
      showToast(t('windsurf.batchDeleteSuccess', { count: successCount }), 'success')
    } else {
      showToast(t('windsurf.batchDeletePartial', { success: successCount, fail: failCount }), 'warning')
    }
  } catch (e) {
    showToast(`${t('windsurf.batchDeleteFailed')}: ${e.message || e}`, 'error')
  } finally {
    isBatchDeleting.value = false
  }
}

const sortedAccounts = computed(() => {
  if (accounts.value.length === 0) return []

  return [...accounts.value].sort((a, b) => {
    let comparison = 0

    if (sortType.value === 'expiry') {
      const dateA = getExpireTimestamp(a)
      const dateB = getExpireTimestamp(b)
      comparison = dateA - dateB
    } else if (sortType.value === 'credits') {
      const getRemaining = (acc) => {
        const total = acc.total_credits ?? 0
        const used = acc.used_credits ?? 0
        return total - used
      }
      comparison = getRemaining(a) - getRemaining(b)
    }

    return sortOrder.value === 'desc' ? -comparison : comparison
  })
})

const filteredAccounts = computed(() => {
  if (!searchQuery.value.trim()) return sortedAccounts.value
  const q = searchQuery.value.toLowerCase().trim()
  return sortedAccounts.value.filter(a => getAccountSearchText(a).includes(q))
})

// 根据当前卡片数切换网格列定义：
// - 只有 1 张时，限制最大宽度（不拉伸占满整行）
// - 2 张及以上用 auto-fit + 1fr 平分铺满容器（无右侧空白）
const gridStyle = computed(() => {
  const count = filteredAccounts.value.length
  if (count <= 1) {
    return { gridTemplateColumns: 'minmax(280px, 420px)' }
  }
  return { gridTemplateColumns: 'repeat(auto-fit, minmax(280px, 1fr))' }
})

const setSortType = (type, order) => {
  sortType.value = type
  sortOrder.value = order
  showSortMenu.value = false
}

const sanitizeForCache = (acc) => {
  const { __loggingIn, __refreshingCredits, __updating, ...rest } = acc || {}
  return rest
}

const saveCache = () => {
  try {
    const payload = accounts.value.map(sanitizeForCache)
    localStorage.setItem(CACHE_KEY, JSON.stringify(payload))
  } catch {
  }
  scheduleAutoSwitchSnapshotSync()
}

const normalizeEmail = (email) => String(email || '').trim().toLowerCase()

const isCurrentLoginAccount = (account) => {
  return Boolean(currentLoginEmail.value && normalizeEmail(account?.email) === currentLoginEmail.value)
}

const isHighlightedCurrentLoginAccount = (account) => {
  return Boolean(highlightedLoginEmail.value && normalizeEmail(account?.email) === highlightedLoginEmail.value)
}

const getQuotaPercent = (account, field) => {
  const value = Number(account?.[field])
  return Number.isFinite(value) ? Math.max(0, Math.min(100, Math.round(value))) : null
}

const quotaDecision = (account, config) => {
  const daily = getQuotaPercent(account, 'daily_quota_remaining_percent')
  const weekly = getQuotaPercent(account, 'weekly_quota_remaining_percent')
  const hasSignal = daily !== null || weekly !== null
  if (!hasSignal) {
    return { hasSignal: false, shouldSwitch: false, reason: '' }
  }
  if (daily !== null && daily <= config.dailyThresholdPercent) {
    return { hasSignal: true, shouldSwitch: true, reason: `${t('windsurf.dailyQuota')} ${daily}%` }
  }
  if (weekly !== null && weekly <= config.weeklyThresholdPercent) {
    return { hasSignal: true, shouldSwitch: true, reason: `${t('windsurf.weeklyQuota')} ${weekly}%` }
  }
  return { hasSignal: true, shouldSwitch: false, reason: '' }
}

const quotaSignature = (account) => [
  account?.daily_quota_remaining_percent ?? '',
  account?.weekly_quota_remaining_percent ?? '',
  account?.daily_quota_reset_at_unix ?? '',
  account?.weekly_quota_reset_at_unix ?? '',
].join('|')

const delay = (ms) => new Promise(resolve => setTimeout(resolve, ms))

const setAutoSwitchMessage = (message) => {
  autoSwitchRuntime.value.message = message
}

const applyAutoSwitchStatus = (status) => {
  if (!status || typeof status !== 'object') return
  autoSwitchRuntime.value.busy = status.busy === true
  autoSwitchRuntime.value.idle = status.idle === true
  autoSwitchRuntime.value.unchangedChecks = Number(status.unchangedChecks || 0)
  autoSwitchRuntime.value.cooldownUntil = Number(status.cooldownUntil || 0)
  autoSwitchRuntime.value.nextDueAt = Number(status.nextDueAt || 0)
  autoSwitchRuntime.value.lastCheckAt = status.lastCheckAt || null
  autoSwitchRuntime.value.lastSwitchAt = status.lastSwitchAt || null
  autoSwitchRuntime.value.lastActivityWakeAt = status.lastActivityWakeAt || null
  if (typeof status.message === 'string') autoSwitchRuntime.value.message = status.message
}

const autoSwitchSnapshotPayload = (immediate = false) => ({
  config: autoSwitchConfig.value,
  accounts: accounts.value.map(sanitizeForCache),
  windsurfTarget: buildInvokeTarget(),
  lastSwitchEmail: loadLastSwitchEmail(),
  immediate
})

const syncAutoSwitchSnapshot = async ({ immediate = false } = {}) => {
  try {
    const status = await invoke('windsurf_auto_switch_update_snapshot', {
      snapshot: autoSwitchSnapshotPayload(immediate)
    })
    applyAutoSwitchStatus(status)
  } catch (e) {
    setAutoSwitchMessage(`${t('windsurf.autoSwitchCheckFailed')}: ${e.message || e}`)
  }
}

const scheduleAutoSwitchSnapshotSync = () => {
  if (autoSwitchSnapshotSyncTimer) clearTimeout(autoSwitchSnapshotSyncTimer)
  autoSwitchSnapshotSyncTimer = setTimeout(() => {
    autoSwitchSnapshotSyncTimer = null
    syncAutoSwitchSnapshot()
  }, 100)
}

const mergeAutoSwitchAccount = (updated) => {
  if (!updated || !updated.email) return null
  const email = normalizeEmail(updated.email)
  const existingIndex = accounts.value.findIndex(account => normalizeEmail(account.email) === email)
  if (existingIndex >= 0) {
    const existing = accounts.value[existingIndex]
    const merged = normalizeLocalItem({ ...existing, ...updated })
    merged.__loggingIn = existing.__loggingIn || false
    merged.__refreshingCredits = existing.__refreshingCredits || false
    merged.__updating = existing.__updating || false
    accounts.value[existingIndex] = merged
    return accounts.value[existingIndex]
  }
  accounts.value.unshift(normalizeLocalItem(updated))
  return accounts.value[0]
}

const handleAutoSwitchEvent = async (event) => {
  const payload = event?.payload || {}
  applyAutoSwitchStatus(payload.status)
  if (payload.account) {
    mergeAutoSwitchAccount(payload.account)
    saveCache()
  }
  if (payload.kind === 'switched') {
    const email = normalizeEmail(payload.email || payload.account?.email)
    if (email) {
      saveLastSwitchEmail(email)
      await revealCurrentLoginAccount(email)
    }
    if (payload.message) showToast(payload.message, 'success')
  }
}

const setupAutoSwitchEventListener = async () => {
  if (unlistenAutoSwitchEvent) return
  unlistenAutoSwitchEvent = await listen('windsurf-auto-switch-event', event => {
    handleAutoSwitchEvent(event)
  })
}

const clearAutoSwitchTimer = () => {
  if (autoSwitchTimer.value) {
    clearTimeout(autoSwitchTimer.value)
    autoSwitchTimer.value = null
  }
  autoSwitchTimerMeta.value = { dueAt: 0, lightProbe: false }
}

const scheduleAutoSwitch = (delaySeconds, { lightProbe = false } = {}) => {
  clearAutoSwitchTimer()
  const config = autoSwitchConfig.value
  if (!config.enabled) return
  const safeDelay = Math.max(1, Math.round(delaySeconds))
  const timer = setTimeout(() => {
    autoSwitchTick()
  }, safeDelay * 1000)
  autoSwitchTimer.value = timer
  autoSwitchTimerMeta.value = {
    dueAt: Date.now() + safeDelay * 1000,
    lightProbe,
  }
}

const markAutoSwitchFailure = (email) => {
  const key = normalizeEmail(email)
  const count = (autoSwitchFailedCount.get(key) || 0) + 1
  autoSwitchFailedCount.set(key, count)
  const retryMs = Math.min(AUTO_SWITCH_FAILURE_BASE_MS * Math.pow(2, Math.min(count - 1, 4)), AUTO_SWITCH_FAILURE_MAX_MS)
  autoSwitchFailedUntil.set(key, Date.now() + retryMs)
}

const markAutoSwitchSuccess = (email) => {
  const key = normalizeEmail(email)
  autoSwitchFailedCount.delete(key)
  autoSwitchFailedUntil.delete(key)
}

const applySeamlessPatchIfNeeded = async (config, { silent = true } = {}) => {
  if (!config.seamlessEnabled || !config.seamlessAutoApply) return true
  const target = buildInvokeTarget()
  try {
    const status = await invoke('check_seamless_patch_status', {
      windsurfTarget: target,
      windsurf_target: target
    })
    if (status && status.installed && status.current_version !== false) return true
    const resp = await invoke('apply_seamless_patch', {
      windsurfTarget: target,
      windsurf_target: target
    })
    const ok = !!(resp && resp.success !== false)
    if (!ok && !silent) showToast(`${t('windsurf.seamlessAutoApplyFailed')}: ${resp?.error || ''}`.trim(), 'error')
    return ok
  } catch (e) {
    if (!silent) showToast(`${t('windsurf.seamlessAutoApplyFailed')}: ${e.message || e}`, 'error')
    return false
  }
}

const refreshAutoSwitchAccount = async (account) => {
  const ok = await handleRefreshCredits(account, { silentSuccess: true, silentError: true })
  if (!ok) return null
  return account
}

const getCurrentLoginEmail = async () => {
  try {
    const target = buildInvokeTarget()
    const resp = await invoke('windsurf_get_current_login', {
      windsurfTarget: target,
      windsurf_target: target
    })
    if (resp && resp.success && resp.email) return normalizeEmail(resp.email)
  } catch {
  }
  return ''
}

const scrollToCurrentLoginAccount = async (email) => {
  await nextTick()
  const grid = accountGridRef.value
  const selector = `[data-account-email="${CSS.escape(email)}"]`
  const card = grid?.querySelector?.(selector)
  if (!card) return false
  card.scrollIntoView({ behavior: 'smooth', block: 'center', inline: 'nearest' })
  highlightedLoginEmail.value = email
  if (currentLoginHighlightTimer) clearTimeout(currentLoginHighlightTimer)
  currentLoginHighlightTimer = setTimeout(() => {
    if (highlightedLoginEmail.value === email) highlightedLoginEmail.value = ''
    currentLoginHighlightTimer = null
  }, 5000)
  return true
}

const revealCurrentLoginAccount = async (email) => {
  const normalized = normalizeEmail(email)
  if (!normalized) return false
  currentLoginEmail.value = normalized
  const visible = filteredAccounts.value.some(account => normalizeEmail(account.email) === normalized)
  if (!visible) searchQuery.value = ''
  return scrollToCurrentLoginAccount(normalized)
}

const locateCurrentLoginAccount = async () => {
  if (locatingCurrentAccount.value) return
  locatingCurrentAccount.value = true
  try {
    const email = await getCurrentLoginEmail()
    currentLoginEmail.value = email
    if (!email) {
      showToast(t('windsurf.currentLoginNotFound'), 'warning')
      return
    }
    const matched = accounts.value.find(account => normalizeEmail(account.email) === email)
    if (!matched) {
      showToast(t('windsurf.currentLoginNotInList', { email }), 'warning')
      return
    }
    const scrolled = await revealCurrentLoginAccount(email)
    if (scrolled) showToast(t('windsurf.currentLoginLocated', { email: matched.email || email }), 'success')
    else showToast(t('windsurf.currentLoginLocatedButHidden', { email: matched.email || email }), 'warning')
  } catch (e) {
    showToast(`${t('windsurf.locateCurrentAccountFailed')}: ${e.message || e}`, 'error')
  } finally {
    locatingCurrentAccount.value = false
  }
}

const getWindsurfWindowStatus = async () => {
  try {
    const target = buildInvokeTarget()
    const resp = await invoke('windsurf_get_window_status', {
      windsurfTarget: target,
      windsurf_target: target
    })
    return resp && typeof resp === 'object' ? resp : null
  } catch {
    return null
  }
}

const wakeAutoSwitchFromWindsurfActivity = async ({ allowCooldown = false } = {}) => {
  const config = autoSwitchConfig.value
  if (!config.enabled || autoSwitchRuntime.value.busy) return false
  const now = Date.now()
  const inCooldown = now < autoSwitchRuntime.value.cooldownUntil
  if (!autoSwitchRuntime.value.idle && !(allowCooldown && inCooldown)) return false
  if (inCooldown && !allowCooldown) return false
  const lastWake = autoSwitchRuntime.value.lastActivityWakeAt || 0
  const throttleMs = allowCooldown ? AUTO_SWITCH_COOLDOWN_FRONTMOST_PROBE_SECONDS * 1000 : AUTO_SWITCH_ACTIVITY_WAKE_THROTTLE_MS
  if (now - lastWake < throttleMs) return false
  const status = await getWindsurfWindowStatus()
  if (!status || !status.running || !status.frontmost) return false
  if (autoSwitchTimerMeta.value.dueAt) {
    const remainingSeconds = Math.ceil((autoSwitchTimerMeta.value.dueAt - now) / 1000)
    if (remainingSeconds > 0 && remainingSeconds <= AUTO_SWITCH_ACTIVITY_WAKE_SKIP_WITHIN_SECONDS) return false
  }
  autoSwitchRuntime.value.lastActivityWakeAt = now
  autoSwitchRuntime.value.cooldownUntil = 0
  autoSwitchRuntime.value.idle = false
  setAutoSwitchMessage(t('windsurf.autoSwitchActivityWake'))
  scheduleAutoSwitch(AUTO_SWITCH_ACTIVITY_WAKE_DELAY_SECONDS)
  return true
}

const currentAccountForAutoSwitch = async () => {
  const currentEmail = await getCurrentLoginEmail()
  if (currentEmail) {
    const matched = accounts.value.find(acc => normalizeEmail(acc.email) === currentEmail)
    if (matched) return matched
  }
  const lastEmail = loadLastSwitchEmail()
  if (lastEmail) {
    const matched = accounts.value.find(acc => normalizeEmail(acc.email) === lastEmail)
    if (matched) return matched
  }
  return accounts.value.find(acc => acc.auth_token) || null
}

const updateAutoSwitchIdle = (account, config) => {
  const signature = quotaSignature(account)
  if (signature && signature === autoSwitchRuntime.value.lastQuotaSignature) {
    autoSwitchRuntime.value.unchangedChecks += 1
  } else {
    autoSwitchRuntime.value.lastQuotaSignature = signature
    autoSwitchRuntime.value.unchangedChecks = 0
  }
  autoSwitchRuntime.value.idle = autoSwitchRuntime.value.unchangedChecks >= config.idleAfterUnchangedChecks
}

const enterAutoSwitchCooldown = (seconds, message) => {
  autoSwitchRuntime.value.cooldownUntil = Date.now() + Math.max(1, seconds) * 1000
  setAutoSwitchMessage(message)
}

const switchAccountInternal = async (account, { silent = false } = {}) => {
  const before = toast.value
  const ok = await handleSwitchAccount(account, { autoTriggered: true })
  if (silent) toast.value = before
  return ok
}

const verifyAutoSwitch = async (account, config) => {
  const email = normalizeEmail(account.email)
  const deadline = Date.now() + AUTO_SWITCH_VERIFY_TIMEOUT_MS
  let lastReason = t('windsurf.autoSwitchVerifyTimeout')
  while (Date.now() < deadline) {
    await delay(AUTO_SWITCH_VERIFY_POLL_MS)
    const currentEmail = await getCurrentLoginEmail()
    if (currentEmail && currentEmail !== email) {
      lastReason = t('windsurf.autoSwitchVerifyStillCurrent', { email: currentEmail })
      continue
    }
    const refreshed = await refreshAutoSwitchAccount(account)
    if (!refreshed) return { success: false, reason: t('windsurf.autoSwitchRefreshFailed') }
    const decision = quotaDecision(refreshed, config)
    if (!decision.hasSignal) return { success: false, reason: t('windsurf.autoSwitchNoQuotaSignal') }
    if (decision.shouldSwitch) return { success: false, reason: decision.reason }
    return { success: true }
  }
  return { success: false, reason: lastReason }
}

const trySwitchToNextAvailable = async (currentAccount, config, reason) => {
  const currentEmail = normalizeEmail(currentAccount.email)
  const hasCurrentAccount = accounts.value.some(acc => normalizeEmail(acc.email) === currentEmail)
  if (!hasCurrentAccount || accounts.value.length < 2) {
    enterAutoSwitchCooldown(config.allExhaustedCooldownSeconds, t('windsurf.autoSwitchNoCandidate'))
    return false
  }
  const candidates = accounts.value
    .map((account, index) => ({ account, index }))
    .filter(({ account }) => {
      const email = normalizeEmail(account.email)
      if (!email || email === currentEmail || !account.auth_token) return false
      if (isExpired(account) || isFreeAccount(account)) return false
      return true
    })
    .sort((a, b) => {
      const expiresComparison = getExpireTimestamp(a.account) - getExpireTimestamp(b.account)
      return expiresComparison || a.index - b.index
    })
  let attempts = 0
  let hasRetryableFailure = false
  let lastFailure = ''
  for (const { account: candidate } of candidates) {
    const email = normalizeEmail(candidate.email)
    const failedUntil = autoSwitchFailedUntil.get(email) || 0
    if (failedUntil > Date.now()) {
      hasRetryableFailure = true
      continue
    }
    const refreshed = await refreshAutoSwitchAccount(candidate)
    if (!refreshed) {
      hasRetryableFailure = true
      continue
    }
    const candidateDecision = quotaDecision(refreshed, config)
    if (!candidateDecision.hasSignal) {
      hasRetryableFailure = true
      continue
    }
    if (candidateDecision.shouldSwitch) continue
    if (attempts >= AUTO_SWITCH_MAX_ATTEMPTS) break
    attempts += 1
    await switchAccountInternal(refreshed, { silent: true })
    const verified = await verifyAutoSwitch(refreshed, config)
    if (!verified.success) {
      markAutoSwitchFailure(email)
      hasRetryableFailure = true
      lastFailure = `${refreshed.email}: ${verified.reason || ''}`.trim()
      continue
    }
    markAutoSwitchSuccess(email)
    saveLastSwitchEmail(refreshed.email)
    autoSwitchRuntime.value.lastSwitchAt = new Date().toISOString()
    await revealCurrentLoginAccount(refreshed.email)
    enterAutoSwitchCooldown(config.cooldownSeconds, t('windsurf.autoSwitchSwitched', { email: refreshed.email, reason }))
    showToast(t('windsurf.autoSwitchSwitched', { email: refreshed.email, reason }), 'success')
    return true
  }
  if (attempts > 0) {
    enterAutoSwitchCooldown(config.allExhaustedCooldownSeconds, t('windsurf.autoSwitchManualRequired', { reason: lastFailure || reason }))
    return false
  }
  if (hasRetryableFailure) {
    enterAutoSwitchCooldown(config.cooldownSeconds, t('windsurf.autoSwitchDeferred'))
    return false
  }
  enterAutoSwitchCooldown(config.allExhaustedCooldownSeconds, t('windsurf.autoSwitchNoCandidate'))
  return false
}

const autoSwitchCheckOnce = async () => {
  const config = autoSwitchConfig.value
  if (!config.enabled) {
    setAutoSwitchMessage(t('windsurf.autoSwitchDisabled'))
    return
  }
  if (accounts.value.length === 0) {
    setAutoSwitchMessage(t('windsurf.autoSwitchNoAccounts'))
    return
  }
  const seamlessReady = await applySeamlessPatchIfNeeded(config, { silent: false })
  if (!seamlessReady) {
    setAutoSwitchMessage(t('windsurf.autoSwitchSeamlessRequired'))
    autoSwitchConfig.value = saveAutoSwitchConfig({ ...config, enabled: false })
    return
  }
  const now = Date.now()
  if (now < autoSwitchRuntime.value.cooldownUntil) {
    setAutoSwitchMessage(t('windsurf.autoSwitchCooling'))
    return
  }
  const current = await currentAccountForAutoSwitch()
  if (!current) {
    setAutoSwitchMessage(t('windsurf.autoSwitchNoCurrent'))
    return
  }
  autoSwitchRuntime.value.lastCheckAt = new Date().toISOString()
  const refreshed = await refreshAutoSwitchAccount(current)
  if (!refreshed) {
    setAutoSwitchMessage(t('windsurf.autoSwitchRefreshFailed'))
    return
  }
  saveLastSwitchEmail(refreshed.email)
  updateAutoSwitchIdle(refreshed, config)
  const decision = quotaDecision(refreshed, config)
  if (!decision.hasSignal) {
    setAutoSwitchMessage(t('windsurf.autoSwitchNoQuotaSignal'))
    return
  }
  if (!decision.shouldSwitch) {
    setAutoSwitchMessage(autoSwitchRuntime.value.idle ? t('windsurf.autoSwitchIdleMode') : t('windsurf.autoSwitchQuotaOk'))
    return
  }
  autoSwitchRuntime.value.idle = false
  autoSwitchRuntime.value.unchangedChecks = 0
  await trySwitchToNextAvailable(refreshed, config, decision.reason)
}

const autoSwitchTick = async () => {
  const lightProbe = !!autoSwitchTimerMeta.value.lightProbe
  clearAutoSwitchTimer()
  const config = autoSwitchConfig.value
  if (!config.enabled || autoSwitchRuntime.value.busy) return
  const now = Date.now()
  const cooldownRemaining = Math.ceil((autoSwitchRuntime.value.cooldownUntil - now) / 1000)
  if (lightProbe && cooldownRemaining > 0) {
    const woke = await wakeAutoSwitchFromWindsurfActivity({ allowCooldown: true })
    if (!woke) {
      scheduleAutoSwitch(Math.min(AUTO_SWITCH_COOLDOWN_FRONTMOST_PROBE_SECONDS, cooldownRemaining), { lightProbe: true })
    }
    return
  }
  if (lightProbe && autoSwitchRuntime.value.idle) {
    const woke = await wakeAutoSwitchFromWindsurfActivity()
    if (!woke) {
      const lastFullCheck = autoSwitchRuntime.value.lastFullIdleCheckAt || Date.now()
      const fullCheckRemaining = Math.ceil((lastFullCheck + config.idleIntervalSeconds * 1000 - Date.now()) / 1000)
      if (fullCheckRemaining <= 0) scheduleAutoSwitch(1)
      else scheduleAutoSwitch(Math.min(AUTO_SWITCH_IDLE_FRONTMOST_PROBE_SECONDS, fullCheckRemaining), { lightProbe: true })
    }
    return
  }
  autoSwitchRuntime.value.busy = true
  try {
    await autoSwitchCheckOnce()
  } catch (e) {
    setAutoSwitchMessage(`${t('windsurf.autoSwitchCheckFailed')}: ${e.message || e}`)
  } finally {
    autoSwitchRuntime.value.busy = false
    const latest = autoSwitchConfig.value
    if (latest.enabled) {
      const cooldownRemaining = Math.ceil((autoSwitchRuntime.value.cooldownUntil - Date.now()) / 1000)
      if (cooldownRemaining > 0) {
        scheduleAutoSwitch(Math.min(AUTO_SWITCH_COOLDOWN_FRONTMOST_PROBE_SECONDS, cooldownRemaining), { lightProbe: true })
      }
      else if (autoSwitchRuntime.value.idle) {
        autoSwitchRuntime.value.lastFullIdleCheckAt = Date.now()
        scheduleAutoSwitch(Math.min(AUTO_SWITCH_IDLE_FRONTMOST_PROBE_SECONDS, latest.idleIntervalSeconds), { lightProbe: true })
      } else scheduleAutoSwitch(latest.intervalSeconds)
    }
  }
}

const startAutoSwitchService = async ({ immediate = false } = {}) => {
  clearAutoSwitchTimer()
  autoSwitchConfig.value = loadAutoSwitchConfig()
  if (!autoSwitchConfig.value.enabled) {
    setAutoSwitchMessage(t('windsurf.autoSwitchDisabled'))
    await syncAutoSwitchSnapshot()
    return
  }
  const seamlessReady = await applySeamlessPatchIfNeeded(autoSwitchConfig.value, { silent: true })
  if (!seamlessReady) {
    setAutoSwitchMessage(t('windsurf.autoSwitchSeamlessRequired'))
    autoSwitchConfig.value = saveAutoSwitchConfig({ ...autoSwitchConfig.value, enabled: false })
    await syncAutoSwitchSnapshot()
    return
  } else {
    setAutoSwitchMessage(t('windsurf.autoSwitchWaiting'))
  }
  await syncAutoSwitchSnapshot({ immediate })
}

const stopAutoSwitchService = () => {
  clearAutoSwitchTimer()
  autoSwitchRuntime.value.busy = false
  autoSwitchRuntime.value.idle = false
  autoSwitchRuntime.value.unchangedChecks = 0
  autoSwitchRuntime.value.cooldownUntil = 0
  autoSwitchRuntime.value.lastFullIdleCheckAt = 0
  setAutoSwitchMessage(t('windsurf.autoSwitchDisabled'))
  syncAutoSwitchSnapshot()
}

const requestAutoSwitchCheck = async () => {
  if (autoSwitchRuntime.value.busy) return
  try {
    const status = await invoke('windsurf_auto_switch_request_check')
    applyAutoSwitchStatus(status)
  } catch (e) {
    setAutoSwitchMessage(`${t('windsurf.autoSwitchCheckFailed')}: ${e.message || e}`)
  }
}

const closeAutoSwitchDialog = () => {
  showAutoSwitchDialog.value = false
  autoSwitchDraft.value = { ...autoSwitchConfig.value }
}

const handleAutoSwitchEnabledChange = () => {
  if (!autoSwitchDraft.value.enabled) return
  autoSwitchDraft.value.seamlessEnabled = true
  autoSwitchDraft.value.seamlessAutoApply = true
}

const saveAutoSwitchSettings = async () => {
  const next = normalizeAutoSwitchConfig(autoSwitchDraft.value)
  if (next.enabled) {
    next.seamlessEnabled = true
    next.seamlessAutoApply = true
    autoSwitchDraft.value.seamlessEnabled = true
    autoSwitchDraft.value.seamlessAutoApply = true
    const ok = await applySeamlessPatchIfNeeded(next, { silent: false })
    if (!ok) {
      next.enabled = false
      autoSwitchDraft.value.enabled = false
      setAutoSwitchMessage(t('windsurf.autoSwitchSeamlessRequired'))
    }
  }
  autoSwitchConfig.value = saveAutoSwitchConfig(next)
  autoSwitchDraft.value = { ...autoSwitchConfig.value }
  showAutoSwitchDialog.value = false
  if (autoSwitchConfig.value.enabled) {
    await startAutoSwitchService({ immediate: true })
    showToast(t('windsurf.autoSwitchSavedEnabled'), 'success')
  } else {
    stopAutoSwitchService()
    showToast(t('windsurf.autoSwitchSavedDisabled'), 'success')
  }
}

// ========== Auth1 Token 添加流程 ==========

// 解析 Auth1 输入：每行一个 auth1_token 或 `标签,auth1_token` / `标签:auth1_token`
// 仅提取 token；标签保留但由后端 GetCurrentUser 的 email 覆盖。
const parseAuth1Lines = (raw) => {
  const lines = String(raw || '')
    .split(/\r?\n/)
    .map(l => l.trim())
    .filter(Boolean)

  const out = []
  const seen = new Set()
  const pushItem = (token, label = null) => {
    const normalizedToken = String(token || '').trim()
    if (!normalizedToken || seen.has(normalizedToken)) return
    seen.add(normalizedToken)
    out.push({ label, token: normalizedToken })
  }

  for (const line of lines) {
    const tokens = Array.from(line.matchAll(/auth1_[A-Za-z0-9_\-]+/g)).map(match => match[0])
    if (tokens.length === 0) {
      continue
    }

    if (tokens.length === 1) {
      const singleToken = tokens[0]
      const escapedToken = singleToken.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
      const labeled = line.match(new RegExp(`^([^,\\t|:;\\s]+)\\s*[,\\t|:;]\\s*(${escapedToken})$`))
      if (labeled) {
        pushItem(singleToken, labeled[1] || null)
        continue
      }

      const bySpace = line.match(new RegExp(`^(\\S+)\\s+(${escapedToken})$`))
      if (bySpace) {
        pushItem(singleToken, bySpace[1] || null)
        continue
      }
    }

    for (const token of tokens) {
      pushItem(token, null)
    }
  }

  return out
}

const parseSessionLines = (raw) => {
  const lines = String(raw || '')
    .split(/\r?\n/)
    .map(l => l.trim())
    .filter(Boolean)

  const out = []
  const seen = new Set()
  const pushItem = (token, label = null) => {
    const normalizedToken = String(token || '').trim()
    if (!normalizedToken || seen.has(normalizedToken)) return
    seen.add(normalizedToken)
    out.push({ label, token: normalizedToken })
  }

  for (const line of lines) {
    const tokens = Array.from(line.matchAll(/devin-session-token\$[^\s,|:;]+/g)).map(match => match[0])
    if (tokens.length === 0) {
      continue
    }

    if (tokens.length === 1) {
      const singleToken = tokens[0]
      const escapedToken = singleToken.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
      const labeled = line.match(new RegExp(`^([^,\\t|:;\\s]+)\\s*[,\\t|:;]\\s*(${escapedToken})$`))
      if (labeled) {
        pushItem(singleToken, labeled[1] || null)
        continue
      }

      const bySpace = line.match(new RegExp(`^(${escapedToken})\\s+(.+)$`))
      if (bySpace) {
        pushItem(singleToken, bySpace[2] || null)
        continue
      }
    }

    for (const token of tokens) {
      pushItem(token, null)
    }
  }

  return out
}

// 打开多组织选择子对话框，返回 Promise，resolve(orgId) 或 resolve(null)（用户取消）
const askOrgSelection = (orgs) => {
  return new Promise((resolve) => {
    orgPickerOrgs.value = Array.isArray(orgs) ? orgs : []
    orgPickerSelectedId.value = orgs?.[0]?.id || ''
    orgPickerResolver.value = resolve
    showOrgPickerDialog.value = true
  })
}

const confirmOrgPicker = () => {
  if (!orgPickerSelectedId.value) return
  isOrgPickerConfirming.value = true
  const resolver = orgPickerResolver.value
  orgPickerResolver.value = null
  showOrgPickerDialog.value = false
  const selected = orgPickerSelectedId.value
  // 清理：保持 picker 状态为空，以便下次打开时新数据覆盖
  orgPickerOrgs.value = []
  orgPickerSelectedId.value = ''
  isOrgPickerConfirming.value = false
  if (typeof resolver === 'function') resolver(selected)
}

const cancelOrgPicker = () => {
  const resolver = orgPickerResolver.value
  orgPickerResolver.value = null
  showOrgPickerDialog.value = false
  orgPickerOrgs.value = []
  orgPickerSelectedId.value = ''
  isOrgPickerConfirming.value = false
  if (typeof resolver === 'function') resolver(null)
}

// 把后端返回的 account JSON 合并到本地列表
const upsertAuth1Account = (payload) => {
  if (!payload || !payload.email) return null
  const email = String(payload.email).trim()
  const existing = accounts.value.find(a => (a?.email || '').toLowerCase().trim() === email.toLowerCase())
  const base = existing || {}
  const merged = normalizeLocalItem({
    id: base.id || `auth1-${Date.now()}-${Math.random().toString(16).slice(2)}`,
    email,
    name: payload.name || base.name || email.split('@')[0] || '',
    first_name: payload.first_name || base.first_name || '',
    last_name: payload.last_name || base.last_name || '',
    api_key: payload.api_key || base.api_key || null,
    status: payload.status || base.status || 'active',
    plan_name: payload.plan_name || base.plan_name || null,
    team_id: payload.team_id || base.team_id || null,
    // Devin 体系：把 auth1_token 作为 auth_token 主字段，session_token 单独保留供分析复用
    auth_token: payload.auth_token || base.auth_token || null,
    session_token: payload.session_token || base.session_token || null,
    devin_account_id: payload.devin_account_id || base.devin_account_id || null,
    devin_primary_org_id: payload.devin_primary_org_id || base.devin_primary_org_id || null,
    auth_provider: 'auth1',
    user_name: base.user_name || null,
    ...pickCreditFields(base),
    raw_user_info: payload.raw_user_info || base.raw_user_info || null
  })
  if (existing) {
    const idx = accounts.value.indexOf(existing)
    if (idx >= 0) {
      accounts.value[idx] = merged
      return accounts.value[idx]
    }
    return existing
  }
  accounts.value.unshift(merged)
  // 注意：必须返回数组内元素（Vue 自动包装的 reactive 代理），
  // 直接返回 merged 会绕过响应式，导致后续属性修改不触发 UI 更新。
  return accounts.value[0]
}

// 通用批次并发：每次最多 batchSize 个同时执行，结果按原顺序返回
const runInBatches = async (list, handler, batchSize = 20) => {
  const results = new Array(list.length)
  for (let i = 0; i < list.length; i += batchSize) {
    const chunk = list.slice(i, i + batchSize)
    const r = await Promise.all(chunk.map((item, idx) => handler(item, i + idx)))
    for (let k = 0; k < r.length; k++) results[i + k] = r[k]
  }
  return results
}

const executeAddAuth1 = async () => {
  if (isAuth1Importing.value) return
  const isSessionImport = tokenImportMode.value === 'session'
  const items = isSessionImport ? parseSessionLines(auth1Text.value) : parseAuth1Lines(auth1Text.value)
  if (items.length === 0) {
    showToast(t(isSessionImport ? 'windsurf.sessionParseFailed' : 'windsurf.auth1ParseFailed'), 'error')
    return
  }

  isAuth1Importing.value = true
  auth1LastErrors.value = []
  let successCount = 0
  let failCount = 0
  let skippedCount = 0

  const mapImportError = (reason) => {
    const raw = String(reason ?? '').trim()
    if (!raw) return ''
    if (/\b403\b/.test(raw)) return t('windsurf.errorNetwork403')
    if (/\b503\b/.test(raw)) return t('windsurf.errorServerBusy503')
    return raw
  }

  const recordFailure = (label, token, reason) => {
    failCount++
    const masked = token ? `${token.slice(0, 12)}...${token.slice(-4)}` : ''
    const friendly = mapImportError(reason) || String(reason || '未知错误')
    auth1LastErrors.value.push({
      label: label || masked || '(token)',
      token: masked,
      reason: friendly
    })
    // eslint-disable-next-line no-console
    console.warn('[windsurf:token-import] add failed', { label, token: masked, reason: String(reason ?? '') })
  }

  try {
    // 阶段 A：8 并发 worker，每个 worker 串行执行 add → 若无需选组织则立刻 refresh credits
    // 需要选组织的 token 先把结果标记回传，阶段 B 串行处理（UI 限制）
    auth1Progress.value = t('windsurf.auth1Progress', { current: 0, total: items.length })
    let doneCount = 0
    const bumpProgress = () => {
      doneCount++
      auth1Progress.value = t('windsurf.auth1Progress', { current: doneCount, total: items.length })
    }

    const addResults = await runInBatches(items, async ({ token, label }) => {
      let resp
      try {
        if (isSessionImport) {
          resp = await invoke('windsurf_add_account_by_session_token', {
            sessionToken: token,
            session_token: token
          })
        } else {
          resp = await invoke('windsurf_add_account_by_auth1_token', {
            auth1Token: token,
            auth1_token: token,
            orgId: null,
            org_id: null,
            autoSelectPrimaryOrg: Boolean(auth1AutoPickPrimary.value),
            auto_select_primary_org: Boolean(auth1AutoPickPrimary.value)
          })
        }
      } catch (e) {
        bumpProgress()
        return { token, label, status: 'error', error: e?.message || String(e) }
      }

      if (!isSessionImport && resp && resp.requires_org_selection && Array.isArray(resp.orgs) && resp.orgs.length > 0) {
        // 推迟到阶段 B 让用户选组织（保持阶段 A 不阻塞）
        bumpProgress()
        return { token, label, status: 'needOrg', resp }
      }

      if (resp && resp.success && resp.account) {
        const acc = upsertAuth1Account(resp.account)
        if (!acc) {
          bumpProgress()
          return { token, label, status: 'error', error: '后端返回 success=true 但 account 解析失败' }
        }
        // 导入成功的账号立即并发刷新额度，不等整批完成
        try {
          await handleRefreshCredits(acc, { silentSuccess: true, silentError: true })
        } catch {
          // 刷新失败不影响导入计数，handleRefreshCredits 内部已处理错误
        }
        bumpProgress()
        return { token, label, status: 'ok' }
      }

      bumpProgress()
      return { token, label, status: 'error', error: resp?.error || '未知错误（后端未返回 error 字段）' }
    }, 100)

    // 阶段 B：串行处理需要选组织的 token（选完立即 add + refresh）
    for (const r of addResults) {
      if (r.status === 'ok') {
        successCount++
        continue
      }
      if (r.status === 'error') {
        recordFailure(r.label, r.token, r.error)
        continue
      }
      // status === 'needOrg'
      const effectiveToken = r.resp?.auth1_token || r.token
      const orgId = await askOrgSelection(r.resp.orgs)
      if (!orgId) {
        skippedCount++
        continue
      }
      let finalResp
      try {
        finalResp = await invoke('windsurf_add_account_by_auth1_token', {
          auth1Token: effectiveToken,
          auth1_token: effectiveToken,
          orgId,
          org_id: orgId,
          autoSelectPrimaryOrg: false,
          auto_select_primary_org: false
        })
      } catch (e) {
        recordFailure(r.label, effectiveToken, e?.message || e)
        continue
      }

      if (finalResp && finalResp.success && finalResp.account) {
        const acc = upsertAuth1Account(finalResp.account)
        if (acc) {
          successCount++
          try {
            await handleRefreshCredits(acc, { silentSuccess: true, silentError: true })
          } catch {
            // 同上
          }
        } else {
          recordFailure(r.label, r.token, '后端返回 success=true 但 account 解析失败')
        }
      } else {
        recordFailure(r.label, r.token, finalResp?.error || '未知错误（后端未返回 error 字段）')
      }
    }

    saveCache()

    if (failCount === 0 && skippedCount === 0) {
      showToast(t(isSessionImport ? 'windsurf.sessionAddSuccess' : 'windsurf.auth1AddSuccess', { count: successCount }), 'success')
    } else if (successCount > 0) {
      showToast(t(isSessionImport ? 'windsurf.sessionAddPartial' : 'windsurf.auth1AddPartial', { success: successCount, fail: failCount, skipped: skippedCount }), 'warning')
    } else {
      const first = auth1LastErrors.value[0]
      const hint = first ? `：${first.reason}` : ''
      showToast(`${t(isSessionImport ? 'windsurf.sessionAddAllFailed' : 'windsurf.auth1AddAllFailed')}${hint}`, 'error')
    }

    if (successCount > 0 && failCount === 0) {
      showAuth1Dialog.value = false
      auth1Text.value = ''
    }
  } finally {
    isAuth1Importing.value = false
    auth1Progress.value = ''
  }
}
const loadCache = () => {
  try {
    const raw = localStorage.getItem(CACHE_KEY)
    if (!raw) return []
    const parsed = JSON.parse(raw)
    if (!Array.isArray(parsed)) return []
    return parsed
  } catch {
    return []
  }
}

const inferAuthProvider = (item = {}) => {
  if (item.auth_provider) return item.auth_provider

  const authToken = typeof item.auth_token === 'string' ? item.auth_token : ''
  if (
    authToken.startsWith('auth1_') ||
    authToken.startsWith('devin-session-token$') ||
    item.session_token ||
    item.devin_account_id ||
    item.devin_primary_org_id
  ) {
    return 'auth1'
  }

  return null
}

const pickCreditFields = (item = {}) => ({
  plan_name: item.plan_name || null,
  used_credits: item.used_credits ?? null,
  total_credits: item.total_credits ?? null,
  used_prompt_credits: item.used_prompt_credits ?? null,
  available_prompt_credits: item.available_prompt_credits ?? null,
  used_flow_credits: item.used_flow_credits ?? null,
  available_flow_credits: item.available_flow_credits ?? null,
  used_flex_credits: item.used_flex_credits ?? null,
  available_flex_credits: item.available_flex_credits ?? null,
  monthly_prompt_credits: item.monthly_prompt_credits ?? null,
  monthly_flow_credits: item.monthly_flow_credits ?? null,
  daily_quota_remaining_percent: item.daily_quota_remaining_percent ?? null,
  weekly_quota_remaining_percent: item.weekly_quota_remaining_percent ?? null,
  daily_quota_reset_at_unix: item.daily_quota_reset_at_unix ?? null,
  weekly_quota_reset_at_unix: item.weekly_quota_reset_at_unix ?? null,
  expires_at: item.expires_at || null,
  plan_start: item.plan_start || null,
  plan_start_unix: item.plan_start_unix ?? null,
  plan_end_unix: item.plan_end_unix ?? null,
  credits_updated_at: item.credits_updated_at || null
})

const normalizeLocalItem = (item) => {
  const assignedUser = item.user_name || item.created_by_name || null
  return {
    ...item,
    __loggingIn: false,
    __refreshingCredits: false,
    __updating: false,
    user_name: assignedUser,
    ...pickCreditFields(item),
    devin_account_id: item.devin_account_id || null,
    devin_primary_org_id: item.devin_primary_org_id || null,
    session_token: item.session_token || null,
    auth_provider: inferAuthProvider(item)
  }
}

const handleRefreshCredits = async (account, opts = {}) => {
  const { silentSuccess = false, silentError = false } = opts || {}
  if (account.__refreshingCredits) return false
  account.__refreshingCredits = true

  try {
    if (!account.auth_token) {
      if (!silentError) showToast(t('windsurf.noToken'), 'warning')
      return false
    }

    const result = await invoke('windsurf_refresh_credits', {
      authToken: account.auth_token,
      auth_token: account.auth_token,
      devinAccountId: account.devin_account_id || null,
      devin_account_id: account.devin_account_id || null,
      devinAuth1Token: typeof account.auth_token === 'string' && account.auth_token.startsWith('auth1_') ? account.auth_token : null,
      devin_auth1_token: typeof account.auth_token === 'string' && account.auth_token.startsWith('auth1_') ? account.auth_token : null,
      devinPrimaryOrgId: account.devin_primary_org_id || null,
      devin_primary_org_id: account.devin_primary_org_id || null,
      sessionToken: account.session_token || null,
      session_token: account.session_token || null
    })
    if (!result || !result.success) {
      if (!silentError) showToast(`${t('windsurf.refreshCreditsFailed')}: ${result?.error || ''}`.trim(), 'error')
      return false
    }

    account.plan_name = result.plan_name
    account.teams_tier = result.teams_tier
    account.teams_tier_name = result.teams_tier_name
    account.used_credits = result.used_credits
    account.total_credits = result.total_credits
    account.used_prompt_credits = result.used_prompt_credits
    account.available_prompt_credits = result.available_prompt_credits
    account.used_flow_credits = result.used_flow_credits
    account.available_flow_credits = result.available_flow_credits
    account.used_flex_credits = result.used_flex_credits
    account.available_flex_credits = result.available_flex_credits
    account.monthly_prompt_credits = result.monthly_prompt_credits
    account.monthly_flow_credits = result.monthly_flow_credits
    account.daily_quota_remaining_percent = result.daily_quota_remaining_percent
    account.weekly_quota_remaining_percent = result.weekly_quota_remaining_percent
    account.daily_quota_reset_at_unix = result.daily_quota_reset_at_unix
    account.weekly_quota_reset_at_unix = result.weekly_quota_reset_at_unix
    account.expires_at = result.expires_at
    account.plan_start = result.plan_start
    account.plan_start_unix = result.plan_start_unix
    account.plan_end_unix = result.plan_end_unix
    account.credits_updated_at = new Date().toISOString()

    saveCache()

    if (!silentSuccess) showToast(t('windsurf.refreshCreditsSuccess'), 'success')
    return true
  } catch (e) {
    if (!silentError) showToast(`${t('windsurf.refreshCreditsFailed')}: ${e.message || e}`, 'error')
    return false
  } finally {
    account.__refreshingCredits = false
  }
}

const handleRefreshAccount = async (account) => {
  if (account.__updating) return
  account.__updating = true
  try {
    if (!account.auth_token) {
      showToast(t('windsurf.noToken'), 'warning')
      return
    }
    await handleRefreshCredits(account, { silentSuccess: true })
    if (account.total_credits !== undefined && account.total_credits !== null) {
      saveCache()
      showToast(t('windsurf.updateSuccess'), 'success')
    }
  } finally {
    account.__updating = false
  }
}

const handleDeleteLocal = (account) => {
  deleteTargetAccount.value = account
  showDeleteConfirm.value = true
}

const handleShowAnalytics = (account) => {
  if (!account) return
  if (!account.auth_token) {
    showToast(t('windsurf.noToken'), 'warning')
    return
  }
  analyticsAccount.value = account
  showAnalyticsDialog.value = true
}

const closeAnalyticsDialog = () => {
  showAnalyticsDialog.value = false
  analyticsAccount.value = null
}

const confirmDelete = async () => {
  const account = deleteTargetAccount.value
  if (!account) return

  isDeleting.value = true
  try {
    const keyEmail = (account?.email || '').toLowerCase().trim()
    accounts.value = accounts.value.filter(a => (a.email || '').toLowerCase().trim() !== keyEmail)
    saveCache()
    showToast(t('windsurf.deleteLocalSuccess'), 'success')
    showDeleteConfirm.value = false
    deleteTargetAccount.value = null
  } catch (e) {
    showToast(`${t('windsurf.deleteFailed')}: ${e.message || e}`, 'error')
  } finally {
    isDeleting.value = false
  }
}

const handleSwitchAccount = async (account, options = {}) => {
  if (account.__loggingIn) return false
  account.__loggingIn = true
  try {
    if (!account.auth_token) {
      showToast(t('windsurf.missingCredentials'), 'error')
      return false
    }

    const windsurfTarget = buildInvokeTarget()
    const resp = await invoke('windsurf_switch_account', {
      authToken: account.auth_token,
      auth_token: account.auth_token,
      refreshToken: null,
      refresh_token: null,
      devinAccountId: account.devin_account_id || null,
      devin_account_id: account.devin_account_id || null,
      devinPrimaryOrgId: account.devin_primary_org_id || null,
      devin_primary_org_id: account.devin_primary_org_id || null,
      windsurfTarget,
      windsurf_target: windsurfTarget
    })

    if (!resp || !resp.success) {
      showToast(`${t('windsurf.switchFailed')}: ${resp?.error || ''}`.trim(), 'error')
      return false
    }

    // 回写：Auth1 路径会把新换的 session_token 和 devin_* 返回
    let mutated = false
    if (resp.auth_token) {
      account.auth_token = resp.auth_token
      mutated = true
    }
    if (resp.session_token) {
      account.session_token = resp.session_token
      mutated = true
    }
    if (resp.devin_account_id) {
      account.devin_account_id = resp.devin_account_id
      mutated = true
    }
    if (resp.devin_primary_org_id) {
      account.devin_primary_org_id = resp.devin_primary_org_id
      mutated = true
    }
    if (resp.provider && !account.auth_provider) {
      account.auth_provider = resp.provider
      mutated = true
    }
    if (mutated) saveCache()

    if (resp.machine_id_reset === false) {
      const reason = resp.machine_id_reset_error ? `: ${resp.machine_id_reset_error}` : ''
      showToast(`${t('windsurf.switchSuccessNoMachineReset')}${reason}`, 'warning')
    } else {
      showToast(t('windsurf.switchSuccess'), 'success')
    }
    saveLastSwitchEmail(account.email)
    if (!options.autoTriggered) {
      await revealCurrentLoginAccount(account.email)
    }
    if (autoSwitchConfig.value.enabled && !options.autoTriggered) {
      enterAutoSwitchCooldown(autoSwitchConfig.value.cooldownSeconds, t('windsurf.autoSwitchManualCooldown'))
      scheduleAutoSwitch(Math.min(AUTO_SWITCH_COOLDOWN_FRONTMOST_PROBE_SECONDS, autoSwitchConfig.value.cooldownSeconds), { lightProbe: true })
    }
    return true
  } catch (e) {
    showToast(`${t('windsurf.switchFailed')}: ${e.message || e}`, 'error')
    return false
  } finally {
    account.__loggingIn = false
  }
}

const handleRefreshAll = async () => {
  if (accounts.value.length === 0 || isRefreshing.value) return
  isRefreshing.value = true

  let successCount = 0
  let failCount = 0

  const BATCH_SIZE = 100

  const refreshOne = async (acc) => {
    try {
      if (acc.auth_token) {
        return await handleRefreshCredits(acc, { silentSuccess: true, silentError: true })
      }
      return false
    } catch {
      return false
    }
  }

  try {
    const allAccounts = [...accounts.value]
    for (let i = 0; i < allAccounts.length; i += BATCH_SIZE) {
      const batch = allAccounts.slice(i, i + BATCH_SIZE)
      const results = await Promise.all(batch.map(refreshOne))
      results.forEach(ok => ok ? successCount++ : failCount++)
    }

    if (failCount === 0) {
      showToast(t('windsurf.refreshAllSuccess', { count: successCount }), 'success')
    } else {
      showToast(t('windsurf.refreshAllPartial', { success: successCount, fail: failCount }), 'warning')
    }
  } finally {
    isRefreshing.value = false
    saveCache()
  }
}

const handleClickOutside = (e) => {
  if (showSortMenu.value) {
    showSortMenu.value = false
  }
}

// 弹窗 Esc 关闭：栈式处理，最上层先关
useEscToClose(() => showAuth1Dialog.value, () => {
  if (!isAuth1Importing.value) showAuth1Dialog.value = false
})
useEscToClose(() => showOrgPickerDialog.value, () => {
  if (!isOrgPickerConfirming.value) cancelOrgPicker()
})
useEscToClose(() => showDeleteConfirm.value, () => {
  if (!isDeleting.value) showDeleteConfirm.value = false
})
useEscToClose(() => showBatchDeleteDialog.value, () => {
  if (!isBatchDeleting.value) closeBatchDeleteDialog()
})
useEscToClose(() => showAutoSwitchDialog.value, () => {
  if (!autoSwitchRuntime.value.busy) closeAutoSwitchDialog()
})

onMounted(() => {
  isLoading.value = true
  const cached = loadCache().map(normalizeLocalItem)
  accounts.value = cached
  isLoading.value = false
  document.addEventListener('click', handleClickOutside)
  setupAutoSwitchEventListener()
  applySeamlessPatchIfNeeded(autoSwitchConfig.value, { silent: true })
  startAutoSwitchService({ immediate: true })
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
  clearAutoSwitchTimer()
  if (autoSwitchSnapshotSyncTimer) clearTimeout(autoSwitchSnapshotSyncTimer)
  if (typeof unlistenAutoSwitchEvent === 'function') unlistenAutoSwitchEvent()
  if (currentLoginHighlightTimer) clearTimeout(currentLoginHighlightTimer)
})
</script>

<style scoped>
.embedded-container {
  width: 100%;
  min-width: 0;
  height: 100%;
  min-height: 0;
  flex: 1;
}

.embedded-inner {
  width: 100%;
  min-width: 0;
  height: 100%;
  min-height: 0;
}

.embedded-panel {
  width: 100%;
  min-width: 0;
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;
  background: var(--color-surface, #ffffff);
}

.windsurf-account-list-modal {
  position: fixed;
  inset: 0;
  z-index: 2100;
}

.modal-overlay {
  position: absolute;
  inset: 0;
  background: rgba(15, 23, 42, 0.45);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
}

.modal-content {
  background: var(--card-bg, #fff);
  border-radius: 14px;
  width: min(1200px, 96vw);
  height: min(86vh, 900px);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  box-shadow: 0 20px 40px rgba(0, 0, 0, 0.22);
}

.modal-header {
  padding: 16px 18px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.06);
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 12px;
}

.header-title {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 12px;
  min-width: 0;
}

.header-title h2 {
  margin: 0;
  font-size: 18px;
}

.status-badge {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  border-radius: 999px;
  background: rgba(59, 130, 246, 0.12);
  color: #2563eb;
  font-weight: 600;
  font-size: 12px;
}

.status-dot {
  width: 6px;
  height: 6px;
  border-radius: 999px;
  background: currentColor;
}

.header-actions {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 10px;
}

.close-btn {
  border: none;
  background: transparent;
  font-size: 22px;
  cursor: pointer;
  opacity: 0.7;
  width: 32px;
  height: 32px;
  border-radius: 10px;
}

.close-btn:hover {
  background: rgba(0, 0, 0, 0.05);
  opacity: 1;
}

.modal-body {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  padding: 12px 18px 16px;
}

.auto-switch-banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 12px;
  margin-bottom: 12px;
  border-radius: 12px;
  border: 1px solid rgba(148, 163, 184, 0.24);
  background: rgba(148, 163, 184, 0.08);
  font-size: 12px;
  color: rgba(15, 23, 42, 0.72);
}

.auto-switch-banner.enabled {
  border-color: rgba(16, 185, 129, 0.25);
  background: rgba(16, 185, 129, 0.08);
}

.auto-switch-banner.busy .auto-switch-dot {
  animation: pulseDot 1s ease-in-out infinite;
}

.auto-switch-main {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 8px;
}

.auto-switch-dot {
  width: 8px;
  height: 8px;
  border-radius: 999px;
  flex: 0 0 auto;
  background: rgba(148, 163, 184, 0.8);
}

.auto-switch-banner.enabled .auto-switch-dot {
  background: #10b981;
}

.loading-state,
.empty-state {
  height: 100%;
  min-height: 360px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
}

.spinner {
  width: 28px;
  height: 28px;
  border: 3px solid rgba(0, 0, 0, 0.12);
  border-top-color: rgba(59, 130, 246, 0.9);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

.empty-icon {
  opacity: 0.35;
}

.empty-actions {
  display: flex;
  align-items: center;
  justify-content: center;
  flex-wrap: wrap;
  gap: 14px;
  margin-top: 4px;
}

.account-list {
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  flex: 1;
  gap: 12px;
}

.list-toolbar {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px;
  flex-shrink: 0;
}

.account-grid-scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  padding-right: 4px;
  scrollbar-width: thin;
  scrollbar-color: rgba(100, 116, 139, 0.35) transparent;
}

.account-grid-scroll::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}

.account-grid-scroll::-webkit-scrollbar-track {
  background: transparent;
}

.account-grid-scroll::-webkit-scrollbar-thumb {
  background: rgba(100, 116, 139, 0.3);
  border-radius: 999px;
}

.account-grid-scroll::-webkit-scrollbar-thumb:hover {
  background: rgba(100, 116, 139, 0.55);
}

[data-theme='dark'] .account-grid-scroll::-webkit-scrollbar-thumb {
  background: rgba(148, 163, 184, 0.35);
}

[data-theme='dark'] .account-grid-scroll::-webkit-scrollbar-thumb:hover {
  background: rgba(148, 163, 184, 0.6);
}

.search-box {
  flex: 0 1 260px;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 6px;
  background: rgba(0, 0, 0, 0.04);
  border-radius: 10px;
  padding: 4px 10px;
  height: 30px;
}

.search-icon {
  opacity: 0.55;
}

.search-input {
  flex: 1;
  min-width: 0;
  border: none;
  outline: none;
  background: transparent;
  font-size: 13px;
  line-height: 1.2;
}

.clear-search-btn {
  border: none;
  background: transparent;
  cursor: pointer;
  opacity: 0.6;
}

.clear-search-btn:hover {
  opacity: 1;
}

.account-count {
  margin-left: auto;
  font-size: 12px;
  opacity: 0.6;
  font-weight: 600;
}

.sort-wrapper {
  position: relative;
}

.sort-btn {
  background: rgba(0, 0, 0, 0.06);
  color: inherit;
}

.sort-btn:hover {
  background: rgba(0, 0, 0, 0.1);
}

.sort-menu {
  position: absolute;
  top: calc(100% + 6px);
  right: 0;
  background: var(--card-bg, #fff);
  border-radius: 10px;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.15);
  min-width: 200px;
  z-index: 100;
  padding: 6px;
  border: 1px solid rgba(0, 0, 0, 0.08);
}

.sort-option {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  padding: 10px 12px;
  border: none;
  background: transparent;
  cursor: pointer;
  font-size: 13px;
  border-radius: 6px;
  transition: background 0.15s;
  text-align: left;
}

.sort-option:hover {
  background: rgba(0, 0, 0, 0.05);
}

.sort-option.active {
  background: rgba(59, 130, 246, 0.1);
  color: #2563eb;
}

.sort-divider {
  height: 1px;
  background: rgba(0, 0, 0, 0.08);
  margin: 4px 0;
}

.check-icon {
  color: #2563eb;
}

.dropdown-enter-active,
.dropdown-leave-active {
  transition: opacity 0.15s, transform 0.15s;
}

.dropdown-enter-from,
.dropdown-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}

.account-grid {
  display: grid;
  gap: 14px;
  justify-content: start;
}

@media (max-width: 960px) {
  .modal-header {
    align-items: stretch;
  }

  .header-actions {
    width: 100%;
    justify-content: flex-start;
  }

  .search-box {
    flex-basis: 100%;
  }
}

.btn {
  display: inline-flex;
  align-items: center;
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

.btn.small {
  padding: 6px 12px;
  font-size: 13px;
}

.btn.success {
  background: var(--success-color, #10b981);
  color: white;
}

.btn.success:hover {
  background: var(--success-hover, #059669);
}

.btn.info {
  background: var(--info-color, #06b6d4);
  color: white;
}

.btn.info:hover {
  background: var(--info-hover, #0891b2);
}

.btn.danger {
  background: var(--danger-color, #ef4444);
  color: white;
}

.btn.danger:hover {
  background: var(--danger-hover, #dc2626);
}

.btn.primary {
  background: var(--primary-color, #3b82f6);
  color: white;
}

.btn.primary:hover {
  background: var(--primary-hover, #2563eb);
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
}

.sync-spinner {
  display: inline-block;
  width: 14px;
  height: 14px;
  flex-shrink: 0;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: white;
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

.toast {
  position: fixed;
  bottom: 20px;
  right: 20px;
  z-index: 4000;
  padding: 12px 14px;
  border-radius: 12px;
  color: #fff;
  box-shadow: 0 12px 24px rgba(0, 0, 0, 0.18);
  max-width: 360px;
}

.toast.success {
  background: rgba(16, 185, 129, 0.95);
}

.toast.error {
  background: rgba(239, 68, 68, 0.95);
}

.toast.warning {
  background: rgba(245, 158, 11, 0.95);
}

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
  background: var(--card-bg, #fff);
  border-radius: 14px;
  overflow: hidden;
  box-shadow: 0 20px 40px rgba(0, 0, 0, 0.22);
}

.dialog-header {
  padding: 14px 16px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.06);
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.dialog-header h3 {
  margin: 0;
  font-size: 16px;
}

.dialog-close {
  border: none;
  background: transparent;
  cursor: pointer;
  font-size: 22px;
  line-height: 1;
  opacity: 0.7;
  width: 32px;
  height: 32px;
  border-radius: 10px;
}

.dialog-close:hover {
  background: rgba(0, 0, 0, 0.05);
  opacity: 1;
}

.dialog-body {
  padding: 14px 16px;
}

.dialog-message {
  margin: 0 0 10px 0;
  font-size: 13px;
  opacity: 0.75;
  line-height: 1.5;
}

.import-textarea {
  width: 100%;
  border: 1px solid rgba(0, 0, 0, 0.12);
  border-radius: 10px;
  padding: 10px 12px;
  font-size: 12px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
  outline: none;
  resize: vertical;
  min-height: 160px;
}

.dialog-footer {
  padding: 14px 16px;
  border-top: 1px solid rgba(0, 0, 0, 0.06);
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}

.btn-cancel {
  padding: 8px 16px;
  border-radius: 10px;
  border: 1px solid rgba(0, 0, 0, 0.12);
  background: transparent;
  cursor: pointer;
}

.btn-confirm {
  padding: 8px 16px;
  border-radius: 10px;
  border: none;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  background: var(--primary-color, #3b82f6);
  color: #fff;
  cursor: pointer;
}

.btn-confirm:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

@keyframes pulseDot {
  0%, 100% {
    opacity: 0.45;
    transform: scale(0.9);
  }
  50% {
    opacity: 1;
    transform: scale(1.25);
  }
}

.batch-delete-dialog {
  background: var(--card-bg, #fff);
  border-radius: 14px;
  width: min(480px, 90vw);
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  box-shadow: 0 20px 40px rgba(0, 0, 0, 0.18);
}

.auto-switch-dialog {
  background: var(--card-bg, #fff);
  border-radius: 14px;
  width: min(640px, 92vw);
  max-height: calc(100vh - 40px);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  box-shadow: 0 20px 40px rgba(0, 0, 0, 0.18);
}

.auto-switch-dialog .dialog-header,
.auto-switch-dialog .dialog-footer {
  flex-shrink: 0;
}

.auto-switch-dialog .dialog-body {
  flex: 1 1 auto;
  min-height: 0;
  overflow-y: auto;
}

.auto-switch-form {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.auto-switch-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(190px, 1fr));
  gap: 10px;
}

.auto-switch-grid label {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 10px 12px;
  border-radius: 10px;
  background: rgba(0, 0, 0, 0.03);
  font-size: 12px;
  font-weight: 600;
}

.auto-switch-grid input {
  border: 1px solid rgba(0, 0, 0, 0.12);
  border-radius: 8px;
  padding: 7px 9px;
  background: rgba(255, 255, 255, 0.88);
}

.auto-switch-advanced-toggle {
  align-self: flex-start;
  border: none;
  background: transparent;
  color: var(--primary-color, #3b82f6);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  padding: 4px 0;
}

.auto-switch-advanced-toggle:hover {
  text-decoration: underline;
}

.auto-switch-status-text {
  margin-top: 12px;
  padding: 10px 12px;
  border-radius: 10px;
  background: rgba(59, 130, 246, 0.08);
}

.delete-mode-tabs {
  display: flex;
  gap: 8px;
  margin-bottom: 16px;
}

.mode-tab {
  flex: 1;
  padding: 10px;
  border: 1px solid rgba(0, 0, 0, 0.1);
  background: transparent;
  border-radius: 8px;
  cursor: pointer;
  font-size: 13px;
  transition: all 0.2s;
}

.mode-tab.active {
  background: var(--primary-color, #3b82f6);
  color: white;
  border-color: transparent;
}

.delete-conditions {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.condition-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  background: rgba(0, 0, 0, 0.03);
  border-radius: 8px;
  cursor: pointer;
}

.condition-item:hover {
  background: rgba(0, 0, 0, 0.06);
}

.condition-count {
  margin-left: auto;
  color: #666;
  font-size: 12px;
}

.manual-select {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.select-actions {
  display: flex;
  gap: 8px;
}

.select-btn {
  padding: 6px 12px;
  border: 1px solid rgba(0, 0, 0, 0.1);
  background: transparent;
  border-radius: 6px;
  cursor: pointer;
  font-size: 12px;
}

.select-btn:hover {
  background: rgba(0, 0, 0, 0.05);
}

.locate-account-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  border-color: rgba(16, 185, 129, 0.32);
  background:
    linear-gradient(
      180deg,
      rgba(16, 185, 129, 0.16) 0%,
      rgba(16, 185, 129, 0.08) 100%
    );
  color: #047857;
  font-weight: 700;
}

.locate-account-btn:hover:not(:disabled) {
  border-color: rgba(16, 185, 129, 0.56);
  background:
    linear-gradient(
      180deg,
      rgba(16, 185, 129, 0.22) 0%,
      rgba(16, 185, 129, 0.12) 100%
    );
  color: #065f46;
}

.locate-account-btn svg {
  flex: 0 0 auto;
}

.manual-list {
  max-height: 200px;
  overflow-y: auto;
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 8px;
}

.manual-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-bottom: 1px solid rgba(0, 0, 0, 0.05);
  cursor: pointer;
}

.manual-item:last-child {
  border-bottom: none;
}

.manual-item:hover {
  background: rgba(0, 0, 0, 0.03);
}

.account-email {
  flex: 1;
  font-size: 13px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tag {
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 10px;
  font-weight: 600;
}

.tag.no-credits {
  background: #ef4444;
  color: white;
}

.tag.expired {
  background: #ef4444;
  color: white;
}

.tag.free {
  background: #f59e0b;
  color: white;
}

.batch-delete-icon-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border: none;
  background: transparent;
  color: #ef4444;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
}

.batch-delete-icon-btn:hover {
  background: rgba(239, 68, 68, 0.1);
}

.batch-delete-icon-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.delete-stats {
  margin-top: 12px;
  padding: 10px 12px;
  background: rgba(0, 0, 0, 0.03);
  border-radius: 8px;
}

.stat-item {
  display: flex;
  justify-content: space-between;
  font-size: 13px;
}

.stat-label {
  color: #666;
}

.stat-value {
  font-weight: 600;
}

.dialog-warning {
  margin-top: 10px;
  font-size: 12px;
  color: #ef4444;
}

.org-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  max-height: 280px;
  overflow: auto;
}

.org-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 10px;
  background: rgba(0, 0, 0, 0.02);
  cursor: pointer;
  transition: border-color 0.15s, background 0.15s;
}

.org-item:hover {
  border-color: rgba(59, 130, 246, 0.4);
  background: rgba(59, 130, 246, 0.04);
}

.org-item input[type="radio"] {
  margin: 0;
}

.org-item .org-name {
  flex: 1;
  font-weight: 600;
  font-size: 13px;
  color: rgba(0, 0, 0, 0.78);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.org-item .org-id {
  font-size: 11px;
  color: rgba(0, 0, 0, 0.45);
  font-family: ui-monospace, Menlo, monospace;
  max-width: 160px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.auth1-error-list {
  margin-top: 12px;
  padding: 10px 12px;
  border-radius: 10px;
  background: rgba(239, 68, 68, 0.06);
  border: 1px solid rgba(239, 68, 68, 0.18);
  max-height: 220px;
  overflow: auto;
}

.auth1-error-title {
  font-size: 12px;
  font-weight: 700;
  color: #b91c1c;
  margin-bottom: 6px;
}

.auth1-error-item {
  padding: 6px 0;
  border-top: 1px dashed rgba(239, 68, 68, 0.2);
}
.auth1-error-item:first-of-type {
  border-top: none;
}

.auth1-error-label {
  font-size: 12px;
  font-weight: 600;
  color: rgba(0, 0, 0, 0.8);
  margin-bottom: 2px;
  display: flex;
  gap: 8px;
  align-items: baseline;
}

.auth1-error-token {
  font-family: ui-monospace, Menlo, monospace;
  font-weight: 400;
  font-size: 11px;
  color: rgba(0, 0, 0, 0.45);
}

.auth1-error-reason {
  font-size: 12px;
  color: #b91c1c;
  white-space: pre-wrap;
  word-break: break-all;
}

/* ============================== */
/*  暗色主题统一覆盖：           */
/*  避免借用的 --card-bg / --primary-color 等未定义变量导致白底   */
/* ============================== */

[data-theme='dark'] .modal-content,
[data-theme='dark'] .import-dialog,
[data-theme='dark'] .batch-delete-dialog,
[data-theme='dark'] .auto-switch-dialog,
[data-theme='dark'] .embedded-panel {
  background: var(--color-surface, #1e293b);
  color: var(--color-text-primary, #e2e8f0);
  border-color: var(--color-border, rgba(148, 163, 184, 0.25));
  box-shadow: var(--color-shadow-modal, 0 20px 40px rgba(0, 0, 0, 0.55));
}

[data-theme='dark'] .modal-header,
[data-theme='dark'] .dialog-header,
[data-theme='dark'] .dialog-footer {
  border-color: rgba(148, 163, 184, 0.2);
}

[data-theme='dark'] .close-btn,
[data-theme='dark'] .dialog-close {
  color: var(--color-text-secondary, #cbd5f5);
}

[data-theme='dark'] .close-btn:hover,
[data-theme='dark'] .dialog-close:hover {
  background: rgba(255, 255, 255, 0.08);
}

[data-theme='dark'] .spinner {
  border-color: rgba(148, 163, 184, 0.25);
  border-top-color: var(--color-btn-primary-bg, #60a5fa);
}

[data-theme='dark'] .sort-btn {
  background: rgba(148, 163, 184, 0.16);
  color: var(--color-text-primary, #e2e8f0);
}

[data-theme='dark'] .sort-btn:hover {
  background: rgba(148, 163, 184, 0.24);
}

[data-theme='dark'] .sort-menu {
  background: var(--color-surface, #1e293b);
  border-color: rgba(148, 163, 184, 0.24);
  box-shadow: 0 18px 38px rgba(0, 0, 0, 0.55);
  color: var(--color-text-primary, #e2e8f0);
}

[data-theme='dark'] .sort-option:hover {
  background: rgba(148, 163, 184, 0.14);
}

[data-theme='dark'] .sort-option.active {
  background: rgba(96, 165, 250, 0.18);
  color: #93c5fd;
}

[data-theme='dark'] .sort-divider {
  background: rgba(148, 163, 184, 0.2);
}

[data-theme='dark'] .check-icon {
  color: #93c5fd;
}

[data-theme='dark'] .search-input {
  color: var(--color-text-primary, #e2e8f0);
  background: transparent;
}

[data-theme='dark'] .search-input::placeholder {
  color: var(--color-input-placeholder, #64748b);
}

[data-theme='dark'] .import-textarea {
  background: var(--color-input-bg, rgba(15, 23, 42, 0.85));
  border-color: var(--color-input-border, rgba(148, 163, 184, 0.35));
  color: var(--color-text-primary, #e2e8f0);
}

[data-theme='dark'] .import-textarea::placeholder {
  color: var(--color-input-placeholder, #64748b);
}

[data-theme='dark'] .btn-cancel {
  border-color: rgba(148, 163, 184, 0.35);
  color: var(--color-text-primary, #e2e8f0);
}

[data-theme='dark'] .btn-cancel:hover {
  background: rgba(148, 163, 184, 0.12);
}

[data-theme='dark'] .btn-confirm {
  background: var(--color-btn-primary-bg, #6366f1);
  color: var(--color-btn-primary-text, #f9fafb);
}

[data-theme='dark'] .btn-confirm:hover:not(:disabled) {
  background: var(--color-btn-primary-bg-hover, #818cf8);
}

[data-theme='dark'] .dialog-message {
  color: var(--color-text-secondary, #94a3b8);
}

[data-theme='dark'] .auto-switch-banner {
  border-color: rgba(148, 163, 184, 0.24);
  background: rgba(148, 163, 184, 0.1);
  color: var(--color-text-secondary, #cbd5f5);
}

[data-theme='dark'] .auto-switch-banner.enabled {
  border-color: rgba(45, 212, 191, 0.28);
  background: rgba(45, 212, 191, 0.1);
}

[data-theme='dark'] .auto-switch-grid label {
  background: rgba(148, 163, 184, 0.1);
  color: var(--color-text-primary, #e2e8f0);
}

[data-theme='dark'] .auto-switch-grid input {
  background: var(--color-input-bg, rgba(15, 23, 42, 0.85));
  border-color: var(--color-input-border, rgba(148, 163, 184, 0.35));
  color: var(--color-text-primary, #e2e8f0);
}

[data-theme='dark'] .mode-tab {
  border-color: rgba(148, 163, 184, 0.28);
  color: var(--color-text-secondary, #cbd5f5);
}

[data-theme='dark'] .mode-tab:hover:not(.active) {
  background: rgba(148, 163, 184, 0.12);
}

[data-theme='dark'] .mode-tab.active {
  background: var(--color-btn-primary-bg, #6366f1);
  color: var(--color-btn-primary-text, #f9fafb);
}

[data-theme='dark'] .condition-item,
[data-theme='dark'] .delete-stats {
  background: rgba(148, 163, 184, 0.1);
  color: var(--color-text-primary, #e2e8f0);
}

[data-theme='dark'] .condition-item:hover {
  background: rgba(148, 163, 184, 0.16);
}

[data-theme='dark'] .condition-count,
[data-theme='dark'] .stat-label {
  color: var(--color-text-muted, #94a3b8);
}

[data-theme='dark'] .select-btn {
  border-color: rgba(148, 163, 184, 0.28);
  color: var(--color-text-primary, #e2e8f0);
}

[data-theme='dark'] .select-btn:hover {
  background: rgba(148, 163, 184, 0.14);
}

[data-theme='dark'] .locate-account-btn {
  border-color: rgba(45, 212, 191, 0.34);
  background:
    linear-gradient(
      180deg,
      rgba(45, 212, 191, 0.18) 0%,
      rgba(45, 212, 191, 0.1) 100%
    );
  color: #5eead4;
}

[data-theme='dark'] .locate-account-btn:hover:not(:disabled) {
  border-color: rgba(45, 212, 191, 0.58);
  background:
    linear-gradient(
      180deg,
      rgba(45, 212, 191, 0.24) 0%,
      rgba(45, 212, 191, 0.14) 100%
    );
  color: #99f6e4;
}

[data-theme='dark'] .manual-list {
  border-color: rgba(148, 163, 184, 0.22);
  background: rgba(15, 23, 42, 0.35);
}

[data-theme='dark'] .manual-item {
  border-bottom-color: rgba(148, 163, 184, 0.14);
  color: var(--color-text-primary, #e2e8f0);
}

[data-theme='dark'] .manual-item:hover {
  background: rgba(148, 163, 184, 0.1);
}

[data-theme='dark'] .org-item {
  border-color: rgba(148, 163, 184, 0.22);
  background: rgba(148, 163, 184, 0.08);
}

[data-theme='dark'] .org-item:hover {
  border-color: rgba(96, 165, 250, 0.5);
  background: rgba(96, 165, 250, 0.12);
}

[data-theme='dark'] .org-item .org-name {
  color: var(--color-text-primary, #e2e8f0);
}

[data-theme='dark'] .org-item .org-id {
  color: var(--color-text-muted, #94a3b8);
}

[data-theme='dark'] .auth1-error-list {
  background: rgba(248, 113, 113, 0.12);
  border-color: rgba(248, 113, 113, 0.35);
}

[data-theme='dark'] .auth1-error-title,
[data-theme='dark'] .auth1-error-reason {
  color: #fca5a5;
}

[data-theme='dark'] .auth1-error-label {
  color: var(--color-text-primary, #e2e8f0);
}

[data-theme='dark'] .auth1-error-token {
  color: var(--color-text-muted, #94a3b8);
}

[data-theme='dark'] .auth1-error-item {
  border-top-color: rgba(248, 113, 113, 0.28);
}

[data-theme='dark'] .modal-overlay,
[data-theme='dark'] .import-overlay {
  background: rgba(0, 0, 0, 0.68);
}
</style>
