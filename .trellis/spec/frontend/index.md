# Frontend Development Guidelines

> 管理 UI 约定（**目标栈**，脚手架落地后按真实代码修订）。
>
> **React + Vite + TypeScript + Tailwind** SPA，嵌入 Tauri WebView。
> **无登录页**；打开即用。

---

## Overview

| 能力 | 约定 |
|------|------|
| 构建 | Vite |
| UI | React 函数组件 + TypeScript |
| 样式 | Tailwind CSS |
| 服务端状态 | TanStack Query（优先） |
| 客户端状态 | Zustand（UI/本地偏好） |
| 与壳通信 | `@tauri-apps/api` invoke（网关启停/状态/路径） |
| 与网关通信 | HTTP 调用侧车管理 API（业务 CRUD） |

信息架构以渠道/分组/日志/设置为中心，**不要求**像素级复刻，**不使用** Next.js SSR。

---

## Guidelines Index

| Guide | Description | Status |
|-------|-------------|--------|
| [Directory Structure](./directory-structure.md) | 页面/组件/API 目录 | Target |
| [Component Guidelines](./component-guidelines.md) | 组件与布局 | Target |
| [Hook Guidelines](./hook-guidelines.md) | hooks 约定 | Target |
| [State Management](./state-management.md) | Query / Zustand 边界 | Target |
| [Type Safety](./type-safety.md) | TS 与 API 类型 | Target |
| [Quality Guidelines](./quality-guidelines.md) | 质量与 a11y 底线 | Target |

---

**Language**: 简体中文（代码标识符保持英文）。
