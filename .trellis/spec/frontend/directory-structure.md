# 前端目录结构

```text
src/
├── main.ts
├── App.vue
├── index.css
├── api/
│   └── tauri.ts          # invoke 封装与跨层类型
├── components/
│   └── AppShell.vue
├── composables/          # 仅在确有复用时创建
├── pages/
│   ├── OverviewPage.vue  # 代理状态、Base URL、端口
│   ├── ProvidersPage.vue
│   ├── GroupsPage.vue
│   ├── ApiKeysPage.vue
│   └── LogsPage.vue
└── router/
    └── index.ts
```

## 规则

1. 业务数据只经 `src/api/tauri.ts` 的 Tauri invoke 封装读写，不直连本机 HTTP 管理接口。
2. 客户端 Key 创建后只展示一次明文；列表仅展示脱敏值。
3. 页面使用 Vue 3 `<script setup lang="ts">`，并覆盖加载、错误和空数据状态。
4. 通用展示组件放 `components/`；有复用价值的异步状态编排放 `composables/`。
5. 外部客户端 HTTP `/v1/*` 不由前端调用或代理。
