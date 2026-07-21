# Backend Development Guidelines

> 本项目后端约定（**目标栈**，脚手架落地后按真实代码修订）。
>
> 架构：**Tauri（Rust 壳）+ Rust 原生网关侧车（gateway-rust / model-hub-gateway）**。
> 平台：Windows（MVP 仅验收 Windows）。

---

## Overview

| 层级 | 职责 | 技术 |
|------|------|------|
| 桌面壳 | 窗口、生命周期、数据目录、拉起/停止侧车、健康检查 | Tauri 2 + Rust |
| 网关侧车 | HTTP 管理 API + OpenAI 兼容转发 + SQLite | gateway-rust；接口以 HTTP 契约为准 |
| 持久化（MVP） | 渠道/分组/日志等 | **仅 SQLite** |

管理 UI **无登录**；默认监听 **`127.0.0.1`**；管理 API 用 JWT，客户端 `/v1/*` 须使用网关签发的 API Key（`sk-modelhub-...`）。默认网关实现为 `gateway-rust`。

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

- 路径 C 混合渐进、M1 MVP、Windows only、无管理登录：见任务 `07-17-tauri-port-octopus/prd.md`。
- 客户端网关 API Key 与鉴权闭环：见任务 `07-18-gateway-api-key-chat/prd.md`。

---

**Language**: 简体中文（代码标识符保持英文）。
