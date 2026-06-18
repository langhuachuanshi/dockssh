# DockSSH 开发约定 (agent.md)

> 本文件是 DockSSH 项目的开发规范。所有代码（含 AI 生成）必须遵循。
> 技术栈：Tauri v2 (Rust) + Vue 3 + TypeScript + Element Plus。

---

## 行为准则

以下准则偏向谨慎而非速度，目的是减少常见编码失误。

### 1. 先想清楚再动手

不要想当然。不要掩盖困惑。主动暴露取舍。
动手之前：

- 明确说出你的假设，不确定就问
- 如果有多种理解，列出来让用户选，不要自己悄悄选一个
- 如果存在更简单的方案，说出来。该反驳就反驳
- 遇到不清楚的地方，停下来。指出哪里不明白，然后问

### 2. 简单优先

用最少的代码解决问题。不要写猜测性的代码。

- 不加没要求的功能
- 一次性使用的代码不做抽象
- 没要求的"灵活性"或"可配置性"不加
- 不可能发生的场景不做错误处理
- 如果写了 200 行但 50 行就能搞定，重写
- 问自己："资深工程师会觉得这太复杂吗？" 如果是，简化

### 3. 精准修改

只动该动的。只清理自己弄乱的。
编辑已有代码时：

- 不要"顺手改进"旁边的代码、注释或格式
- 不要重构没坏的东西
- 匹配已有风格，即使你习惯不同
- 发现无关的死代码，提一句就行，别删

你的改动产生的孤立代码：

- 删除你的改动导致不再使用的 import/变量/函数
- 不要删除之前就存在的死代码，除非被要求

检验标准：每一行改动都应该能追溯到用户的需求。

### 4. 目标驱动

定义成功标准，循环直到验证通过。
把任务转化为可验证的目标：

- "加校验" → "为无效输入写测试，然后让测试通过"
- "修 bug" → "写一个能复现的测试，然后让它通过"
- "重构 X" → "确保重构前后测试都能通过"

多步骤任务，简要列个计划：

1. [步骤] → 验证：[检查点]
2. [步骤] → 验证：[检查点]
3. [步骤] → 验证：[检查点]

---

## 一、技术栈（已定，不再变更）

| 层 | 技术 |
|----|------|
| 桌面框架 | Tauri v2（单进程，禁用 Node / WebSocket / 本地端口 / 目标机安装） |
| 原生层 | Rust + russh + tokio |
| 前端 | Vue 3 + Vite + TypeScript |
| UI | **Element Plus** |
| 状态 | Pinia |
| 路由 | Vue Router (hash) |
| 终端 | xterm.js（仅容器 exec 用） |
| 图表 | ECharts（仅 stats 实时图表用） |

## 二、UI / 样式规范（核心，务必遵守）

1. **统一使用 Element Plus 官方 dark 主题**：
   - `index.html` 的 `<html>` 标签加 `class="dark"`。
   - `main.ts` 引入 `element-plus/theme-chalk/dark/css-vars.css`。
   - **禁止**自定义 `--dsh-*`、`--bg-*` 之类的设计系统变量。颜色一律走 Element Plus 的 CSS 变量（如 `var(--el-bg-color)`、`var(--el-text-color-primary)`、`var(--el-border-color)`）。

2. **不过度设计样式**：
   - 优先用 Element Plus 现成组件完成布局（`el-container` / `el-aside` / `el-header` / `el-main` / `el-card` / `el-table` / `el-form`）。
   - 组件 `<style scoped>` 只写**必要的尺寸、间距、flex 排布**，不要自己画卡片、阴影、圆角、渐变。卡片用 `el-card`，不用手写 `.card`。
   - 禁止自造按钮、标签、徽章样式——用 `el-button` / `el-tag` / `el-badge`。

3. **不引入非必要组件 / 库**：
   - 一个功能能用 Element Plus 解决，就不再加第三方库。
   - 当前已批准的额外依赖：Pinia、Vue Router、xterm.js、ECharts、Tauri API。新增任何依赖前必须在对话中确认。

4. **全局样式 `src/style.css` 保持极简**：
   - 只保留：reset、`html/body/#app` 满屏、滚动条、极少数工具类（`.flex` 等）。
   - 不要在全局写组件级样式覆盖，必要时用 `:deep()` 在组件内局部覆盖。

## 三、Rust 后端规范

1. **分层**（已建，不要打乱）：
   - `ssh/` SSH 客户端与会话池
   - `docker/` docker 命令封装与解析
   - `pty/` 终端透传
   - `crypto/` 凭据加密
   - `commands/` Tauri command 入口
   - `models.rs` 数据结构
   - `state.rs` 全局状态
   - `error.rs` 统一错误

2. **命令执行**：所有 docker 操作通过 `SshClient::exec` / `exec_stream` 走 SSH，**绝不**在目标机装 agent、开 Docker API 端口。

3. **错误**：统一用 `AppError`，command 返回 `AppResult<T>`，错误序列化成字符串回前端。

4. **凭据**：密码进系统密钥环（`keyring`），明文绝不落盘；`hosts.json` 只存非敏感元信息。

5. **构建工具链**：Windows 用 **MSVC** 目标（见 `rust-toolchain.toml`）。GNU/MinGW 会因符号导出过多报 `export ordinal too large`。

## 四、前端目录结构

```
src/
├── api/          Tauri invoke 封装 + 类型
├── store/        Pinia store
├── router/       路由
├── components/   通用组件（如 HostLayout）
└── views/        页面（按模块子目录）
```

- `api/types.ts` 的类型与 `src-tauri/src/models.rs` 严格对齐。
- 页面套用 `HostLayout` 作为二级路由父组件。

## 五、编码风格

- 中文注释，说明"为什么"而非"是什么"。
- Rust：snake_case；TS/Vue：camelCase 变量、PascalCase 组件。
- 提交前：后端 `cargo check` 0 error，前端 `npm run build` 通过。
