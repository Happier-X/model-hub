# Backend Development Guidelines

> 本项目后端约定（**Vue3 重写后目标栈**）。
>
> 架构：**Tauri 2（Rust 壳 + 进程内本地代理）**，管理面 IPC，客户端面本机 HTTP。
> 平台：Windows（MVP 仅验收 Windows）。

---

## Overview

| 层级 | 职责 | 技术 |
|------|------|------|
| 桌面壳 | 窗口、生命周期、数据目录、托盘、代理启停 | Tauri 2 + Rust |
| 内嵌代理 | OpenAI 兼容 `/v1/*`、顺序故障转移、SQLite | `src-tauri/src/proxy` + `domain` + `db` |
| 持久化（MVP） | 供应商/分组/请求日志 | **仅 SQLite**（当前 schema 不执行旧数据自动迁移；无 `api_keys` 表） |

- 管理 UI **无登录**；数据经 **Tauri commands**。
- 默认监听 **`127.0.0.1`**。
- 客户端 `/v1/*` **不校验**客户端 API Key（有无 `Authorization` 均放行）。
- 代理始终作为 Tauri 进程内模块运行；管理面不暴露 HTTP CRUD 契约。

---

## Guidelines Index

| Guide | Description | Status |
|-------|-------------|--------|
| [Directory Structure](./directory-structure.md) | 仓库与 Rust 模块 | Current |
| [Database Guidelines](./database-guidelines.md) | SQLite、迁移 | Current |
| [Error Handling](./error-handling.md) | 壳错误与 HTTP 错误 | Current |
| [Logging Guidelines](./logging-guidelines.md) | 日志与脱敏 | Current |
| [Quality Guidelines](./quality-guidelines.md) | 质量门禁 | Current |
| [Model Leaderboard](./model-leaderboard.md) | OpenRouter 榜单 IPC / 缓存 / 白名单 | Current |
| [Upstream Access](./upstream-access.md) | 禁止用户上游测活；允许真实 Chat 与点击拉模型 | Current |
| [Desktop Overlay](./desktop-overlay.md) | 桌面悬浮状态条窗口、IPC 命令、配置字段与位置钳制 | Current |

---

## Related Product Decisions

- Vue3 重写 + 顺序故障转移：任务 `07-21-vue3-rewrite-api-gateway`。
- 取消供应商熔断与 `auto_failover`，错误即换源：任务 `07-23-fix-model-failover`。

---

**Language**: 简体中文（代码标识符保持英文）。
