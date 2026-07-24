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
│   ├── HomePage.vue      # 代理运行状态、Base URL、统计、接入指引
│   ├── ProvidersPage.vue
│   ├── GroupsPage.vue
│   ├── LogsPage.vue
│   └── SettingsPage.vue  # 端口、数据目录、应用更新、自动检查偏好
└── router/
    └── index.ts
```

## 规则

1. 业务数据只经 `src/api/tauri.ts` 的 Tauri invoke 封装读写，不直连本机 HTTP 管理接口。
2. 页面使用 Vue 3 `<script setup lang="ts">`，并覆盖加载、错误和空数据状态。
3. 通用展示组件放 `components/`；有复用价值的异步状态编排放 `composables/`。
4. 外部客户端 HTTP `/v1/*` 不由前端调用或代理。
5. 无客户端 API Key 管理页；供应商上游 Key 仅在供应商表单中编辑。
