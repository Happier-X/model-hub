# 执行计划：升级 happier-ui 0.0.2 + 手写控件替换

## 有序清单

### 门 1：升级 + CSS 入口（先跑通样式）
1. `package.json` 已是 `0.0.2`（pnpm add 完成）；确认 lockfile 已更新。
2. `src/main.ts`：`happier-ui/style.css` → `happier-ui/styles.css`。
3. `pnpm typecheck` + `pnpm lint` 冒烟，确认导入无误。

### 门 2：HIconButton 替换（低风险，5 处）
4. `AppTitleBar.vue`：三个原生 `<button>` → `HIconButton`（最小化/最大化还原 ghost，关闭 danger 系）；保留图标切换、ariaLabel、不加 drag 属性；标题栏高度/命中区实测取 size。
5. `AppShell.vue`：更新提示关闭按钮 → `HIconButton`（ghost，icon X）。
6. `OverlayApp.vue`：打开主窗口按钮 → `HIconButton`（ghost，icon ExternalLink）。
7. `pnpm typecheck` + `pnpm lint`。

### 门 3：HCard 替换（各页面分区）
8. 逐页把外层 `section.rounded-xl border ... shadow-sm` → `<HCard variant="outlined" padding="md">`，`<h2>` 标题移入 `#header`：HomePage、GroupsPage、LogsPage、ProvidersPage、SettingsPage。
9. 内层小卡按契合度选 `flat`/`filled` 或保留 Tailwind（逐处判断，不硬套）。
10. `pnpm typecheck` + `pnpm lint`，肉眼核对模板结构无破坏。

### 门 4：HSidebar 替换（最高风险，单独一段）
11. `AppShell.vue`：手写 `<aside>` 导航 → `<HSidebar :items :model-value="route.path" @update:model-value="router.push">`，品牌区进 `#header` slot。
12. 实测侧栏视觉/布局与全宽标题栏衔接；若冲突严重 → 回退该文件保留手写侧栏，并在 design/spec 记录。
13. `pnpm typecheck` + `pnpm lint`。

### 门 5：spec + 全量验证
14. 更新 `component-guidelines.md` 3.1 组件面与边界（含不启用组件理由）。
15. 全量 `pnpm lint`、`pnpm typecheck`、`cargo build`。

## 验证命令

```powershell
pnpm lint
pnpm typecheck
cargo build --manifest-path src-tauri/Cargo.toml
```

## 审查门 / 回滚点

- 门 1 单独可验证（样式跑通）；门 2/3/4 各自 typecheck+lint 后再进下一门。
- 门 4（HSidebar）是独立回滚点：不佳则单文件 `git checkout src/components/AppShell.vue` 的侧栏部分。
- 全部改动前端为主 + package/lock + spec，无数据/迁移风险。

## 手动验收清单（需 pnpm tauri dev）
- [ ] 应用样式无缺失（styles.css 生效）。
- [ ] 标题栏最小化/最大化还原(图标切换)/关闭(藏托盘)正常。
- [ ] 更新提示关闭钮正常。
- [ ] overlay 打开主窗口正常，overlay 行为不变。
- [ ] 侧栏导航高亮与路由跳转正常。
- [ ] 各页面卡片视觉可接受（无阴影后层次仍清晰）。
