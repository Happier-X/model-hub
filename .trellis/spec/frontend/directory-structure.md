# Directory Structure

> 前端 SPA 目录。

---

## Overview

```
src/
├── main.tsx
├── App.tsx                     # 无登录单页壳与占位内容
├── index.css                   # Tailwind 入口与全局样式
├── vite-env.d.ts
├── api/
│   └── tauri.ts                # get_paths 封装与浏览器预览降级
└── components/
    └── layout/
        ├── Sidebar.tsx         # 中文导航与网关状态占位
        └── StatusBar.tsx       # 全局状态条
```

后续业务页面放入 `routes/` 或 `pages/`，领域状态按需增加 `features/`、`hooks/`、`stores/`；通用组件放入 `components/ui/`。

---

## Rules

1. **业务请求走 `api/`**，不在组件内散落裸 `fetch` URL 字符串（可集中 baseURL）。
2. **Tauri 专用**放 `lib/tauri.ts` 或 `api/tauri.ts`，浏览器纯 dev 时可 mock。
3. **路由**：MVP 扁平：渠道、分组、日志、设置；（可选）仪表盘占位。
4. **无** `login` / `auth` 页面与会话存储（产品 D3）。

---

## Anti-Patterns

- 把侧车 Base URL 写死为生产端口且无法在设置中展示。
- 在组件里直接 `import` 操作系统路径 API 而不经 Tauri 封装。
