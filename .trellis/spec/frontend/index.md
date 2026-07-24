# Frontend Development Guidelines

> 管理 UI 约定（**Vue 3 重写后**）。
>
> **Vue 3 + Vite + TypeScript + happier-ui + Tailwind** SPA，嵌入 Tauri WebView。
> **无登录页**；打开即用。

---

## Overview

| 能力 | 约定 |
|------|------|
| 构建 | Vite |
| UI | Vue 3 SFC + TypeScript |
| 组件库 | **happier-ui**（npm；可映射控件优先 `H*`） |
| 表单状态 | **@tanstack/vue-form**（对话框业务表单）+ H* 控件 |
| 样式 | happier-ui tokens/style + **Tailwind**（布局/表格/select/textarea/侧栏） |
| 路由 | vue-router |
| 与壳通信 | `@tauri-apps/api` **invoke only**（代理启停 + 业务 CRUD） |
| 与客户端代理 | 不走前端；外部工具直连本机 `/v1/*` |

信息架构：首页、供应商、分组（故障转移队列）、日志。

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
| [分组队列模型排序](./model-queue-sort.md) | 本地/外部混合排序与匹配 | |
| [上游访问（backend）](../backend/upstream-access.md) | 禁止测活；拉取模型仅点击 | |

---

**Language**: 简体中文（代码标识符保持英文）。
