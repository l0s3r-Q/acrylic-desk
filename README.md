# AcrylicDesk

> Windows 桌面管理软件 —— 亚克力全透明 · 贴边吸附 · 边缘唤出

把桌面图标收纳进一块**贴靠在屏幕边缘、平时完全隐形、鼠标碰边即滑出**的
亚克力透明面板中，让桌面保持干净，同时保证常用程序触手可及。

---

## 特性

- 🪟 **亚克力全透明** —— 使用系统原生 Acrylic 材质，可透视后方窗口
- 📌 **四边可挂** —— 上 / 下 / 左 / 右任意位置，一键切换
- 🖱️ **碰边唤出** —— 鼠标碰到屏幕边缘自动滑出，移开自动收起
- 👻 **完全隐形** —— 收起后完全移出屏幕，不占用任何视觉空间
- 🖼️ **背景自选** —— 支持本地图片，五种填充模式 + 暗度调节
- 🎯 **图标管理** —— 拖拽排序、分组切换、自动提取程序图标
- 🚀 **轻量** —— 单文件 exe，内存占用 ~35 MB

---

## 快速开始

### 环境要求

| 组件 | 版本 |
|---|---|
| Windows | 10 2004+ / 11 |
| WebView2 Runtime | 任意（Win11 内置） |
| Rust | ≥ 1.77 |
| Node.js | ≥ 18 |

### 开发

```powershell
# 安装依赖
npm install

# 开发模式（热重载）
npm run tauri:dev

# 构建发布版
npm run tauri:build

# 构建绿色版（单文件，不打包安装器）
npm run tauri:build:portable
```

### 环境变量（本项目约定）

为规避 Smart App Control 对临时目录的限制，建议设置：

```powershell
$env:CARGO_TARGET_DIR = "$PWD\artifacts\_cargo-target"
$env:TMP  = "$PWD\artifacts\_tmp"
$env:TEMP = "$PWD\artifacts\_tmp"
```

详见 [docs/07-环境搭建指南.md](docs/07-环境搭建指南.md)。

---

## 使用说明

### 唤出面板

把鼠标移到屏幕边缘（默认右侧），面板会自动滑出。
鼠标移开后约 0.4 秒自动收起。

> 如果唤不出来，到 **设置 → 边缘行为 → 触发区宽度** 调大数值。

### 添加图标

把 `.exe`、`.lnk`、文件夹或文件**拖进面板**即可。
图标会自动提取，`.lnk` 会解析出真实目标。

### 打开设置

- 点击面板右上角的齿轮图标
- 或右键托盘图标 → 设置

### 调整位置

**设置 → 面板 → 面板位置**，四选一，立即生效。

---

## 数据存储

采用**便携式存储**，所有数据在 exe 同级的 `data/` 目录：

```
data/
├── config.json     配置
├── icons/          提取的图标缓存
└── backups/        配置自动备份（保留最近 5 份）
```

若 exe 所在目录不可写（如放在 `Program Files`），
自动回退到 `%APPDATA%\AcrylicDesk\`。

**卸载**：关闭程序，删除整个文件夹即可，不残留任何系统痕迹。

---

## 项目结构

```
AcrylicDesk/
├── docs/              设计文档（10 份）
├── src/               前端源码（Vue 3 + TS）
│   ├── components/    组件
│   ├── views/         视图（面板 / 设置）
│   ├── stores/        Pinia 状态
│   ├── composables/   IPC 封装
│   └── styles/        样式与设计变量
├── src-tauri/         Rust 后端
│   └── src/
│       ├── window/    窗口与材质
│       ├── edge/      边缘吸附与唤出（核心）
│       ├── icon/      图标提取与启动
│       ├── config/    配置持久化
│       └── commands/  Tauri 命令层
└── assets/            资源
```

---

## 技术栈

| 层 | 技术 |
|---|---|
| 前端 | Vue 3 + TypeScript + Vite |
| 状态 | Pinia |
| 后端 | Rust + Tauri 2 |
| 系统 API | windows-rs (Win32) |
| 材质 | 系统 Acrylic（Tauri `WindowEffect`） |

---

## 设计文档

完整设计文档见 [`docs/`](docs/README.md)：

| 文档 | 内容 |
|---|---|
| [技术选型报告](docs/00-技术选型报告.md) | 为什么是 Tauri，杀软拦截分析 |
| [产品需求规格](docs/01-产品需求规格.md) | 功能需求与验收标准 |
| [系统架构设计](docs/02-系统架构设计.md) | 分层架构与模块划分 |
| [窗体与亚克力方案](docs/03-窗体与亚克力技术方案.md) | 核心难点与 14 个踩坑记录 |
| [数据模型](docs/04-数据模型与持久化.md) | 配置 Schema 与图标缓存 |
| [API 契约](docs/05-API契约.md) | 前后端接口定义 |
| [开发计划](docs/06-开发计划与里程碑.md) | 阶段划分与进度 |
| [环境搭建](docs/07-环境搭建指南.md) | 依赖安装与故障排查 |
| [分发与防误报](docs/08-分发与防误报指南.md) | 打包策略与杀软白名单 |

---

## 已知限制

- 仅支持 Windows
- 需要系统安装 WebView2（Win10 需手动装，Win11 内置）
- 亚克力效果在 Windows 10 上会降级为 Blur

---

## 许可证

MIT
