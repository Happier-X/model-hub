# MVP 工程脚手架

## Goal

在空仓初始化 **Windows 可用** 的 Tauri 2 + React/Vite/TypeScript/Tailwind 工程，建立应用数据目录路径契约与基础主窗口，为后续网关侧车与管理 UI 子任务提供可运行底座。

## Parent

- 父任务：`07-17-tauri-port-octopus`
- 本任务对应父任务 Child Map 第 1 项：`mvp-scaffold`

## Background

- 仓库此前仅有 Trellis / AGENTS；业务代码从本任务开始。
- 产品决策已定：Windows only、React+Vite+TS+Tailwind、无登录、壳与侧车分离。
- 本任务 **不** 集成网关侧车进程， **不** 实现渠道/分组业务页。

## Requirements

### R1. 工程初始化

- 初始化 Tauri 2 桌面应用 + Vite React TypeScript 前端。
- 集成 Tailwind CSS。
- 包管理优先 **pnpm**（环境已有）。
- 应用标识与窗口标题可用 `Model Hub`（或等价产品名，与 `tauri.conf` identifier 一致即可）。

### R2. 目录与路径契约

- 按 spec 建立可扩展布局：`src/` 前端、`src-tauri/` 壳。
- Rust 侧提供 `paths` 能力：解析 app data 根目录，并定义子目录约定（`config/`、`gateway/`、`bin/`），启动时确保可创建。
- 暴露 `get_paths`（或等价）invoke，返回路径字符串供后续设置页使用。

### R3. 最小 UI 壳

- 无登录；打开即主界面。
- 简体中文占位：侧栏结构（仪表盘/渠道/分组/日志/设置）+ 网关状态条占位（固定显示「未集成」或 idle）。
- 设置区或首页展示 `get_paths` 返回的数据目录（验证路径契约）。

### R4. 开发文档

- 根目录 `README.md`：如何安装依赖、Windows 开发前置、启动 `tauri dev`、致谢 octopus 与 AGPL 提示（简短）。
- `.gitignore` 覆盖 node、target、dist、env 等。

## Out of Scope

- 网关侧车下载/启停/health
- 管理 API 对接与业务 CRUD
- OpenAI 转发、SQLite 业务库
- macOS/Linux 打包验收
- 系统托盘、自动更新

## Acceptance Criteria

- [x] AC1：Windows 上可执行开发启动（`pnpm tauri dev --no-watch`），出现应用窗口；并通过 release exe 冒烟。
- [x] AC2：前端为 React+Vite+TS，并已配置 Tailwind；`pnpm build` 通过。
- [x] AC3：`src-tauri` 的 `cargo check`、`cargo test`、`cargo fmt --check` 通过。
- [x] AC4：`get_paths` 返回 app data 根及 `config`/`gateway`/`bin` 子路径；UI 可见路径；Rust 契约测试通过。
- [x] AC5：无登录页；占位导航为简体中文。
- [x] AC6：README 含启动说明与源项目致谢/许可证提示。

## Dependencies

- 无代码依赖其他子任务。
- 规范依赖：`.trellis/spec/backend/*`、`.trellis/spec/frontend/*`。
