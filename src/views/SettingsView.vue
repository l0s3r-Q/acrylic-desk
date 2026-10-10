<script setup lang="ts">
/**
 * 设置窗口
 *
 * 与面板窗口共用同一份代码，通过 URL query 区分。
 */
import { computed, ref } from 'vue'
import { useConfigStore } from '@/stores/config'
import { api } from '@/composables/useTauri'
import { EDGE_LABELS, EFFECT_LABELS, BG_FIT_LABELS } from '@/types'
import type { Edge, EffectKind, BgFit, ScanEntry } from '@/types'

const configStore = useConfigStore()

const activeTab = ref<'panel' | 'edge' | 'appearance' | 'groups' | 'import' | 'about'>(
  'panel'
)
const saving = ref(false)
const message = ref<string | null>(null)

/** 热键录制状态 */
const recordingHotkey = ref(false)
const hotkeyError = ref<string | null>(null)

/** 扫描结果 */
const scanResults = ref<ScanEntry[]>([])
const scanning = ref(false)
const selectedPaths = ref<Set<string>>(new Set())

const cfg = computed(() => configStore.config)

const tabs = [
  { id: 'panel', label: '面板', icon: '▭' },
  { id: 'edge', label: '边缘行为', icon: '⇥' },
  { id: 'appearance', label: '外观', icon: '◐' },
  { id: 'groups', label: '分组', icon: '☰' },
  { id: 'import', label: '导入图标', icon: '⬇' },
  { id: 'about', label: '关于', icon: 'ⓘ' },
] as const

/** 通用更新助手 */
async function patch(update: Record<string, unknown>, section: string) {
  if (!cfg.value) return
  saving.value = true
  try {
    await configStore.update({ [section]: update } as never)
    flash('已保存')
  } catch (e) {
    flash(e instanceof Error ? e.message : '保存失败')
  } finally {
    saving.value = false
  }
}

let msgTimer: number | undefined
function flash(text: string) {
  message.value = text
  window.clearTimeout(msgTimer)
  msgTimer = window.setTimeout(() => (message.value = null), 1800)
}

async function changeEdge(edge: Edge) {
  try {
    await configStore.setEdge(edge)
    flash('已切换挂载位置')
  } catch (e) {
    flash(e instanceof Error ? e.message : '切换失败')
  }
}

async function chooseBackground() {
  try {
    const path = await api.pickBackground()
    if (path) {
      await patch({ type: 'image', imagePath: path }, 'appearance')
      await configStore.update({
        appearance: { background: { type: 'image', imagePath: path } },
      } as never)
      flash('背景已更新')
    }
  } catch (e) {
    flash(e instanceof Error ? e.message : '选择背景失败')
  }
}

async function resetAll() {
  if (!confirm('确定要恢复所有设置为默认值吗？当前配置会自动备份。')) return
  try {
    await configStore.reset()
    flash('已恢复默认设置')
  } catch (e) {
    flash(e instanceof Error ? e.message : '恢复失败')
  }
}

/* ---------------- 热键录制 ---------------- */

/**
 * 按键事件 → 热键字符串
 *
 * 采用 PowerToys 风格：按下修饰键+主键即完成录制。
 */
function onHotkeyKeydown(e: KeyboardEvent) {
  if (!recordingHotkey.value) return
  e.preventDefault()
  e.stopPropagation()

  const parts: string[] = []
  if (e.ctrlKey) parts.push('Ctrl')
  if (e.altKey) parts.push('Alt')
  if (e.shiftKey) parts.push('Shift')
  if (e.metaKey) parts.push('Win')

  const key = e.key

  // 忽略单独的修饰键
  if (['Control', 'Alt', 'Shift', 'Meta'].includes(key)) return

  // 主键规范化
  let main = ''
  if (key === ' ') main = 'Space'
  else if (key === 'Escape') {
    // Esc 取消录制
    recordingHotkey.value = false
    hotkeyError.value = null
    return
  } else if (key.length === 1) main = key.toUpperCase()
  else if (/^F\d{1,2}$/.test(key)) main = key
  else if (key === 'ArrowUp') main = 'Up'
  else if (key === 'ArrowDown') main = 'Down'
  else if (key === 'ArrowLeft') main = 'Left'
  else if (key === 'ArrowRight') main = 'Right'
  else if (key === 'Delete') main = 'Delete'
  else if (key === 'Home') main = 'Home'
  else if (key === 'End') main = 'End'
  else if (key === 'PageUp') main = 'PageUp'
  else if (key === 'PageDown') main = 'PageDown'
  else if (key === 'Tab') main = 'Tab'
  else if (key === 'Enter') main = 'Enter'
  else if (key === 'Backspace') main = 'Backspace'
  else main = key

  if (parts.length === 0) {
    hotkeyError.value = '必须包含至少一个修饰键（Ctrl / Alt / Shift / Win）'
    return
  }

  const spec = [...parts, main].join('+')

  // 交给后端校验
  api
    .validateHotkey(spec)
    .then(() => {
      hotkeyError.value = null
      recordingHotkey.value = false
      void applyHotkey(spec)
    })
    .catch((err) => {
      hotkeyError.value = err instanceof Error ? err.message : '热键无效'
    })
}

async function applyHotkey(spec: string) {
  try {
    await api.applyHotkey(spec, true)
    if (cfg.value) cfg.value.edge.summonHotkey = spec
    flash(`已设置热键：${spec}`)
  } catch (e) {
    hotkeyError.value = e instanceof Error ? e.message : '热键设置失败'
    flash(hotkeyError.value)
  }
}

function startRecording() {
  recordingHotkey.value = true
  hotkeyError.value = null
}

/* ---------------- 扫描导入 ---------------- */

async function scan(kind: 'desktop' | 'startmenu') {
  scanning.value = true
  scanResults.value = []
  selectedPaths.value = new Set()
  try {
    scanResults.value =
      kind === 'desktop' ? await api.scanDesktop() : await api.scanStartMenu()
    flash(`扫描到 ${scanResults.value.length} 项`)
  } catch (e) {
    flash(e instanceof Error ? e.message : '扫描失败')
  } finally {
    scanning.value = false
  }
}

function toggleSelect(path: string) {
  const s = new Set(selectedPaths.value)
  if (s.has(path)) s.delete(path)
  else s.add(path)
  selectedPaths.value = s
}

function selectAll() {
  if (selectedPaths.value.size === scanResults.value.length) {
    selectedPaths.value = new Set()
  } else {
    selectedPaths.value = new Set(scanResults.value.map((r) => r.path))
  }
}

async function importSelected() {
  const group = configStore.activeGroup
  if (!group) {
    flash('没有可用分组')
    return
  }
  if (selectedPaths.value.size === 0) {
    flash('请先选择要导入的项目')
    return
  }

  try {
    const paths = [...selectedPaths.value]
    const added = await api.addIcons(group.id, paths)
    await configStore.load() // 重新加载以刷新列表
    scanResults.value = scanResults.value.filter(
      (r) => !selectedPaths.value.has(r.path)
    )
    selectedPaths.value = new Set()
    flash(`已导入 ${added.length} 项`)
  } catch (e) {
    flash(e instanceof Error ? e.message : '导入失败')
  }
}
</script>

<template>
  <div class="settings-root">
    <!-- 侧边导航 -->
    <aside class="settings-nav">
      <div class="settings-nav__title">AcrylicDesk</div>
      <button
        v-for="t in tabs"
        :key="t.id"
        class="nav-item"
        :class="{ 'is-active': activeTab === t.id }"
        @click="activeTab = t.id"
      >
        <span class="nav-item__icon">{{ t.icon }}</span>
        <span>{{ t.label }}</span>
      </button>
    </aside>

    <!-- 内容区 -->
    <main class="settings-main">
      <div v-if="!cfg" class="loading">加载中…</div>

      <template v-else>
        <!-- ========== 面板 ========== -->
        <section v-show="activeTab === 'panel'" class="section">
          <h2 class="section__title">面板位置</h2>
          <p class="section__desc">选择面板吸附到屏幕的哪一侧。切换后面板会立即移动。</p>

          <div class="edge-picker">
            <button
              v-for="(label, key) in EDGE_LABELS"
              :key="key"
              class="edge-btn"
              :class="{ 'is-active': cfg.panel.edge === key }"
              @click="changeEdge(key as Edge)"
            >
              {{ label }}
            </button>
          </div>

          <h2 class="section__title">尺寸</h2>
          <div class="field">
            <label>长度
              <span class="field__value">{{ cfg.panel.length }} px</span>
            </label>
            <input
              type="range"
              min="200"
              max="1600"
              step="10"
              :value="cfg.panel.length"
              @change="
                patch({ length: +($event.target as HTMLInputElement).value }, 'panel')
              "
            />
          </div>

          <div class="field">
            <label>厚度
              <span class="field__value">{{ cfg.panel.thickness }} px</span>
            </label>
            <input
              type="range"
              min="120"
              max="600"
              step="10"
              :value="cfg.panel.thickness"
              @change="
                patch({ thickness: +($event.target as HTMLInputElement).value }, 'panel')
              "
            />
          </div>

          <div class="field">
            <label>距边缘间距
              <span class="field__value">{{ cfg.panel.offset }} px</span>
            </label>
            <input
              type="range"
              min="0"
              max="200"
              step="2"
              :value="cfg.panel.offset"
              @change="patch({ offset: +($event.target as HTMLInputElement).value }, 'panel')"
            />
          </div>

          <div class="field">
            <label>圆角
              <span class="field__value">{{ cfg.panel.cornerRadius }} px</span>
            </label>
            <input
              type="range"
              min="0"
              max="24"
              step="1"
              :value="cfg.panel.cornerRadius"
              @change="
                patch({ cornerRadius: +($event.target as HTMLInputElement).value }, 'panel')
              "
            />
          </div>

          <label class="switch">
            <input
              type="checkbox"
              :checked="cfg.panel.alwaysOnTop"
              @change="
                patch({ alwaysOnTop: ($event.target as HTMLInputElement).checked }, 'panel')
              "
            />
            <span>始终置顶</span>
          </label>
        </section>

        <!-- ========== 边缘行为 ========== -->
        <section v-show="activeTab === 'edge'" class="section">
          <h2 class="section__title">唤出方式</h2>
          <p class="section__desc">
            鼠标碰到屏幕边缘时唤出面板，移开后自动收起。
          </p>

          <label class="switch">
            <input
              type="checkbox"
              :checked="cfg.edge.enableHover"
              @change="
                patch({ enableHover: ($event.target as HTMLInputElement).checked }, 'edge')
              "
            />
            <span>启用鼠标碰边唤出</span>
          </label>

          <label class="switch">
            <input
              type="checkbox"
              :checked="cfg.edge.alwaysVisible"
              @change="
                patch({ alwaysVisible: ($event.target as HTMLInputElement).checked }, 'edge')
              "
            />
            <span>常驻显示（不自动收起）</span>
          </label>

          <label class="switch">
            <input
              type="checkbox"
              :checked="cfg.edge.disableOnFullscreen"
              @change="
                patch(
                  { disableOnFullscreen: ($event.target as HTMLInputElement).checked },
                  'edge'
                )
              "
            />
            <span>全屏应用时自动隐藏</span>
          </label>

          <h2 class="section__title">触发灵敏度</h2>

          <div class="field">
            <label>触发区宽度
              <span class="field__value">{{ cfg.edge.triggerWidth }} px</span>
            </label>
            <input
              type="range"
              min="1"
              max="20"
              step="1"
              :value="cfg.edge.triggerWidth"
              @change="
                patch({ triggerWidth: +($event.target as HTMLInputElement).value }, 'edge')
              "
            />
            <p class="hint">数值越大越容易触发。如果唤不出，试着调大。</p>
          </div>

          <div class="field">
            <label>收起延时
              <span class="field__value">{{ cfg.edge.hideDelay }} ms</span>
            </label>
            <input
              type="range"
              min="0"
              max="2000"
              step="50"
              :value="cfg.edge.hideDelay"
              @change="
                patch({ hideDelay: +($event.target as HTMLInputElement).value }, 'edge')
              "
            />
          </div>

          <div class="field">
            <label>动画时长
              <span class="field__value">{{ cfg.edge.animationDuration }} ms</span>
            </label>
            <input
              type="range"
              min="0"
              max="800"
              step="20"
              :value="cfg.edge.animationDuration"
              @change="
                patch(
                  { animationDuration: +($event.target as HTMLInputElement).value },
                  'edge'
                )
              "
            />
          </div>

          <h2 class="section__title">全局热键</h2>
          <p class="section__desc">
            任何时候按此组合键都能强制唤出或收起面板。点击下方按钮后直接按下你想要的组合键。
          </p>

          <div class="hotkey-row">
            <button
              class="hotkey-input"
              :class="{ 'is-recording': recordingHotkey }"
              @click="startRecording"
              @keydown="onHotkeyKeydown"
            >
              <template v-if="recordingHotkey">
                <span class="hotkey-hint">请按下组合键…（Esc 取消）</span>
              </template>
              <template v-else>
                <kbd v-for="part in cfg.edge.summonHotkey.split('+')" :key="part">
                  {{ part }}
                </kbd>
              </template>
            </button>
            <button
              class="btn"
              @click="
                api
                  .applyHotkey(cfg.edge.summonHotkey, false)
                  .then(() => {
                    if (cfg) cfg.edge.hotkeyEnabled = false
                    flash('已停用热键')
                  })
                  .catch((e) => flash(String(e)))
              "
            >
              停用
            </button>
          </div>

          <p v-if="hotkeyError" class="hint err-text">{{ hotkeyError }}</p>
          <p v-else class="hint">
            提示：全局热键可能与系统或其他软件冲突，若设置无效请更换组合。
          </p>
        </section>

        <!-- ========== 外观 ========== -->
        <section v-show="activeTab === 'appearance'" class="section">
          <h2 class="section__title">材质</h2>
          <div class="chip-row">
            <button
              v-for="(label, key) in EFFECT_LABELS"
              :key="key"
              class="chip"
              :class="{ 'is-active': cfg.appearance.effect === key }"
              @click="patch({ effect: key as EffectKind }, 'appearance')"
            >
              {{ label }}
            </button>
          </div>

          <div class="field">
            <label>不透明度
              <span class="field__value">
                {{ Math.round(cfg.appearance.opacity * 100) }}%
              </span>
            </label>
            <input
              type="range"
              min="0.3"
              max="1"
              step="0.05"
              :value="cfg.appearance.opacity"
              @change="
                patch({ opacity: +($event.target as HTMLInputElement).value }, 'appearance')
              "
            />
          </div>

          <h2 class="section__title">背景</h2>
          <div class="chip-row">
            <button
              v-for="t in [
                { k: 'none', label: '无' },
                { k: 'image', label: '图片' },
                { k: 'color', label: '纯色' },
              ]"
              :key="t.k"
              class="chip"
              :class="{ 'is-active': cfg.appearance.background.type === t.k }"
              @click="
                configStore.update({
                  appearance: { background: { type: t.k as never } },
                } as never)
              "
            >
              {{ t.label }}
            </button>
          </div>

          <div v-if="cfg.appearance.background.type === 'image'" class="field">
            <button class="btn" @click="chooseBackground">选择背景图片…</button>
            <p v-if="cfg.appearance.background.imagePath" class="hint path">
              {{ cfg.appearance.background.imagePath }}
            </p>

            <label class="field__label">填充方式</label>
            <div class="chip-row">
              <button
                v-for="(label, key) in BG_FIT_LABELS"
                :key="key"
                class="chip"
                :class="{ 'is-active': cfg.appearance.background.fit === key }"
                @click="
                  configStore.update({
                    appearance: { background: { fit: key as BgFit } },
                  } as never)
                "
              >
                {{ label }}
              </button>
            </div>

            <label>背景暗度
              <span class="field__value">
                {{ Math.round(cfg.appearance.background.dim * 100) }}%
              </span>
            </label>
            <input
              type="range"
              min="0"
              max="0.9"
              step="0.05"
              :value="cfg.appearance.background.dim"
              @change="
                configStore.update({
                  appearance: {
                    background: { dim: +($event.target as HTMLInputElement).value },
                  },
                } as never)
              "
            />
          </div>

          <h2 class="section__title">图标</h2>
          <div class="field">
            <label>图标大小
              <span class="field__value">{{ cfg.appearance.icons.size }} px</span>
            </label>
            <input
              type="range"
              min="32"
              max="96"
              step="4"
              :value="cfg.appearance.icons.size"
              @change="
                configStore.update({
                  appearance: {
                    icons: { size: +($event.target as HTMLInputElement).value },
                  },
                } as never)
              "
            />
          </div>

          <label class="switch">
            <input
              type="checkbox"
              :checked="cfg.appearance.icons.showLabel"
              @change="
                configStore.update({
                  appearance: {
                    icons: {
                      showLabel: ($event.target as HTMLInputElement).checked,
                    },
                  },
                } as never)
              "
            />
            <span>显示图标名称</span>
          </label>
        </section>

        <!-- ========== 分组 ========== -->
        <section v-show="activeTab === 'groups'" class="section">
          <h2 class="section__title">分组管理</h2>
          <p class="section__desc">
            分组用于把图标分类。当前共 {{ cfg.groups.length }} 个分组。
          </p>

          <ul class="group-list">
            <li
              v-for="g in cfg.groups"
              :key="g.id"
              class="group-row"
              :class="{ 'is-active': g.id === cfg.activeGroupId }"
            >
              <span class="group-row__icon">{{ g.icon }}</span>
              <span class="group-row__name">{{ g.name }}</span>
              <span class="group-row__count">{{ g.items.length }} 项</span>
              <button
                v-if="g.id !== cfg.activeGroupId"
                class="mini-btn"
                @click="configStore.setActiveGroup(g.id)"
              >
                设为当前
              </button>
            </li>
          </ul>
        </section>

        <!-- ========== 导入图标 ========== -->
        <section v-show="activeTab === 'import'" class="section">
          <h2 class="section__title">从系统导入图标</h2>
          <p class="section__desc">
            一键扫描桌面或开始菜单，勾选后批量导入到当前分组
            （<strong>{{ configStore.activeGroup?.name ?? '—' }}</strong>）。
          </p>

          <div class="chip-row">
            <button class="btn" :disabled="scanning" @click="scan('desktop')">
              {{ scanning ? '扫描中…' : '扫描桌面' }}
            </button>
            <button class="btn" :disabled="scanning" @click="scan('startmenu')">
              {{ scanning ? '扫描中…' : '扫描开始菜单' }}
            </button>
          </div>

          <template v-if="scanResults.length">
            <div class="scan-toolbar">
              <label class="switch">
                <input
                  type="checkbox"
                  :checked="selectedPaths.size === scanResults.length"
                  @change="selectAll"
                />
                <span>全选（{{ scanResults.length }} 项）</span>
              </label>
              <button
                class="btn is-primary"
                :disabled="selectedPaths.size === 0"
                @click="importSelected"
              >
                导入选中项（{{ selectedPaths.size }}）
              </button>
            </div>

            <ul class="scan-list">
              <li
                v-for="item in scanResults"
                :key="item.path"
                class="scan-row"
                :class="{ 'is-selected': selectedPaths.has(item.path) }"
                @click="toggleSelect(item.path)"
              >
                <input
                  type="checkbox"
                  :checked="selectedPaths.has(item.path)"
                  @click.stop="toggleSelect(item.path)"
                />
                <span class="scan-row__name">{{ item.name }}</span>
                <span class="scan-row__path">{{ item.path }}</span>
              </li>
            </ul>
          </template>

          <p v-else-if="!scanning" class="hint">
            点击上方按钮开始扫描。扫描只读取文件列表，不会修改任何文件。
          </p>
        </section>

        <!-- ========== 关于 ========== -->
        <section v-show="activeTab === 'about'" class="section">
          <h2 class="section__title">关于 AcrylicDesk</h2>
          <p class="section__desc">
            把桌面图标收纳进贴靠屏幕边缘的亚克力面板，鼠标碰边即唤出。
          </p>

          <dl class="info-list">
            <dt>版本</dt>
            <dd>0.1.0</dd>
            <dt>技术栈</dt>
            <dd>Tauri 2 + Vue 3 + TypeScript</dd>
            <dt>数据目录</dt>
            <dd class="path">{{ cfg ? '' : '' }}</dd>
          </dl>

          <button class="btn is-danger" @click="resetAll">恢复默认设置</button>
        </section>
      </template>

      <!-- 保存提示 -->
      <Transition name="fade">
        <div v-if="message" class="save-toast">{{ message }}</div>
      </Transition>
    </main>
  </div>
</template>

<style scoped>
.settings-root {
  display: flex;
  width: 100%;
  height: 100%;
  background: var(--surface);
  color: var(--fg);
}

/* ---------- 侧边导航 ---------- */
.settings-nav {
  flex-shrink: 0;
  width: 172px;
  padding: 12px 8px;
  border-right: 1px solid var(--border);
}

.settings-nav__title {
  padding: 6px 10px 14px;
  font-size: 13px;
  font-weight: 600;
  color: var(--fg);
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 9px;
  width: 100%;
  padding: 8px 10px;
  margin-bottom: 2px;
  color: var(--fg-secondary);
  font-size: 13px;
  text-align: left;
  background: transparent;
  border: none;
  border-radius: var(--radius-item);
  cursor: pointer;
  transition: background var(--dur-fast), color var(--dur-fast);
}

.nav-item:hover {
  color: var(--fg);
  background: var(--surface-hover);
}

.nav-item.is-active {
  color: var(--fg);
  background: var(--surface-active);
}

.nav-item__icon {
  width: 16px;
  font-size: 13px;
  text-align: center;
}

/* ---------- 内容区 ---------- */
.settings-main {
  position: relative;
  flex: 1;
  min-width: 0;
  overflow-y: auto;
  padding: 22px 26px 40px;
}

.loading {
  padding: 40px;
  color: var(--fg-tertiary);
  text-align: center;
}

.section__title {
  margin: 22px 0 6px;
  font-size: 14px;
  font-weight: 600;
  color: var(--fg);
}

.section__title:first-child {
  margin-top: 0;
}

.section__desc {
  margin: 0 0 14px;
  font-size: 12px;
  line-height: 1.6;
  color: var(--fg-secondary);
}

/* ---------- 挂载边选择器 ---------- */
.edge-picker {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 8px;
  max-width: 380px;
  margin-bottom: 8px;
}

.edge-btn {
  padding: 14px 8px;
  color: var(--fg-secondary);
  font-size: 12px;
  background: var(--surface-hover);
  border: 1px solid var(--border);
  border-radius: var(--radius-item);
  cursor: pointer;
  transition: all var(--dur-fast) var(--ease-out);
}

.edge-btn:hover {
  color: var(--fg);
  border-color: var(--border-strong);
}

.edge-btn.is-active {
  color: #fff;
  background: var(--accent);
  border-color: var(--accent);
}

/* ---------- 表单控件 ---------- */
.field {
  max-width: 420px;
  margin-bottom: 18px;
}

.field label,
.field__label {
  display: block;
  margin-bottom: 6px;
  font-size: 12px;
  color: var(--fg-secondary);
}

.field__value {
  float: right;
  color: var(--fg);
  font-variant-numeric: tabular-nums;
}

input[type='range'] {
  width: 100%;
  height: 4px;
  background: var(--surface-active);
  border-radius: 2px;
  outline: none;
  -webkit-appearance: none;
  appearance: none;
}

input[type='range']::-webkit-slider-thumb {
  width: 14px;
  height: 14px;
  background: var(--accent);
  border: 2px solid #fff;
  border-radius: 50%;
  cursor: pointer;
  -webkit-appearance: none;
  appearance: none;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.3);
}

.switch {
  display: flex;
  align-items: center;
  gap: 9px;
  margin-bottom: 12px;
  font-size: 13px;
  color: var(--fg);
  cursor: pointer;
}

.switch input {
  width: 15px;
  height: 15px;
  accent-color: var(--accent);
  cursor: pointer;
}

.chip-row {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-bottom: 16px;
}

.chip {
  padding: 5px 13px;
  color: var(--fg-secondary);
  font-size: 12px;
  background: var(--surface-hover);
  border: 1px solid var(--border);
  border-radius: 14px;
  cursor: pointer;
  transition: all var(--dur-fast);
}

.chip:hover {
  color: var(--fg);
}

.chip.is-active {
  color: #fff;
  background: var(--accent);
  border-color: var(--accent);
}

.btn {
  padding: 7px 15px;
  color: var(--fg);
  font-size: 12px;
  background: var(--surface-hover);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-item);
  cursor: pointer;
  transition: background var(--dur-fast);
}

.btn:hover {
  background: var(--surface-active);
}

.btn.is-danger {
  color: #ff6b6b;
  border-color: rgba(255, 107, 107, 0.4);
}

.btn.is-danger:hover {
  background: rgba(255, 80, 80, 0.12);
}

.hint {
  margin: 6px 0 0;
  font-size: 11px;
  line-height: 1.5;
  color: var(--fg-tertiary);
}

.hint.path {
  word-break: break-all;
}

/* ---------- 分组列表 ---------- */
.group-list {
  max-width: 520px;
  padding: 0;
  margin: 0;
  list-style: none;
}

.group-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  margin-bottom: 5px;
  font-size: 13px;
  background: var(--surface-hover);
  border: 1px solid var(--border);
  border-radius: var(--radius-item);
}

.group-row.is-active {
  border-color: var(--accent);
  background: var(--accent-dim);
}

.group-row__icon {
  font-size: 15px;
}

.group-row__name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.group-row__count {
  font-size: 11px;
  color: var(--fg-tertiary);
}

.mini-btn {
  padding: 3px 9px;
  font-size: 11px;
  color: var(--accent);
  background: transparent;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  cursor: pointer;
}

.mini-btn:hover {
  background: var(--accent-dim);
}

/* ---------- 信息列表 ---------- */
.info-list {
  display: grid;
  grid-template-columns: 88px 1fr;
  gap: 9px 14px;
  max-width: 520px;
  margin: 0 0 22px;
  font-size: 12px;
}

.info-list dt {
  color: var(--fg-tertiary);
}

.info-list dd {
  margin: 0;
  color: var(--fg);
  word-break: break-all;
}

/* ---------- 热键录制 ---------- */
.hotkey-row {
  display: flex;
  align-items: center;
  gap: 10px;
  max-width: 420px;
  margin-bottom: 8px;
}

.hotkey-input {
  display: flex;
  align-items: center;
  justify-content: center;
  flex: 1;
  gap: 5px;
  min-height: 38px;
  padding: 6px 14px;
  background: var(--surface-hover);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-item);
  cursor: pointer;
  transition: all var(--dur-fast);
}

.hotkey-input:hover {
  border-color: var(--accent);
}

.hotkey-input.is-recording {
  border-color: var(--accent);
  background: var(--accent-dim);
}

.hotkey-input kbd {
  padding: 3px 9px;
  color: var(--fg);
  font-family: inherit;
  font-size: 12px;
  background: rgba(255, 255, 255, 0.1);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  box-shadow: 0 1px 0 rgba(0, 0, 0, 0.3);
}

.hotkey-hint {
  color: var(--accent);
  font-size: 12px;
}

.err-text {
  color: #ff8a8a;
}

/* ---------- 扫描导入 ---------- */
.scan-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  max-width: 640px;
  margin: 14px 0 10px;
}

.scan-toolbar .switch {
  margin-bottom: 0;
}

.btn:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.btn.is-primary {
  color: #fff;
  background: var(--accent);
  border-color: var(--accent);
}

.btn.is-primary:hover:not(:disabled) {
  background: var(--accent-hover);
}

.scan-list {
  max-width: 640px;
  max-height: 340px;
  padding: 0;
  margin: 0;
  overflow-y: auto;
  list-style: none;
  border: 1px solid var(--border);
  border-radius: var(--radius-item);
}

.scan-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 7px 12px;
  font-size: 12px;
  cursor: pointer;
  transition: background var(--dur-fast);
}

.scan-row:hover {
  background: var(--surface-hover);
}

.scan-row.is-selected {
  background: var(--accent-dim);
}

.scan-row input {
  flex-shrink: 0;
  accent-color: var(--accent);
}

.scan-row__name {
  flex-shrink: 0;
  min-width: 120px;
  color: var(--fg);
}

.scan-row__path {
  flex: 1;
  overflow: hidden;
  color: var(--fg-tertiary);
  font-size: 11px;
  text-overflow: ellipsis;
  white-space: nowrap;
  direction: rtl;
  text-align: left;
}

/* ---------- 保存提示 ---------- */
.save-toast {
  position: sticky;
  bottom: 0;
  left: 50%;
  width: fit-content;
  padding: 6px 16px;
  margin: 0 auto;
  color: var(--fg);
  font-size: 12px;
  background: rgba(0, 0, 0, 0.72);
  border-radius: 14px;
  pointer-events: none;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity var(--dur-normal);
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
