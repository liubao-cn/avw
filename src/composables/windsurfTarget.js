/**
 * Windsurf 客户端目标设置（客户端类型 / 安装路径 / 用户数据目录）前端存储层。
 *
 * 设计要点：
 * - 纯前端持久化（localStorage，key: `avw.windsurf.target.v1`），后端保持无状态；
 * - 每次调用需要定位 Windsurf 的 Tauri 命令时，用 {@link buildInvokeTarget}
 *   组装 `{ windsurfTarget: { clientType, installPath?, userDataDir? } }` 参数传入；
 * - 跨组件共享同一个响应式引用，设置面板保存后其他调用点立即感知。
 */

import { ref } from 'vue'

const STORAGE_KEY = 'avw.windsurf.target.v1'

const DEFAULT_TARGET = Object.freeze({
  clientType: 'windsurf',
  installPath: '',
  userDataDir: '',
})

const VALID_CLIENT_TYPES = new Set(['windsurf', 'windsurf-next'])

const state = ref({ ...DEFAULT_TARGET })
let hydrated = false

function normalize(raw) {
  if (!raw || typeof raw !== 'object') return { ...DEFAULT_TARGET }
  const clientType = VALID_CLIENT_TYPES.has(raw.clientType)
    ? raw.clientType
    : DEFAULT_TARGET.clientType
  return {
    clientType,
    installPath: typeof raw.installPath === 'string' ? raw.installPath : '',
    userDataDir: typeof raw.userDataDir === 'string' ? raw.userDataDir : '',
  }
}

function hydrate() {
  if (hydrated) return
  hydrated = true
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (raw) {
      state.value = normalize(JSON.parse(raw))
    }
  } catch (err) {
    console.warn('[windsurfTarget] failed to hydrate', err)
  }
}

/**
 * 返回一个共享的响应式 target 引用，组件里可以直接在 template 里用。
 */
export function useWindsurfTarget() {
  hydrate()
  return state
}

/**
 * 一次性读取当前 target 的快照（非响应式）。
 */
export function loadWindsurfTarget() {
  hydrate()
  return { ...state.value }
}

/**
 * 持久化新的 target 值；会先规范化，非法 clientType 回落到默认。
 */
export function saveWindsurfTarget(next) {
  const normalized = normalize(next)
  state.value = normalized
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(normalized))
  } catch (err) {
    console.warn('[windsurfTarget] failed to persist', err)
  }
}

/**
 * 恢复默认值（标准版 Windsurf + 空安装路径 + 空用户数据目录）。
 */
export function resetWindsurfTarget() {
  saveWindsurfTarget({ ...DEFAULT_TARGET })
}

/**
 * 组装给 Tauri 命令使用的 payload。
 *
 * 规则：
 * - `clientType` 总是携带；
 * - `installPath` / `userDataDir` 仅在非空时携带，保持后端行为与"未自定义"等价。
 */
export function buildInvokeTarget(source) {
  const t = source ?? state.value
  const payload = {
    clientType: VALID_CLIENT_TYPES.has(t?.clientType) ? t.clientType : 'windsurf',
  }
  const install = (t?.installPath ?? '').trim()
  if (install) payload.installPath = install
  const dataDir = (t?.userDataDir ?? '').trim()
  if (dataDir) payload.userDataDir = dataDir
  return payload
}

export { DEFAULT_TARGET, VALID_CLIENT_TYPES, STORAGE_KEY }
