<script setup lang="ts">
/**
 * 图标网格 + 拖拽排序
 */
import { computed, ref } from 'vue'
import type { Group, IconItem as IconItemType } from '@/types'
import IconItem from './IconItem.vue'
import { api } from '@/composables/useTauri'
import { useConfigStore } from '@/stores/config'
import { useEdgeStore } from '@/stores/edge'

const props = defineProps<{ group: Group }>()

const configStore = useConfigStore()
const edgeStore = useEdgeStore()

const selectedId = ref<string | null>(null)
const dragIndex = ref<number | null>(null)
const overIndex = ref<number | null>(null)

const items = computed(() =>
  [...props.group.items].sort((a, b) => a.order - b.order)
)

/* ---------------- 拖拽排序 ---------------- */

function onDragStart(index: number, e: DragEvent) {
  dragIndex.value = index
  void edgeStore.setDragging(true)
  if (e.dataTransfer) {
    e.dataTransfer.effectAllowed = 'move'
    // 必须有 setData 才能触发 drag 流程
    e.dataTransfer.setData('text/plain', String(index))
  }
}

function onDragOver(index: number, e: DragEvent) {
  e.preventDefault()
  if (e.dataTransfer) e.dataTransfer.dropEffect = 'move'
  overIndex.value = index
}

async function onDrop(index: number, e: DragEvent) {
  e.preventDefault()
  e.stopPropagation()

  const from = dragIndex.value
  dragIndex.value = null
  overIndex.value = null
  void edgeStore.setDragging(false)

  if (from === null || from === index) return

  // 重排
  const list = [...items.value]
  const [moved] = list.splice(from, 1)
  list.splice(index, 0, moved)

  // 乐观更新本地顺序
  list.forEach((it, i) => {
    it.order = i
  })

  try {
    await api.reorderIcons(
      props.group.id,
      list.map((it) => it.id)
    )
  } catch (err) {
    console.error('[grid] reorder failed:', err)
    // 失败则重新拉取
    await configStore.load()
  }
}

function onDragEnd() {
  dragIndex.value = null
  overIndex.value = null
  void edgeStore.setDragging(false)
}

/* ---------------- 右键菜单 ---------------- */

const menu = ref<{ x: number; y: number; item: IconItemType } | null>(null)

function onContextMenu(item: IconItemType, e: MouseEvent) {
  selectedId.value = item.id
  menu.value = { x: e.clientX, y: e.clientY, item }
  // 点击别处关闭由 window 监听处理
}

async function menuAction(action: 'launch' | 'reveal' | 'remove') {
  const m = menu.value
  menu.value = null
  if (!m) return

  try {
    switch (action) {
      case 'launch':
        await api.launchIcon(m.item.id)
        break
      case 'reveal':
        await api.revealIcon(m.item.id)
        break
      case 'remove':
        await api.removeIcon(props.group.id, m.item.id)
        break
    }
  } catch (e) {
    console.error(`[grid] ${action} failed:`, e)
  }
}

function closeMenu() {
  menu.value = null
}
</script>

<template>
  <div class="icon-grid" @click="closeMenu" @contextmenu.self.prevent>
    <IconItem
      v-for="(item, index) in items"
      :key="item.id"
      :item="item"
      :selected="item.id === selectedId"
      class="icon-grid__cell"
      :class="{
        'is-dragging': dragIndex === index,
        'is-over': overIndex === index && dragIndex !== index,
      }"
      draggable="true"
      @dragstart="onDragStart(index, $event)"
      @dragover="onDragOver(index, $event)"
      @drop="onDrop(index, $event)"
      @dragend="onDragEnd"
      @contextmenu="onContextMenu(item, $event)"
    />
  </div>

  <!-- 右键菜单 -->
  <Teleport to="body">
    <div
      v-if="menu"
      class="ctx-menu"
      :style="{ left: `${menu.x}px`, top: `${menu.y}px` }"
      @click.stop
    >
      <button class="ctx-item" @click="menuAction('launch')">
        <span>打开</span>
      </button>
      <button class="ctx-item" @click="menuAction('reveal')">
        <span>打开所在位置</span>
      </button>
      <div class="ctx-sep" />
      <button class="ctx-item is-danger" @click="menuAction('remove')">
        <span>从面板移除</span>
      </button>
    </div>
  </Teleport>
</template>

<style scoped>
.icon-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(calc(var(--icon-size) + 20px), 1fr));
  gap: var(--icon-gap) 4px;
  align-content: start;
}

.icon-grid__cell {
  transition: opacity var(--dur-fast), transform var(--dur-fast),
    background var(--dur-fast);
}

.icon-grid__cell.is-dragging {
  opacity: 0.35;
}

.icon-grid__cell.is-over {
  background: var(--accent-dim);
  outline: 1px dashed var(--accent);
  outline-offset: -2px;
}

/* ---------- 右键菜单 ---------- */
.ctx-menu {
  position: fixed;
  z-index: var(--z-menu);
  min-width: 168px;
  padding: 4px;
  background: rgba(32, 32, 36, 0.92);
  backdrop-filter: blur(20px) saturate(150%);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-item);
  box-shadow: var(--shadow-menu);
}

.ctx-item {
  display: block;
  width: 100%;
  padding: 6px 10px;
  color: var(--fg);
  font-size: 12px;
  text-align: left;
  background: transparent;
  border: none;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: background var(--dur-fast);
}

.ctx-item:hover {
  background: var(--surface-hover);
}

.ctx-item.is-danger:hover {
  color: #ff6b6b;
  background: rgba(255, 80, 80, 0.12);
}

.ctx-sep {
  height: 1px;
  margin: 4px 6px;
  background: var(--border);
}
</style>
