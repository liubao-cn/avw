import { onBeforeUnmount, watch } from 'vue'

/**
 * Esc 键关闭弹窗的公共 composable。
 *
 * 使用栈管理：当多个弹窗同时打开时，Esc 只关闭最上层的那个，
 * 避免一次按键一次性关掉所有层级。
 *
 * @param {import('vue').Ref<boolean>|(() => boolean)} isOpen 弹窗是否可见（可以是 ref 或 getter）
 * @param {() => void} onClose 按下 Esc 时要调用的关闭函数
 */
const stack = []
let listenerBound = false

const handleKeydown = (event) => {
  if (event.key !== 'Escape' || event.defaultPrevented) return
  const top = stack[stack.length - 1]
  if (!top) return
  event.preventDefault()
  event.stopPropagation()
  top()
}

const ensureListener = () => {
  if (listenerBound || typeof window === 'undefined') return
  window.addEventListener('keydown', handleKeydown, true)
  listenerBound = true
}

export function useEscToClose(isOpen, onClose) {
  if (typeof onClose !== 'function') return

  const remove = () => {
    const idx = stack.lastIndexOf(onClose)
    if (idx >= 0) stack.splice(idx, 1)
  }

  watch(
    isOpen,
    (visible) => {
      remove()
      if (visible) {
        stack.push(onClose)
        ensureListener()
      }
    },
    { immediate: true }
  )

  onBeforeUnmount(() => {
    remove()
  })
}
