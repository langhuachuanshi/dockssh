# DockSSH

> 零安装的 Docker 可视化管理工具 —— 通过 SSH 远程管理目标主机上的 Docker，无需在目标机安装 agent、无需开放 Docker API 端口。

## 简介

DockSSH 是一个基于 Tauri v2 的跨平台桌面应用，用纯 SSH 通道执行 docker 命令来完成容器的可视化管理（容器列表、日志、镜像、实时监控图表、容器终端等）。凭据使用系统密钥环加密保存，明文绝不落盘。

## 技术栈

| 层 | 技术 |
|----|------|
| 桌面框架 | Tauri v2（单进程） |
| 原生层 | Rust + russh + tokio |
| 前端 | Vue 3 + Vite + TypeScript |
| UI | Element Plus（官方 dark 主题） |
| 状态 / 路由 | Pinia / Vue Router (hash) |
| 终端 | xterm.js（容器 exec） |
| 图表 | ECharts（实时 stats） |

## 环境要求

- [Node.js](https://nodejs.org/)（建议 LTS）
- [Rust](https://www.rust-lang.org/) stable（`rustup default stable`）
- Tauri v2 系统依赖：参见 [Tauri Prerequisites](https://tauri.app/start/prerequisites/)
  - **Windows**：必须使用 **MSVC** 工具链（项目已通过 `src-tauri/rust-toolchain.toml` 锁定）。不要用 GNU/MinGW，否则会触发 `export ordinal too large` 链接错误。
  - 需安装 Microsoft C++ Build Tools 与 WebView2。

## 安装依赖

```bash
# 前端依赖
npm install
```

Rust 依赖由 cargo 在首次构建时自动拉取，无需手动安装。

## 启动开发

```bash
npm run tauri dev
```

该命令会自动：
1. 启动 Vite 前端开发服务（`http://localhost:5173`）
2. 编译 Rust 后端并打开桌面窗口（支持前端热更新）

如果只想调试前端 UI（不涉及 SSH / Docker 后端能力），可单独运行：

```bash
npm run dev
```

## 构建生产包

```bash
npm run tauri build
```

产物位于 `src-tauri/target/release/bundle/` 下（Windows 为 `.msi` / `.exe` 安装包）。

## 常用脚本

| 命令 | 说明 |
|------|------|
| `npm run dev` | 仅启动 Vite 前端（浏览器调试 UI） |
| `npm run build` | 类型检查 + 前端打包到 `dist/` |
| `npm run preview` | 预览前端构建产物 |
| `npm run tauri dev` | 启动完整桌面应用（开发模式） |
| `npm run tauri build` | 构建桌面安装包 |

## 目录结构

```
dockssh/
├── src/                  # 前端源码（Vue 3）
│   ├── api/              # Tauri invoke 封装与类型
│   ├── components/       # 通用组件（标题栏、侧边栏等）
│   ├── views/            # 页面（hosts / containers / images / dashboard）
│   ├── store/            # Pinia store
│   └── router/           # 路由
├── src-tauri/            # Rust 后端源码
│   ├── src/
│   │   ├── ssh/          # SSH 客户端与会话池
│   │   ├── docker/       # docker 命令封装与解析
│   │   ├── pty/          # 终端透传
│   │   ├── crypto/       # 凭据加密
│   │   ├── commands/     # Tauri command 入口
│   │   ├── models.rs     # 数据结构
│   │   └── state.rs      # 全局状态
│   ├── Cargo.toml
│   └── tauri.conf.json
├── agent.md              # 开发规范（提交前必读）
└── package.json
```

## 开发规范

详见 [agent.md](./agent.md)。要点：
- UI 统一使用 Element Plus 官方组件与 dark 主题，不自造设计系统变量。
- 所有 docker 操作通过 SSH 执行，绝不在目标机安装 agent。
- 提交前：后端 `cargo check` 0 error，前端 `npm run build` 通过。

## License

MIT
