# 用组件库替换手写 UI 实现

## 目标

把项目中手写的原生 HTML/Tailwind UI 实现替换成 `happier-ui` 组件库的对应组件，保持视觉一致性并减少手写样式维护。

## 背景

- `happier-ui` 当前已安装 `0.0.6`（最新版）。
- 项目中大部分 UI 已使用库组件（HButton、HCard、HBadge、HInput、HSelect、HSwitch、HTable 等）。
- 仍有少量手写实现未替换。

## 需求

### 1. SettingsPage.vue — 下载进度可视化

- 将更新下载进度的纯文本显示替换为 `HProgress` 进度条组件。
- 文本显示保留为进度条的说明标签，与进度条并存。

### 2. AppShell.vue — 更新提示栏关闭按钮

- 将更新通知栏的原生 `<button>` 关闭按钮替换为 `HButton variant="ghost" size="sm" isIconOnly shape="circle"`，与 GroupsPage 新建按钮风格一致。

### 3. 其他文件复核（无需改动）

- OverlayApp：Tauri 特殊深色浮窗，不适合替换。
- AppTitleBar：窗口控件原生按钮，spec 已约定保留原生。
- 统计卡片（HomePage）、内嵌列表（GroupsPage）等无等效组件，继续用 Tailwind。

## 范围外

- 不改后端 / Tauri API / 分组数据结构。
- 不升级 happier-ui 版本（已最新）。
- 不改变页面布局和交互逻辑。
- 不引入 spec 历史约定"本轮不启用"的组件。

## 验收标准

- [ ] `AppShell.vue` 的更新提示关闭按钮使用 HButton，行为不变。
- [ ] `SettingsPage.vue` 的下载进度包含 HProgress 进度条，下载文本保留为辅助标签。
- [ ] `pnpm lint`、`pnpm typecheck`、`pnpm build` 全部通过。