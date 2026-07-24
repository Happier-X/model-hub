# 概览改名为首页

## Goal

将应用中「概览」这一页面的所有对外称呼统一改为「首页」，做到界面文案、代码标识符、文档全部一致，不留新旧混用。

## Requirements

### 界面文案（用户可见）
- 侧边栏导航标签「概览」→「首页」（`src/components/AppShell.vue`）
- 路由页面标题 `meta.title` 「概览」→「首页」（`src/router/index.ts`）
- 概览页内部文案两处（`src/pages/OverviewPage.vue`）：
  - 「已开启：下次进入概览将自动检查更新」→「…下次进入首页将自动检查更新」
  - 「进入概览时自动检查更新（仍需确认后才安装）」→「进入首页时自动检查更新…」

### 代码标识符
- 路由 `name: "overview"` → `name: "home"`（`src/router/index.ts`）
- 组件与文件 `OverviewPage.vue` → `HomePage.vue`，同步更新 import 与 `component` 引用
- 组件内部脚本/引用如无其他强绑定，改名后须保证编译通过

### 文档
- `README.md`、`docs/` 下所有提到「概览」的文件统一改为「首页」
- `changelog/v0.0.1.md` 一并改为「首页」（按「全改」诉求；注：此为历史发布记录）
- `src-tauri/src/db/migrate.rs` 注释中「与概览统计相同」改为「与首页统计相同」

## Acceptance Criteria

- [ ] 全仓库（排除 `.trellis/`、`node_modules/`、`target/`、`dist/`）搜索「概览」「Overview」「overview」无残留
- [ ] 侧边栏显示「首页」，进入 `/` 时顶部标题显示「首页」
- [ ] 路由 `name` 为 `home`，页面组件文件为 `HomePage.vue`，import 与 `component` 引用同步更新
- [ ] 前端类型检查 / 构建通过（`npm run build` 或项目对应命令）
- [ ] 概览页内部两处文案更新为「首页」措辞，功能行为不变

## Notes

- 路径 `/` 保持不变，仅改名称与文案，不调整路由结构与跳转逻辑。
- `changelog` 属历史记录，本次按用户「全改」诉求一并更新；如后续认为应保留历史原文，可单独回退该文件。
- `config/shell.json` 的 `check_update_on_startup` 字段名不变，仅改展示文案。
