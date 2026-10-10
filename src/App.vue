<script setup lang="ts">
/**
 * 根组件
 *
 * 按 URL query 决定渲染面板还是设置窗口：
 *   index.html?view=panel    → PanelView
 *   index.html?view=settings → SettingsView
 *
 * 两个窗口共用同一套代码，Tauri 通过 tauri.conf.json 的 url 区分。
 */
import { computed, onMounted } from 'vue'
import PanelView from './views/PanelView.vue'
import SettingsView from './views/SettingsView.vue'
import { useConfigStore } from './stores/config'
import { useEdgeStore } from './stores/edge'
import { on } from './composables/useTauri'
import type { Config } from './types'

const view = computed(() => {
  const params = new URLSearchParams(window.location.search)
  return params.get('view') === 'settings' ? 'settings' : 'panel'
})

const configStore = useConfigStore()
const edgeStore = useEdgeStore()

onMounted(async () => {
  // 加载配置
  await configStore.load()

  // 订阅跨窗口配置变更
  await on<Config>('config://changed', (cfg) => {
    configStore.applyExternal(cfg)
  })

  // 面板窗口订阅边缘事件
  if (view.value === 'panel') {
    await edgeStore.subscribe()
    await edgeStore.sync()
  }
})
</script>

<template>
  <SettingsView v-if="view === 'settings'" />
  <PanelView v-else />
</template>
