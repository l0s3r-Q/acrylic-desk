<script setup lang="ts">
/**
 * 主面板视图
 *
 * 这是贴在屏幕边缘、碰边唤出的那块面板。
 * 关键点（见 docs/03 §1.4）：容器必须完全透明，
 * 只有 .panel-surface 这一层有 rgba 半透明背景。
 */
import { computed, ref } from 'vue'
import { useConfigStore } from '@/stores/config'
import { useEdgeStore } from '@/stores/edge'
import IconGrid from '@/components/panel/IconGrid.vue'
import GroupTabs from '@/components/panel/GroupTabs.vue'
import { api } from '@/composables/useTauri'

const configStore = useConfigStore()
const edgeStore = useEdgeStore()

const dragOver = ref(false)
const toast = ref<string | null>(null)

const cfg = computed(() => configStore.config)
const group = computed(() => configStore.activeGroup)

/** 面板根样式：动态应用不透明度与背景 */
const panelStyle = computed(() => {
  const a = cfg.value?.appearance
  if (!a) return {}

  const style: Record<string, string> = {
    '--panel-opacity': String(a.opacity),
    '--icon-size': `${a.icons.size}px`,
    '--icon-gap': `${a.icons.gap}px`,
  }

  const bg = a.background

  // 背景图
  if (bg.type === 'image' && bg.imagePath) {
    const url = toAssetUrl(bg.imagePath)
    style.backgroundImage = `linear-gradient(rgba(0,0,0,${bg.dim}), rgba(0,0,0,${bg.dim})), url("${url}")`

    switch (bg.fit) {
      case 'cover':
        style.backgroundSize = 'cover'
        style.backgroundRepeat = 'no-repeat'
        break
      case 'contain':
        style.backgroundSize = 'contain'
        style.backgroundRepeat = 'no-repeat'
        break
      case 'fill':
        style.backgroundSize = '100% 100%'
        style.backgroundRepeat = 'no-repeat'
        break
      case 'tile':
        style.backgroundSize = 'auto'
        style.backgroundRepeat = 'repeat'
        break
      case 'center':
        style.backgroundSize = 'auto'
        style.backgroundRepeat = 'no-repeat'
        style.backgroundPosition = 'center'
        break
    }
  } else if (bg.type === 'color') {
    style.backgroundColor = bg.color
  }

  // 圆角
  if (cfg.value) {
    style.borderRadius = `${cfg.value.panel.cornerRadius}px`
  }

  return style
})

/** 把本地路径转为可用的资源 URL */
function toAssetUrl(path: string): string {
  // 使用 Tauri 的 asset 协议
  const normalized = path.replace(/\\/g, '/')
  return `http://asset.localhost/${encodeURIComponent(normalized)}`
}

/** 拖入文件 */
async function onDrop(e: DragEvent) {
  dragOver.value = false
  await edgeStore.setDragging(false)

  const files = e.dataTransfer?.files
  if (!files || files.length === 0) return

  // WebView2 下 file 对象带 path 属性
  const paths: string[] = []
  for (const f of Array.from(files)) {
    const p = (f as File & { path?: string }).path
    if (p) paths.push(p)
  }

  if (paths.length === 0) {
    showToast('无法读取文件路径')
    return
  }

  if (!group.value) {
    showToast('请先创建一个分组')
    return
  }

  try {
    await api.addIcons(group.value.id, paths)
    showToast(`已添加 ${paths.length} 项`)
  } catch (err) {
    showToast(err instanceof Error ? err.message : '添加失败')
  }
}

function onDragEnter(e: DragEvent) {
  e.preventDefault()
  dragOver.value = true
  void edgeStore.setDragging(true)
}

function onDragOver(e: DragEvent) {
  e.preventDefault()
  if (e.dataTransfer) e.dataTransfer.dropEffect = 'copy'
}

function onDragLeave(e: DragEvent) {
  // 只有真正离开面板才复位
  const related = e.relatedTarget as Node | null
  if (related && (e.currentTarget as Node).contains(related)) return
  dragOver.value = false
  void edgeStore.setDragging(false)
}

let toastTimer: number | undefined
function showToast(msg: string) {
  toast.value = msg
  window.clearTimeout(toastTimer)
  toastTimer = window.setTimeout(() => {
    toast.value = null
  }, 2200)
}

async function openSettings() {
  try {
    await api.openSettings()
  } catch (e) {
    showToast(e instanceof Error ? e.message : '无法打开设置')
  }
}
</script>

<template>
  <div
    class="panel-root"
    :class="{ 'is-dragover': dragOver }"
    @dragenter="onDragEnter"
    @dragover="onDragOver"
    @dragleave="onDragLeave"
    @drop="onDrop"
  >
    <!-- 面板实体：唯一有半透明背景的层 -->
    <div class="panel-surface" :style="panelStyle">
      <!-- 顶部工具条 -->
      <header class="panel-header">
        <GroupTabs v-if="cfg && cfg.groups.length" />
        <button class="icon-btn" title="设置" @click="openSettings">
          <svg width="15" height="15" viewBox="0 0 16 16" fill="none">
            <path
              d="M8 10a2 2 0 100-4 2 2 0 000 4z"
              stroke="currentColor"
              stroke-width="1.3"
            />
            <path
              d="M13.3 9.7a1.1 1.1 0 00.22 1.21l.04.04a1.33 1.33 0 11-1.88 1.88l-.04-.04a1.1 1.1 0 00-1.21-.22 1.1 1.1 0 00-.67 1v.11a1.33 1.33 0 11-2.67 0v-.06a1.1 1.1 0 00-.72-1 1.1 1.1 0 00-1.21.22l-.04.04a1.33 1.33 0 11-1.88-1.88l.04-.04a1.1 1.1 0 00.22-1.21 1.1 1.1 0 00-1-.67h-.11a1.33 1.33 0 110-2.67h.06a1.1 1.1 0 001-.72 1.1 1.1 0 00-.22-1.21l-.04-.04a1.33 1.33 0 111.88-1.88l.04.04a1.1 1.1 0 001.21.22h.05a1.1 1.1 0 00.67-1v-.11a1.33 1.33 0 112.67 0v.06a1.1 1.1 0 00.67 1 1.1 1.1 0 001.21-.22l.04-.04a1.33 1.33 0 111.88 1.88l-.04.04a1.1 1.1 0 00-.22 1.21v.05a1.1 1.1 0 001 .67h.11a1.33 1.33 0 110 2.67h-.06a1.1 1.1 0 00-1 .67z"
              stroke="currentColor"
              stroke-width="1.1"
            />
          </svg>
        </button>
      </header>

      <!-- 图标网格 -->
      <main class="panel-body">
        <!-- 诊断分支：加载失败时显示原因 -->
        <div v-if="configStore.error" class="empty">
          <p class="err">⚠ 配置加载失败</p>
          <p class="err-detail">{{ configStore.error }}</p>
        </div>
        <div v-else-if="configStore.loading" class="empty">
          <p>加载中…</p>
        </div>
        <IconGrid v-else-if="group" :group="group" />
        <div v-else class="empty">
          <p>暂无分组</p>
          <button class="text-btn" @click="openSettings">前往设置创建</button>
        </div>
      </main>

      <!-- 拖入提示层 -->
      <div v-if="dragOver" class="drop-hint">
        <div class="drop-hint__inner">
          <svg width="28" height="28" viewBox="0 0 24 24" fill="none">
            <path
              d="M12 16V4m0 0L7 9m5-5l5 5M4 17v2a2 2 0 002 2h12a2 2 0 002-2v-2"
              stroke="currentColor"
              stroke-width="1.6"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
          <span>松开以添加到「{{ group?.name ?? '当前分组' }}」</span>
        </div>
      </div>

      <!-- 轻提示 -->
      <Transition name="toast">
        <div v-if="toast" class="toast">{{ toast }}</div>
      </Transition>
    </div>
  </div>
</template>

<style scoped>
/* 根容器：完全透明，不拦截任何视觉 */
.panel-root {
  position: relative;
  width: 100%;
  height: 100%;
  padding: 0;
  background: transparent;
  overflow: hidden;
}

/* 面板实体：唯一有背景的层 */
.panel-surface {
  position: relative;
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
  overflow: hidden;

  background-color: var(--surface);
  border: 1px solid var(--border);
  border-radius: inherit;

  /* 在系统 Acrylic 之上再叠一层背景模糊，质感更强（见 docs/03 §1.4） */
  backdrop-filter: blur(24px) saturate(140%);
  -webkit-backdrop-filter: blur(24px) saturate(140%);

  transition: border-color var(--dur-normal) var(--ease-out);
}

.is-dragover .panel-surface {
  border-color: var(--accent);
}

/* ---------- 顶部工具条 ---------- */
.panel-header {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
  height: 36px;
  padding: 0 6px 0 8px;
  border-bottom: 1px solid var(--border);
}

.panel-header :deep(.group-tabs) {
  flex: 1;
  min-width: 0;
}

.icon-btn {
  display: grid;
  place-items: center;
  flex-shrink: 0;
  width: 26px;
  height: 26px;
  padding: 0;
  color: var(--fg-secondary);
  background: transparent;
  border: none;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: background var(--dur-fast) var(--ease-out), color var(--dur-fast);
}

.icon-btn:hover {
  color: var(--fg);
  background: var(--surface-hover);
}

.icon-btn:active {
  background: var(--surface-active);
}

/* ---------- 内容区 ---------- */
.panel-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 8px;
}

.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  height: 100%;
  color: var(--fg-tertiary);
  font-size: 12px;
}

.empty p {
  margin: 0;
}

.err {
  color: #ff8a8a;
  font-weight: 600;
}

.err-detail {
  max-width: 90%;
  font-size: 11px;
  line-height: 1.5;
  text-align: center;
  word-break: break-all;
  opacity: 0.75;
}

.text-btn {
  padding: 4px 10px;
  color: var(--accent);
  font-size: 12px;
  background: transparent;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: background var(--dur-fast);
}

.text-btn:hover {
  background: var(--accent-dim);
}

/* ---------- 拖入提示 ---------- */
.drop-hint {
  position: absolute;
  inset: 0;
  display: grid;
  place-items: center;
  background: rgba(10, 132, 255, 0.1);
  backdrop-filter: blur(2px);
  pointer-events: none;
}

.drop-hint__inner {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 18px 22px;
  color: var(--fg);
  font-size: 12px;
  background: rgba(0, 0, 0, 0.4);
  border: 1px dashed var(--accent);
  border-radius: var(--radius-panel);
}

/* ---------- 轻提示 ---------- */
.toast {
  position: absolute;
  left: 50%;
  bottom: 14px;
  transform: translateX(-50%);
  padding: 6px 14px;
  color: var(--fg);
  font-size: 12px;
  white-space: nowrap;
  background: rgba(0, 0, 0, 0.72);
  border-radius: 14px;
  pointer-events: none;
}

.toast-enter-active,
.toast-leave-active {
  transition: opacity var(--dur-normal) var(--ease-out),
    transform var(--dur-normal) var(--ease-out);
}

.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateX(-50%) translateY(6px);
}
</style>
