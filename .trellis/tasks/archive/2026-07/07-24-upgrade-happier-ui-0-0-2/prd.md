# 升级 happier-ui 0.0.2 并替换手写控件为库组件

## Goal

将 `happier-ui` 从 0.0.1 升级到 0.0.2，修复 CSS 入口破坏性改名，并在契合度高、风险可控的位置用 0.0.2 新增组件替换项目里的手写实现，同时更新前端 spec 的组件面约定。

## Requirements

### 必做（破坏性升级）

- `happier-ui` 依赖升级到 `0.0.2`（package.json + lockfile）。
- CSS 入口破坏性改名：`main.ts` 的 `happier-ui/style.css` → `happier-ui/styles.css`（旧名在 0.0.2 已不存在，不改则样式全丢）。
- `tokens.css` 入口名未变，保持不变。

### 组件替换（能契合就换）

- 手写外层分区卡片 → `HCard`（`variant="outlined"` + `padding`，标题进 `#header`）：HomePage / GroupsPage / LogsPage / ProvidersPage / SettingsPage。接受无 box-shadow 的库设计语言差异。
- 手写侧栏 → `HSidebar`：品牌区进 `#header`；`items` + `model-value=route.path` + 点击 `router.push`；若与全宽标题栏冲突严重则回退保留手写侧栏。
- **HIconButton 实测放弃**：扒编译 CSS 后确认与项目现存图标钮交互模型冲突（HIconButton 无 hover 背景态、固定正方形圆角、ghost 显蓝、danger 常驻红底），AppTitleBar 三窗口控件 / AppShell 更新提示关闭钮 / OverlayApp 打开钮硬换均导致交互退化，无真正契合落点，本轮不启用，三处保留原生 `<button>` + Tailwind。
- **明确不换**：HIconButton（见上，实测不契合）、HToast（更新提示是持久横幅）、HProgress（文本进度改进度条属增强）、HCell/HCellGroup（设置页复合行不符列表项语义）、HFloatingBubble（overlay 是独立窗口）、HRange（无场景）、select/textarea/表格（库无组件）。

### spec 更新

- 更新 `.trellis/spec/frontend/component-guidelines.md` 3.1 的组件面约定，纳入 `HIconButton` / `HCard` / `HSidebar`，并写明不启用组件的理由。

## Decisions

- 组件替换范围：**能契合就换**（最大范围）。实换 HCard + HSidebar。
- **HIconButton 实测放弃**：基于编译后 CSS，HIconButton 无 hover 背景态、尺寸固定正方形 + 圆角、ghost 显蓝、danger 常驻红底，与 AppTitleBar 窗口控件（OS 惯例满高矩形 + hover 变红）、AppShell 更新提示关闭钮（cyan 主题 hover）、OverlayApp 打开钮（scoped 深色浮层）均冲突，硬换即交互退化，本轮整体不启用。
- 不启用的组件及理由：
  - **HToast**：更新提示是持久顶部横幅（跟随主区、不自动消失），与 HToast 自动关闭浮层语义不符。
  - **HProgress**：当前下载进度仅文本，换进度条属新增功能/UX 改变，非替换范畴。
  - **HCell / HCellGroup**：设置页行是「输入 + 按钮 + 多行说明」复合结构，非标准 title+prefix/suffix 列表项，硬套破坏布局。
  - **HFloatingBubble**：overlay 是 Tauri 独立窗口，非 DOM 悬浮气泡，物理不可换。
  - **HRange**：项目无滑块场景。
- HSidebar 为最高风险项，作独立回滚点：若与全宽标题栏视觉/路由联动冲突严重，回退该文件保留手写侧栏并记录。

## Non-Goals

- 不改后端 Rust 代码。
- 不改 overlay 的窗口构造与生命周期（仅可能替换其内部按钮）。
- 不为替换而改变任何现有交互语义或可见文案。

## Acceptance Criteria

- [ ] `happier-ui` 安装版本为 0.0.2。
- [ ] `main.ts` 引用 `happier-ui/styles.css`，应用样式正常（无缺失）。
- [ ] 各页面外层分区卡片已替换为 `HCard`，标题进 header slot。
- [ ] 侧栏已替换为 `HSidebar` 且路由高亮/跳转正常；或已记录回退手写侧栏的原因。
- [ ] `pnpm lint`、`pnpm typecheck` 通过；`cargo build` 通过。
- [ ] 前端 spec 组件面约定已更新，与本轮实际替换一致（含 HIconButton 实测放弃的理由）。

## Notes

- 0.0.2 新增导出：HIconButton、HCard、HCell、HCellGroup、HToast、HRange、HProgress、HFloatingBubble、HSidebar。
- 现有 spec `component-guidelines.md` 3.1 基于 0.0.1 组件面写有「侧栏壳、卡片分区继续 Tailwind」「不扩展库组件面」，与本次「能用库组件就用」存在冲突，需重新划线并更新 spec。
- HIconButton API：`{ icon: Component; ariaLabel: string; variant?: primary|secondary|tertiary|outline|ghost|danger|danger-soft; size?: sm|md|lg; shape?: square|circle; disabled?; type? }`，emit `click`。
- HCard：纯展示容器，`variant: outlined|filled|flat`、`padding`、`radius`，具名 slot header/footer + default。
- HSidebar：`items: HSidebarItem[]` + `v-model` 选中 key + header/footer slot；与当前 RouterLink 手写侧栏需评估路由联动改造成本。
