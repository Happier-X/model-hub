# Directory Structure（Frontend）

```
src/
├── main.ts
├── App.vue
├── index.css
├── api/tauri.ts          # invoke 封装与类型
├── components/AppShell.vue
├── pages/
│   ├── OverviewPage.vue  # 代理状态 / Base URL / 端口
│   ├── ProvidersPage.vue
│   ├── GroupsPage.vue
│   ├── ApiKeysPage.vue
│   └── LogsPage.vue
└── router/index.ts
```

## Rules

1. 业务数据只经 `src/api/tauri.ts` invoke，不直连侧车 HTTP 管理 API。
2. 密钥展示：创建客户端 Key 仅一次明文；列表仅脱敏。
3. 页面保持薄：加载/错误/空态齐全。
