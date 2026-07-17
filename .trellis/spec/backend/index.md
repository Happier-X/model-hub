# Backend Development Guidelines

> 本项目后端约定（**目标栈**，脚手架落地后按真实代码修订）。
>
> 架构：**Tauri（Rust 壳）+ 可替换 LLM 网关侧车（阶段 1 优先 Go/octopus 兼容进程；阶段 2+ 可 Rust 化）**。
> 平台：Windows（MVP 仅验收 Windows）。

---

## Overview

| 层级 | 职责 | 技术 |
|------|------|------|
| 桌面壳 | 窗口、生命周期、数据目录、拉起/停止侧车、健康检查 | Tauri 2 + Rust |
| 网关侧车 | HTTP 管理 API + OpenAI 兼容转发 + SQLite | 阶段 1：外部/内嵌兼容进程；接口以 HTTP 契约为准 |
| 持久化（MVP） | 渠道/分组/日志等 | **仅 SQLite** |

管理 UI **无登录**；默认监听 **`127.0.0.1`**；对外 LLM API **本机免鉴权**（见产品 PRD D3/D6）。

---

## Guidelines Index

| Guide | Description | Status |
|-------|-------------|--------|
| [Directory Structure](./directory-structure.md) | 仓库与 Rust/侧车目录 | Target |
| [Database Guidelines](./database-guidelines.md) | SQLite、路径、迁移原则 | Target |
| [Error Handling](./error-handling.md) | 壳错误、进程失败、HTTP 错误 | Target |
| [Logging Guidelines](./logging-guidelines.md) | 日志级别与落盘 | Target |
| [Quality Guidelines](./quality-guidelines.md) | 质量门禁与禁止项 | Target |

---

## Related Product Decisions

- 路径 C 混合渐进、M1 MVP、Windows only、无管理登录、本机免鉴权：见任务 `07-17-tauri-port-octopus/prd.md`。

---

**Language**: 简体中文（代码标识符保持英文）。
