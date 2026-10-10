# 05 · API 契约

> 前后端接口约定。前端所有系统操作必须通过此契约调用 Rust 侧。

---

## 1. 命令清单 (invoke)

### 1.1 配置相关

| 命令 | 入参 | 返回 | 说明 |
|---|---|---|---|
| `get_config` | — | `Config` | 获取完整配置 |
| `update_config` | `{ patch: object }` | `Config` | 局部更新（深合并），返回新配置 |
| `reset_config` | — | `Config` | 恢复默认（会先备份） |
| `export_config` | `{ path: string }` | `void` | 导出到指定路径 |
| `import_config` | `{ path: string }` | `Config` | 从文件导入 |

### 1.2 面板窗口相关

| 命令 | 入参 | 返回 | 说明 |
|---|---|---|---|
| `set_panel_edge` | `{ edge: Edge }` | `void` | 切换挂载边（自动交换宽高并重定位） |
| `set_panel_size` | `{ length?: number, thickness?: number }` | `void` | 调整面板尺寸 |
| `set_panel_effect` | `{ mode: "quality" \| "performance" }` | `void` | 切换材质（动画期降级用） |
| `set_passthrough` | `{ enabled: boolean }` | `void` | 鼠标穿透开关 |
| `summon_panel` | — | `void` | 强制唤出面板 |
| `hide_panel` | — | `void` | 强制收起面板 |
| `get_edge_state` | — | `EdgeState` | 查询当前显隐状态 |
| `list_monitors` | — | `MonitorInfo[]` | 枚举显示器 |

### 1.3 图标项相关

| 命令 | 入参 | 返回 | 说明 |
|---|---|---|---|
| `add_icons` | `{ groupId: string, paths: string[] }` | `IconItem[]` | 批量添加（拖入时用） |
| `remove_icon` | `{ groupId: string, itemId: string }` | `void` | 删除（仅移除引用） |
| `update_icon` | `{ groupId: string, itemId: string, patch: object }` | `IconItem` | 修改名称/参数等 |
| `reorder_icons` | `{ groupId: string, orderedIds: string[] }` | `void` | 拖拽排序后提交 |
| `move_icon` | `{ fromGroupId, toGroupId, itemId, toIndex }` | `void` | 跨分组移动 |
| `launch_icon` | `{ itemId: string }` | `void` | 启动/打开 |
| `reveal_icon` | `{ itemId: string }` | `void` | 在资源管理器中显示 |
| `refresh_icon` | `{ itemId: string }` | `string` | 重新提取图标，返回新 iconPath |

### 1.4 分组相关

| 命令 | 入参 | 返回 | 说明 |
|---|---|---|---|
| `create_group` | `{ name: string, icon?: string }` | `Group` | 新建分组 |
| `rename_group` | `{ groupId: string, name: string }` | `void` | 重命名 |
| `delete_group` | `{ groupId: string, moveItemsTo?: string }` | `void` | 删除（可指定迁移目标） |
| `reorder_groups` | `{ orderedIds: string[] }` | `void` | 分组排序 |
| `set_active_group` | `{ groupId: string }` | `void` | 切换当前分组 |

### 1.5 系统相关

| 命令 | 入参 | 返回 | 说明 |
|---|---|---|---|
| `open_settings` | — | `void` | 打开设置窗口 |
| `set_autostart` | `{ enabled: boolean }` | `void` | 开机自启 |
| `pick_background` | — | `string \| null` | 打开文件选择器选背景图 |
| `pick_icon_file` | — | `string \| null` | 选择自定义图标文件 |
| `get_icon_data_url` | `{ itemId: string }` | `string` | 取图标为 data URL（供 img 用） |
| `quit_app` | — | `void` | 退出 |
| `reload_app` | — | `void` | 重载面板 |
| `get_app_info` | — | `AppInfo` | 版本等 |

### 1.6 扫描相关（P2）

| 命令 | 入参 | 返回 | 说明 |
|---|---|---|---|
| `scan_desktop` | — | `IconItem[]` | 扫描桌面图标 |
| `scan_start_menu` | — | `IconItem[]` | 扫描开始菜单 |

---

## 2. 事件通道 (emit / listen)

Rust → 前端单向推送。

| 事件名 | Payload | 触发时机 |
|---|---|---|
| `edge://summon` | `{ edge: Edge }` | 面板开始滑出 |
| `edge://shown` | — | 面板完全展开 |
| `edge://hide` | — | 面板开始收起 |
| `edge://hidden` | — | 面板完全收起（前端应清理 hover 状态） |
| `config://changed` | `Config` | 配置被修改（含来自设置窗口的修改） |
| `icons://changed` | `{ groupId: string }` | 某分组的图标列表变化 |
| `icon://invalid` | `{ itemId: string }` | 目标文件已不存在 |
| `monitor://changed` | `MonitorInfo[]` | 显示器配置变化 |
| `theme://changed` | `{ theme: "light" \| "dark" }` | 系统主题变化 |

---

## 3. TypeScript 类型定义

`src/types/index.ts`：

```ts
export type Edge = 'top' | 'bottom' | 'left' | 'right'
export type EffectKind = 'acrylic' | 'mica' | 'blur' | 'solid'
export type ThemeKind = 'system' | 'light' | 'dark'
export type BgType = 'none' | 'image' | 'color'
export type BgFit = 'cover' | 'contain' | 'fill' | 'tile' | 'center'
export type ItemType = 'app' | 'folder' | 'file' | 'url'
export type EdgeState = 'hidden' | 'showing' | 'visible' | 'hiding'

export interface PanelConfig {
  edge: Edge
  monitorIndex: number
  length: number
  thickness: number
  offset: number
  cornerRadius: number
  alwaysOnTop: boolean
  coverTaskbar: boolean
  autoSize: boolean
}

export interface EdgeConfig {
  triggerWidth: number
  hideDelay: number
  animationDuration: number
  enableHover: boolean
  disableOnFullscreen: boolean
  summonHotkey: string
  alwaysVisible: boolean
}

export interface BackgroundConfig {
  type: BgType
  imagePath: string | null
  fit: BgFit
  dim: number
  color: string
}

export interface IconStyleConfig {
  size: number
  showLabel: boolean
  labelLines: 1 | 2
  gap: number
  gridColumns: number
}

export interface AppearanceConfig {
  effect: EffectKind
  opacity: number
  theme: ThemeKind
  accentColor: string
  background: BackgroundConfig
  icons: IconStyleConfig
}

export interface IconItem {
  id: string
  type: ItemType
  name: string
  target: string
  args: string
  workDir: string
  iconPath: string | null
  sourcePath: string | null
  runAsAdmin: boolean
  order: number
  addedAt: string
}

export interface Group {
  id: string
  name: string
  icon: string
  order: number
  items: IconItem[]
}

export interface SystemConfig {
  autoStart: boolean
  startMinimized: boolean
  language: string
  checkUpdate: boolean
}

export interface Config {
  version: number
  panel: PanelConfig
  edge: EdgeConfig
  appearance: AppearanceConfig
  groups: Group[]
  activeGroupId: string | null
  system: SystemConfig
}

export interface MonitorInfo {
  index: number
  name: string
  x: number
  y: number
  width: number
  height: number
  workX: number
  workY: number
  workWidth: number
  workHeight: number
  isPrimary: boolean
  scaleFactor: number
}

export interface AppInfo {
  version: string
  tauriVersion: string
  osVersion: string
  dataDir: string
}
```

---

## 4. 前端 IPC 封装

`src/composables/useTauri.ts`：

```ts
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { Config, Edge, IconItem, MonitorInfo, EdgeState } from '@/types'

/** 统一错误处理：Rust 侧 Result::Err 会 reject */
async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args)
  } catch (e) {
    console.error(`[IPC] ${cmd} failed:`, e)
    throw new Error(typeof e === 'string' ? e : `命令 ${cmd} 执行失败`)
  }
}

export const api = {
  // --- 配置 ---
  getConfig: () => call<Config>('get_config'),
  updateConfig: (patch: DeepPartial<Config>) => call<Config>('update_config', { patch }),
  resetConfig: () => call<Config>('reset_config'),

  // --- 面板 ---
  setPanelEdge: (edge: Edge) => call<void>('set_panel_edge', { edge }),
  setPanelSize: (p: { length?: number; thickness?: number }) =>
    call<void>('set_panel_size', p),
  setPanelEffect: (mode: 'quality' | 'performance') =>
    call<void>('set_panel_effect', { mode }),
  setPassthrough: (enabled: boolean) => call<void>('set_passthrough', { enabled }),
  summonPanel: () => call<void>('summon_panel'),
  hidePanel: () => call<void>('hide_panel'),
  getEdgeState: () => call<EdgeState>('get_edge_state'),
  listMonitors: () => call<MonitorInfo[]>('list_monitors'),

  // --- 图标 ---
  addIcons: (groupId: string, paths: string[]) =>
    call<IconItem[]>('add_icons', { groupId, paths }),
  removeIcon: (groupId: string, itemId: string) =>
    call<void>('remove_icon', { groupId, itemId }),
  updateIcon: (groupId: string, itemId: string, patch: Partial<IconItem>) =>
    call<IconItem>('update_icon', { groupId, itemId, patch }),
  reorderIcons: (groupId: string, orderedIds: string[]) =>
    call<void>('reorder_icons', { groupId, orderedIds }),
  launchIcon: (itemId: string) => call<void>('launch_icon', { itemId }),
  revealIcon: (itemId: string) => call<void>('reveal_icon', { itemId }),

  // --- 分组 ---
  createGroup: (name: string, icon?: string) =>
    call<Group>('create_group', { name, icon }),
  renameGroup: (groupId: string, name: string) =>
    call<void>('rename_group', { groupId, name }),
  deleteGroup: (groupId: string, moveItemsTo?: string) =>
    call<void>('delete_group', { groupId, moveItemsTo }),
  setActiveGroup: (groupId: string) =>
    call<void>('set_active_group', { groupId }),

  // --- 系统 ---
  openSettings: () => call<void>('open_settings'),
  setAutostart: (enabled: boolean) => call<void>('set_autostart', { enabled }),
  pickBackground: () => call<string | null>('pick_background'),
  getIconDataUrl: (itemId: string) => call<string>('get_icon_data_url', { itemId }),
  quitApp: () => call<void>('quit_app'),
  getAppInfo: () => call<AppInfo>('get_app_info'),
}

export function on<T>(event: string, handler: (payload: T) => void): Promise<UnlistenFn> {
  return listen<T>(event, (e) => handler(e.payload))
}

type DeepPartial<T> = { [K in keyof T]?: T[K] extends object ? DeepPartial<T[K]> : T[K] }
```

---

## 5. 错误约定

Rust 侧所有命令返回 `Result<T, String>`，错误信息为**面向用户的中文描述**：

```rust
#[tauri::command]
async fn add_icons(paths: Vec<String>) -> Result<Vec<IconItem>, String> {
    if paths.is_empty() {
        return Err("未选择任何文件".into());
    }
    // ...
    .map_err(|e| format!("添加失败：{}", e))
}
```

**错误分类**：

| 前缀 | 含义 | 前端处理 |
|---|---|---|
| `未...` / `请...` | 用户操作问题 | 轻提示 |
| `添加失败：` / `启动失败：` | 操作失败 | 错误提示 |
| `配置损坏：` | 配置问题 | 弹窗 + 引导恢复 |

---

## 6. 调用时序示例

### 6.0 ⚠️ 陷阱：前端资源未更新导致"假故障"

**现象**：修改前端代码后重新构建，运行 exe 却看到旧界面，
甚至出现 `state not managed for field 'state'` 这类**误导性错误**。

**根因**：Tauri 把前端 `dist/` 资源**嵌入 exe**。
若构建流程不完整（例如只跑了 `cargo build` 而没跑 `tauri build`），
exe 内嵌的仍是上一次的前端资源，
于是新后端 + 旧前端组合，产生看似"状态未注册"的报错。

**排查方法**（按顺序）：

```powershell
# 1. 确认 dist 产物时间戳
Get-ChildItem dist -Recurse -File | Select-Object Name,LastWriteTime

# 2. 确认 exe 时间戳（应晚于 dist）
Get-Item artifacts\_cargo-target\debug\acrylic-desk.exe | Select-Object LastWriteTime

# 3. 确认后端启动日志
Get-Content artifacts\_cargo-target\debug\startup.log
```

**判断依据**：

| 对比结果 | 结论 |
|---|---|
| exe 时间 < dist 时间 | ❌ exe 未嵌入最新前端，需重新 `tauri build` |
| exe 时间 > dist 时间 | ✅ 资源已更新 |

**预防措施**：
- 始终用 `tauri build` 而非 `cargo build` 产出可运行的 exe
- 后端启动日志写入 `startup.log`（见 `lib.rs::log_startup`），
  用于区分"后端问题"与"前端资源陈旧问题"

### 6.1 拖入文件添加图标

```
前端                          后端
  │                             │
  ├─ drop 事件取得路径 ─────────▶│
  │  invoke('add_icons')        │
  │                             ├─ 判断每个路径类型
  │                             ├─ 解析 .lnk
  │                             ├─ 提取图标 → PNG 落盘
  │                             ├─ 写入 config (mark_dirty)
  │                             │
  │◀── IconItem[] ──────────────┤
  │                             │
  │  更新本地 store             │
```

### 6.2 碰边唤出

```
边缘监测线程          主线程/窗口            前端
     │                    │                  │
     ├ 检测到鼠标在触发区  │                  │
     ├─ 状态 Hidden→Showing                  │
     ├─ emit('edge://summon') ──────────────▶│
     ├─ set_ignore_cursor_events(false)      │
     ├─ set_effects(Blur)                    │
     ├─ 启动位移动画 ─────▶│                  │
     │                    │ (逐帧 set_position)
     │                    │                  │
     ├─ 动画结束           │                  │
     ├─ 状态 Showing→Visible                 │
     ├─ set_effects(Acrylic)                 │
     ├─ emit('edge://shown') ───────────────▶│
     │                    │                  │
     ├─ 启动离开检测       │                  │
```

### 6.3 修改设置后实时生效

```
设置窗口                  Rust               面板窗口
   │                       │                    │
   ├─ updateConfig(patch)─▶│                    │
   │                       ├─ 深合并            │
   │                       ├─ mark_dirty        │
   │                       ├─ 应用面板相关变更   │
   │                       │  (尺寸/边/材质)     │
   │                       ├─ emit('config://changed')
   │◀── Config ────────────┤                    │
   │                       │                    │
   │                       ├───────────────────▶│
   │                       │   (前端响应式更新)  │
```
