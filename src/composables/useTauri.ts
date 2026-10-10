/**
 * Tauri IPC 封装
 * 契约见 docs/05-API契约.md §4
 */

import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
  AppInfo,
  Config,
  Edge,
  EdgeState,
  Group,
  IconItem,
  MonitorInfo,
  ScanEntry,
} from '@/types'

type DeepPartial<T> = {
  [K in keyof T]?: T[K] extends object ? DeepPartial<T[K]> : T[K]
}

/** 统一错误处理 */
async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args)
  } catch (e) {
    const msg = typeof e === 'string' ? e : `命令 ${cmd} 执行失败`
    console.error(`[IPC] ${cmd}:`, e)
    throw new Error(msg)
  }
}

export const api = {
  // ---------- 配置 ----------
  getConfig: () => call<Config>('get_config'),

  updateConfig: (patch: DeepPartial<Config>) =>
    call<Config>('update_config', { patch }),

  resetConfig: () => call<Config>('reset_config'),

  // ---------- 面板 ----------
  setPanelEdge: (edge: Edge) => call<void>('set_panel_edge', { edge }),

  setPanelSize: (p: { length?: number; thickness?: number }) =>
    call<void>('set_panel_size', p),

  setPanelEffect: (mode: 'quality' | 'performance') =>
    call<void>('set_panel_effect', { mode }),

  setPassthrough: (enabled: boolean) =>
    call<void>('set_passthrough', { enabled }),

  summonPanel: () => call<void>('summon_panel'),

  hidePanel: () => call<void>('hide_panel'),

  getEdgeState: () => call<EdgeState>('get_edge_state'),

  setDragging: (dragging: boolean) =>
    call<void>('set_dragging', { dragging }),

  listMonitors: () => call<MonitorInfo[]>('list_monitors'),

  // ---------- 图标 ----------
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

  /** 取图标为 data URL（WebView2 不允许 file:// 加载本地图标） */
  getIconDataUrl: (itemId: string) =>
    call<string>('get_icon_data_url', { itemId }),

  /** 重新提取图标（目标文件变化后） */
  refreshIcon: (itemId: string) =>
    call<string>('refresh_icon', { itemId }),

  /** 扫描桌面图标 */
  scanDesktop: () => call<ScanEntry[]>('scan_desktop'),

  /** 扫描开始菜单 */
  scanStartMenu: () => call<ScanEntry[]>('scan_start_menu'),

  /** 校验热键字符串 */
  validateHotkey: (spec: string) =>
    call<void>('validate_hotkey', { spec }),

  /** 应用新热键 */
  applyHotkey: (spec: string, enabled: boolean) =>
    call<void>('apply_hotkey', { spec, enabled }),

  // ---------- 分组 ----------
  createGroup: (name: string, icon?: string) =>
    call<Group>('create_group', { name, icon }),

  renameGroup: (groupId: string, name: string) =>
    call<void>('rename_group', { groupId, name }),

  deleteGroup: (groupId: string, moveItemsTo?: string) =>
    call<void>('delete_group', { groupId, moveItemsTo }),

  setActiveGroup: (groupId: string) =>
    call<void>('set_active_group', { groupId }),

  // ---------- 系统 ----------
  openSettings: () => call<void>('open_settings'),

  /** 打开文件选择器挑选背景图，返回路径或 null */
  pickBackground: () => call<string | null>('pick_background'),

  getAppInfo: () => call<AppInfo>('get_app_info'),

  quitApp: () => call<void>('quit_app'),
}

/** 订阅后端事件 */
export function on<T>(event: string, handler: (payload: T) => void): Promise<UnlistenFn> {
  return listen<T>(event, (e) => handler(e.payload))
}
