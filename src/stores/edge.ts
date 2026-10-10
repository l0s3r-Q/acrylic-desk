/**
 * 边缘状态管理
 */

import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api, on } from '@/composables/useTauri'
import type { EdgeState } from '@/types'

export const useEdgeStore = defineStore('edge', () => {
  const state = ref<EdgeState>('visible')
  /** 是否正在拖拽图标（拖拽期间禁止收起） */
  const dragging = ref(false)

  /** 订阅后端边缘事件 */
  async function subscribe() {
    await on<EdgeState>('edge://summon', () => {
      state.value = 'showing'
    })

    await on<void>('edge://shown', () => {
      state.value = 'visible'
    })

    await on<void>('edge://hide', () => {
      state.value = 'hiding'
    })

    await on<void>('edge://hidden', () => {
      state.value = 'hidden'
      // 清理残留 hover 状态（见 docs/03 §4.3）
      document.querySelectorAll('.is-hovered').forEach((el) => {
        el.classList.remove('is-hovered')
      })
    })
  }

  /** 同步一次当前状态 */
  async function sync() {
    try {
      state.value = await api.getEdgeState()
    } catch {
      // 忽略：开发环境下后端可能未就绪
    }
  }

  /** 设置拖拽标志 */
  async function setDragging(v: boolean) {
    dragging.value = v
    try {
      await api.setDragging(v)
    } catch {
      /* 忽略 */
    }
  }

  return { state, dragging, subscribe, sync, setDragging }
})
