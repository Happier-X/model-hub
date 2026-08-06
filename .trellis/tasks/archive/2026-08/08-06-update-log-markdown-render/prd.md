# 更新日志渲染为 Markdown 解析格式

## Goal

「检查更新」弹层中的更新日志（`pendingUpdate.body`）当前以 `<pre>` 输出 markdown 原文（#、-、``` 等符号可见），改为渲染成解析后的 HTML 格式（标题、列表、代码块、链接等有样式）。

## Background

- `src/pages/SettingsPage.vue` 第 347-349 行：`<pre>{{ pendingUpdate.body }}</pre>` 直接输出 markdown 原文。
- `pendingUpdate.body` 来自 `@tauri-apps/plugin-updater` 的 `Update.body`（即 GitHub Release body，本项目发布时写入 `changelog/v0.1.1.md` 的 markdown）。
- 更新日志内容为项目自身发布生成，但 Release body 理论上可被编辑，渲染需转义原始 HTML（防 XSS）。
- 项目无 opener/shell 插件；Tauri v2 默认 `urlOpenPolicy: allow`，`target="_blank"` 链接会在系统浏览器打开，无需新增插件。
- Tailwind v4 CSS-first 配置（`src/index.css` 以 `@import "tailwindcss"` 开头）。

## Decisions

| 决策 | 结论 |
|------|------|
| 渲染库 | `markdown-it`（成熟、默认 `html: false` 转义原始 HTML、linkify） |
| 样式 | 手写 `.markdown-body` 层（`@layer components`），不引 @tailwindcss/typography 插件，避免依赖膨胀 |
| 链接 | 渲染时加 `target="_blank" rel="noopener"`，Tauri 默认在系统浏览器打开 |
| 任务类型 | 轻量任务（PRD-only） |

## Requirements

1. **R1 引入 markdown-it**
   - `package.json` 新增 `markdown-it`（如包不自带类型则补 `@types/markdown-it` dev 依赖）
   - `pnpm-lock.yaml` 同步

2. **R2 SettingsPage 渲染**
   - `pendingUpdate.body` 经 markdown-it 解析（`html: false`）缓存为 computed
   - 替换 `<pre>` 为 `v-html` 容器（`max-h-40 overflow-auto` 等现有约束保留）

3. **R3 样式**
   - `src/index.css` 新增 `.markdown-body` 组件层样式：标题（h1/h2/h3）、p、ul/ol/li、code/pre、a、strong、hr、blockquote，贴合卡片内浅色背景（`bg-white/80` 区域）

## Acceptance Criteria

- [ ] AC1：`pnpm typecheck` / `pnpm lint` 绿（新依赖类型正常）
- [ ] AC2：更新日志不再显示 `#`、`-` 等 markdown 标记符号，显示为带样式的标题/列表
- [ ] AC3：原始 HTML 被转义（`html: false`），无 XSS 注入风险
- [ ] AC4：代码块/内联代码有等宽字体样式；链接点击在系统浏览器打开（target=_blank）
- [ ] AC5：`pnpm build` / `pnpm test:unit` 绿

## Notes

- 轻量任务：PRD-only，批准后 `task.py start` 直接实现。
- 手动验证需本地 `pnpm tauri dev` 触发检查更新（v0.1.1 已发布，本机 0.1.1 时无更新可显示；可用 v0.1.0 安装包或临时改 body 验证渲染效果，实现时自行确认验证方式）。
