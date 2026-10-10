<script setup lang="ts">
/**
 * 分组切换标签
 */
import { computed } from 'vue'
import { useConfigStore } from '@/stores/config'

const configStore = useConfigStore()

const groups = computed(() =>
  [...(configStore.config?.groups ?? [])].sort((a, b) => a.order - b.order)
)

const activeId = computed(() => configStore.config?.activeGroupId ?? null)

async function select(id: string) {
  if (id === activeId.value) return
  try {
    await configStore.setActiveGroup(id)
  } catch (e) {
    console.error('[groups] switch failed:', e)
  }
}
</script>

<template>
  <nav class="group-tabs">
    <button
      v-for="g in groups"
      :key="g.id"
      class="tab"
      :class="{ 'is-active': g.id === activeId }"
      :title="g.name"
      @click="select(g.id)"
    >
      <span class="tab__icon">{{ g.icon }}</span>
      <span class="tab__name">{{ g.name }}</span>
    </button>
  </nav>
</template>

<style scoped>
.group-tabs {
  display: flex;
  align-items: center;
  gap: 2px;
  min-width: 0;
  overflow-x: auto;
  scrollbar-width: none;
}

.group-tabs::-webkit-scrollbar {
  display: none;
}

.tab {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
  max-width: 120px;
  height: 26px;
  padding: 0 9px;
  color: var(--fg-secondary);
  font-size: 12px;
  white-space: nowrap;
  background: transparent;
  border: none;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: background var(--dur-fast) var(--ease-out),
    color var(--dur-fast) var(--ease-out);
}

.tab:hover {
  color: var(--fg);
  background: var(--surface-hover);
}

.tab.is-active {
  color: var(--fg);
  background: var(--surface-active);
}

.tab__icon {
  font-size: 12px;
  line-height: 1;
}

.tab__name {
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
