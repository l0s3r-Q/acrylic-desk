<script setup lang="ts">
/**
 * 图标项
 *
 * ⚠️ 图标来源（见 docs/04 §7）：
 * 不能直接用 file:/// 引用本地 exe 图标，
 * WebView2 禁止跨协议加载。必须经 Rust 提取后以 data URL 提供。
 */
import { computed, onMounted, ref, watch } from 'vue'
import type { IconItem } from '@/types'
import { api } from '@/composables/useTauri'

const props = defineProps<{
  item: IconItem
  selected?: boolean
}>()

const emit = defineEmits<{
  (e: 'contextmenu', ev: MouseEvent): void
  (e: 'launch'): void
}>()

const dataUrl = ref<string | null>(null)
const loadingIcon = ref(false)
const iconFailed = ref(false)

/**
 * 加载图标
 *
 * 设计决策：组件挂载时立即加载，而不是等鼠标悬停。
 * 理由：面板本身是触发性 UI，用户碰边唤出时期望图标已就绪；
 * 若等 hover 才加载，会看到短暂的 emoji 兜底图标闪烁。
 * 图标已按内容哈希缓存在磁盘，加载开销很小。
 */
async function loadIcon() {
  if (dataUrl.value || loadingIcon.value || iconFailed.value) return
  if (!props.item.iconPath) {
    iconFailed.value = true
    return
  }

  loadingIcon.value = true
  try {
    dataUrl.value = await api.getIconDataUrl(props.item.id)
  } catch (e) {
    console.warn('[icon] load failed for', props.item.name, e)
    iconFailed.value = true
  } finally {
    loadingIcon.value = false
  }
}

onMounted(loadIcon)

// 图标路径变化时（如刷新图标后）重新加载
watch(
  () => props.item.iconPath,
  () => {
    if (props.item.iconPath && props.item.iconPath !== dataUrl.value) {
      iconFailed.value = false
      dataUrl.value = null
      void loadIcon()
    }
  }
)

/** 根据类型给出兜底图标 */
const fallbackEmoji = computed(() => {
  switch (props.item.type) {
    case 'folder':
      return '📁'
    case 'url':
      return '🌐'
    case 'file':
      return '📄'
    default:
      return '📦'
  }
})

async function onDblClick() {
  try {
    await api.launchIcon(props.item.id)
    emit('launch')
  } catch (e) {
    console.error('[icon] launch failed:', e)
  }
}
</script>

<template>
  <div
    class="icon-item"
    :class="{ 'is-selected': selected }"
    :title="item.target"
    @dblclick="onDblClick"
    @contextmenu.prevent="emit('contextmenu', $event)"
  >
    <div class="icon-item__art">
      <img
        v-if="dataUrl"
        :src="dataUrl"
        :alt="item.name"
        class="icon-item__img"
        draggable="false"
      />
      <span v-else class="icon-item__fallback">{{ fallbackEmoji }}</span>

      <!-- 管理员标记 -->
      <span v-if="item.runAsAdmin" class="icon-item__badge" title="以管理员身份运行">
        <svg width="9" height="9" viewBox="0 0 12 12" fill="none">
          <path
            d="M6 1l4 1.5v3.2c0 2.3-1.6 4.3-4 5.3-2.4-1-4-3-4-5.3V2.5L6 1z"
            fill="currentColor"
          />
        </svg>
      </span>
    </div>

    <span v-if="$attrs.showLabel !== false" class="icon-item__label">
      {{ item.name }}
    </span>
  </div>
</template>

<style scoped>
.icon-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 6px 4px;
  border-radius: var(--radius-item);
  cursor: pointer;
  transition: background var(--dur-fast) var(--ease-out);
}

.icon-item:hover,
.icon-item.is-hovered {
  background: var(--surface-hover);
}

.icon-item.is-selected {
  background: var(--accent-dim);
}

.icon-item__art {
  position: relative;
  display: grid;
  place-items: center;
  width: var(--icon-size);
  height: var(--icon-size);
}

.icon-item__img {
  width: 100%;
  height: 100%;
  object-fit: contain;
  /* 图标加一点投影，在透明背景上更清晰 */
  filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.35));
}

.icon-item__fallback {
  font-size: calc(var(--icon-size) * 0.62);
  line-height: 1;
  filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.35));
}

.icon-item__badge {
  position: absolute;
  right: -1px;
  bottom: -1px;
  display: grid;
  place-items: center;
  width: 13px;
  height: 13px;
  color: #fff;
  background: #d97706;
  border-radius: 50%;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.4);
}

.icon-item__label {
  max-width: calc(var(--icon-size) + 16px);
  font-size: 11px;
  line-height: 1.25;
  text-align: center;
  color: var(--fg);
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  line-clamp: 2;
  -webkit-box-orient: vertical;
  word-break: break-all;
  /* 文字投影，保证在任意背景图上可读 */
  text-shadow: 0 1px 3px rgba(0, 0, 0, 0.6);
}
</style>
