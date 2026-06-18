# DockSSH Makefile —— 调试与构建的便捷入口
#
# Windows 用户请使用 Git Bash / MSYS2 / WSL 运行 make（cmd 无 make）。
# 首次使用前请确认已装好 Node.js、Rust(MSVC) 与 Tauri v2 系统依赖。

.DEFAULT_GOAL := help

TAURI_DIR := src-tauri
NPM       := npm
CARGO     := cargo

.PHONY: help
help: ## 显示所有可用命令
	@echo "DockSSH Makefile —— 用法: make <target>"
	@echo ""
	@echo "命令列表:"
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-16s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

# ---------- 依赖 ----------

.PHONY: install
install: ## 安装前端依赖
	$(NPM) install

.PHONY: update
update: ## 更新前端依赖
	$(NPM) update

# ---------- 开发 ----------

.PHONY: dev
dev: ## 启动桌面应用（开发模式，前端热更新）
	$(NPM) run tauri dev

.PHONY: dev-web
dev-web: ## 仅启动前端 Vite 服务（浏览器调试 UI，无 SSH/Docker 后端）
	$(NPM) run dev

# ---------- 构建 ----------

.PHONY: build
build: ## 构建桌面应用安装包（产物: src-tauri/target/release/bundle）
	$(NPM) run tauri build

.PHONY: build-web
build-web: ## 构建前端（vue-tsc 类型检查 + Vite 打包到 dist）
	$(NPM) run build

# ---------- 检查 ----------

.PHONY: check
check: ## Rust 编译检查（cargo check）
	$(CARGO) check --manifest-path $(TAURI_DIR)/Cargo.toml

.PHONY: clippy
clippy: ## Rust 静态检查（cargo clippy）
	$(CARGO) clippy --manifest-path $(TAURI_DIR)/Cargo.toml --all-targets

.PHONY: typecheck
typecheck: ## 前端类型检查（vue-tsc，不打包）
	npx vue-tsc -b

# 提交前标准（agent.md）：后端 cargo check 0 error，前端 npm run build 通过
.PHONY: verify
verify: check build-web ## 提交前验证（Rust check + 前端 build，须全部通过）
	@echo "==> 所有检查通过，可以提交"

# ---------- 清理 ----------

.PHONY: clean-web
clean-web: ## 清理前端构建产物（dist、Vite 缓存）
	@rm -rf dist node_modules/.vite

.PHONY: clean-rust
clean-rust: ## 清理 Rust 构建产物（target）
	$(CARGO) clean --manifest-path $(TAURI_DIR)/Cargo.toml

.PHONY: clean
clean: clean-web clean-rust ## 清理所有构建产物
