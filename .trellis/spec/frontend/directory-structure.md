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
│   ├── GroupsPage.vue    # 分组列表 + 卡片即时操作（编辑入口跳转表单页）
│   ├── GroupFormPage.vue # 分组新建/编辑独立页（/groups/new、/groups/:id/edit）
│   ├── LogsPage.vue
│   └── SettingsPage.vue  # 端口、数据目录、应用更新、自动检查偏好
├── router/
│   └── index.ts
└── utils/                # 纯函数逻辑 + 同名 *.test.ts（`pnpm test:unit` 覆盖）
    ├── groupSaveMode.ts
    ├── markdown.ts       # 更新日志等 markdown 渲染（html:false 转义原始 HTML）
    ├── modelCapability.ts
    ├── providerPaste.ts
    └── statusCode.ts
```

## 规则

1. 业务数据只经 `src/api/tauri.ts` 的 Tauri invoke 封装读写，不直连本机 HTTP 管理接口。
2. 页面使用 Vue 3 `<script setup lang="ts">`，并覆盖加载、错误和空数据状态。
3. 通用展示组件放 `components/`；有复用价值的异步状态编排放 `composables/`。
4. 外部客户端 HTTP `/v1/*` 不由前端调用或代理。
5. 无客户端 API Key 管理页；供应商上游 Key 仅在供应商表单中编辑。
6. 可独立测试的纯函数放 `utils/`，并配同名 `*.test.ts`（`node:test` + `node:assert/strict`，import 带 `.ts` 后缀）；页面组件内不放可复用的纯逻辑。
7. 渲染外部 markdown（更新日志等）统一走 `utils/markdown.ts` 的 `renderMarkdown`，它以 `html: false` 转义原始 HTML 作为 `v-html` 的安全前提，并强制链接 `target="_blank" rel="noopener noreferrer"`；不要在组件里另建 MarkdownIt 实例或放开 `html`。
