/**
 * 配置状态管理
 */

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { api } from '@/composables/useTauri'
import type { Config, Edge, Group } from '@/types'

export const useConfigStore = defineStore('config', () => {
  const config = ref<Config | null>(null)
  const loading = ref(true)
  const error = ref<string | null>(null)

  /** 当前激活的分组 */
  const activeGroup = computed<Group | null>(() => {
    if (!config.value) return null
    const id = config.value.activeGroupId
    return config.value.groups.find((g) => g.id === id) ?? config.value.groups[0] ?? null
  })

  /** 加载配置 */
  async function load() {
    loading.value = true
    error.value = null
    try {
      const cfg = await api.getConfig()
      config.value = cfg
      // 诊断输出：确认前端拿到的数据结构
      console.log('[config] loaded:', {
        groups: cfg?.groups?.length ?? 0,
        activeGroupId: cfg?.activeGroupId,
        raw: cfg,
      })
    } catch (e) {
      error.value = e instanceof Error ? e.message : '配置加载失败'
      console.error('[config] load failed:', e)
    } finally {
      loading.value = false
    }
  }

  /** 局部更新配置（乐观更新 + 后端确认） */
  async function update(patch: Parameters<typeof api.updateConfig>[0]) {
    try {
      const updated = await api.updateConfig(patch)
      config.value = updated
    } catch (e) {
      error.value = e instanceof Error ? e.message : '配置更新失败'
      // 回滚：重新拉取
      await load()
      throw e
    }
  }

  /** 切换挂载边 */
  async function setEdge(edge: Edge) {
    await api.setPanelEdge(edge)
    if (config.value) {
      config.value.panel.edge = edge
    }
  }

  /** 切换激活分组 */
  async function setActiveGroup(groupId: string) {
    await api.setActiveGroup(groupId)
    if (config.value) {
      config.value.activeGroupId = groupId
    }
  }

  /** 由后端事件更新 */
  function applyExternal(next: Config) {
    config.value = next
  }

  /** 恢复默认 */
  async function reset() {
    config.value = await api.resetConfig()
  }

  return {
    config,
    loading,
    error,
    activeGroup,
    load,
    update,
    setEdge,
    setActiveGroup,
    applyExternal,
    reset,
  }
})
