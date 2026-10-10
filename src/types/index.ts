/**
 * 与 Rust 侧严格对应的类型定义
 * 见 docs/05-API契约.md §3
 */

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
  hotkeyEnabled: boolean
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

/** 扫描结果项（桌面 / 开始菜单） */
export interface ScanEntry {
  name: string
  path: string
  kind: string
}

/** 挂载边的中文名称 */
export const EDGE_LABELS: Record<Edge, string> = {
  top: '顶部',
  bottom: '底部',
  left: '左侧',
  right: '右侧',
}

/** 材质的中文名称 */
export const EFFECT_LABELS: Record<EffectKind, string> = {
  acrylic: '亚克力',
  mica: '云母',
  blur: '模糊',
  solid: '纯色',
}

/** 背景适配的中文名称 */
export const BG_FIT_LABELS: Record<BgFit, string> = {
  cover: '填满',
  contain: '适应',
  fill: '拉伸',
  tile: '平铺',
  center: '居中',
}
