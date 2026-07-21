# Frontend Development Guidelines

> 管理 UI 约定（**Vue 3 重写后**）。
>
> **Vue 3 + Vite + TypeScript + Tailwind** SPA，嵌入 Tauri WebView。
> **无登录页**；打开即用。

---

## Overview

| 能力 | 约定 |
|------|------|
| 构建 | Vite |
| UI | Vue 3 SFC + TypeScript |
| 样式 | Tailwind CSS |
| 路由 | vue-router |
| 与壳通信 | `@tauri-apps/api` **invoke only**（代理启停 + 业务 CRUD） |
| 与客户端代理 | 不走前端；外部工具直连本机 `/v1/*` |

信息架构：概览、供应商、分组（故障转移队列）、API Key、日志。

---

## Guidelines Index

| Guide | Description |
|-------|-------------|
| [Directory Structure](./directory-structure.md) | 页面/组件/API |
| [Component Guidelines](./component-guidelines.md) | 组件与布局 |
| [组合式函数规范](./hook-guidelines.md) | Vue 组合式函数与生命周期 |
| [State Management](./state-management.md) | 本地状态约定 |
| [Type Safety](./type-safety.md) | TS 与 invoke 类型 |
| [Quality Guidelines](./quality-guidelines.md) | 质量底线 |

---

**Language**: 简体中文（代码标识符保持英文）。
