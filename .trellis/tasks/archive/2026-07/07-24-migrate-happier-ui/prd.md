# 组件库改用 happier-ui

## 目标

在 Model Hub 管理台渐进接入 npm 版 `happier-ui`：库已提供的语义组件优先替换；表格、Select、侧栏壳、卡片分区等未覆盖部分继续使用 Tailwind CSS 与现有结构。**业务表单状态与提交流程统一使用 `@tanstack/vue-form`（TanStack Form）管理**，控件渲染仍用 `HInput`/`HCheckbox` 等。

## 背景与已确认事实

- 当前前端：`Vue 3 + Vite + TypeScript + Tailwind 4`（`@tailwindcss/vite`、`src/index.css` 引入 Tailwind）。
- 页面：`OverviewPage`、`ProvidersPage`、`GroupsPage`、`LogsPage`；壳：`AppShell`；对话框：`AppDialog`。
- 现有 `AppDialog` 契约：`open` / `title` / `size`（`default`|`wide`）/ `closeDisabled`、Escape 与遮罩关闭、焦点循环与关闭后焦点恢复。
- npm `happier-ui@0.0.1` 导出：`HButton`、`HSwitch`、`HBottomSheet`、`HDialog`、`HInput`、`HCheckbox`、`HEmpty`、`HImage`、`HIcon`、`HTabBar`、`HNavBar`；样式 `style.css` / `tokens.css`；peer `vue`、`@lucide/vue`。
- `HDialog` 使用 `v-model`、`closeOnOverlay`、`closeOnEsc`、`title`，**无** `size` / `closeDisabled` 等宽与禁用关闭 props。
- 库不提供表格、Select、Card、Badge、分页、侧栏导航壳、`textarea` 封装。
- 信息架构与业务逻辑不变。

## 已确认产品决策

- 依赖来源：**npm `happier-ui@0.0.1`**（锁定版本；另装 peer `@lucide/vue`）。
- **渐进替换**：有什么组件先替换什么；其余继续 Tailwind。
- **保留 Tailwind**：不卸载、不强制清 utility class。
- **不扩库**：本任务不修改 `happier-ui` 仓库。
- **表单状态**：供应商/分组对话框等业务表单使用 `@tanstack/vue-form` 的 `useForm` + `form.Field`，不再用手写 `reactive` 表单对象驱动提交字段。

## 需求

- R1：安装 `happier-ui@0.0.1` 与 `@lucide/vue`；`main.ts`（或等价入口）导入 `happier-ui/style.css` 与 `happier-ui/tokens.css`。
- R2：四页主要按钮改为 `HButton`（主操作 primary、次操作 secondary/outline/ghost、危险操作 danger）。
- R3：主要单行文本输入改为 `HInput`（含供应商 API Key 的 password 类型）；`select` / `textarea` 保持原生 + Tailwind。
- R4：布尔开关类控件优先 `HSwitch` 或带 label 的 `HCheckbox`（供应商启用、日志筛选、概览更新偏好等按交互选择更贴切者）。
- R5：`AppDialog` 改为基于 `HDialog` 的薄封装（推荐保留 `AppDialog` API 以降低页面改动），继续支持 `open`、`title`、`size`（wide 用宿主 CSS 约束）、`closeDisabled`（保存中禁止遮罩/Esc/关闭）。
- R6：列表空状态改为 `HEmpty`（供应商/分组/日志等「暂无…」）。
- R7：`AppShell` 侧栏与页面卡片分区、表格、分页继续 Tailwind，可不引入 `HNavBar`/`HTabBar`（移动端导航不匹配桌面侧栏）。
- R8：业务逻辑、Tauri IPC、路由、中文文案语义不变。
- R9：前端 `pnpm lint`、`test:unit`、`typecheck`、`build` 通过；Tailwind 构建仍可用。
- R10：安装 `@tanstack/vue-form`；**供应商新建/编辑**与**分组新建/编辑**对话框表单用 TanStack Form 管理字段值与提交（含分组队列 `items` 数组）；控件仍绑定 `HInput`/`HCheckbox`/原生 `select`/`input`。
- R11：粘贴识别、拉取模型、拖拽排序、批量添加等仍可写回表单状态（`setFieldValue` / 数组 field API），保存失败保留表单与编辑 id。
- R12：日志筛选、概览端口/偏好等**非对话框业务表单**可不强制迁入 TanStack Form（保持 ref 即可）；若一并迁移不得破坏筛选与启停行为。

## 验收标准

- [ ] AC1：`package.json` / lockfile 含 `happier-ui` 与 `@lucide/vue`；入口导入库样式与 token。
- [ ] AC2：概览、供应商、分组、日志页的主要按钮已使用 `HButton`。
- [ ] AC3：供应商/分组主要文本字段与 Key 输入已使用 `HInput`；`select`/`textarea` 可仍为原生。
- [ ] AC4：主要布尔控件已使用 `HSwitch` 或 `HCheckbox`。
- [ ] AC5：新建/编辑供应商与分组仍走对话框；保存中不可关闭；分组 wide 表单内容可滚动。
- [ ] AC6：空列表展示 `HEmpty`（或等价库组件），加载中文案不破坏。
- [ ] AC7：核心功能无回归：列表刷新、对话框保存取消、日志筛选分页、代理启停与更新相关按钮仍可点。
- [ ] AC8：未移除 Tailwind；lint / 单测 / typecheck / build 通过。
- [ ] AC9：`package.json` 含 `@tanstack/vue-form`；供应商与分组对话框表单经 `useForm`/`form.Field` 读写，保存仍走既有 IPC。
- [ ] AC10：分组队列增删/排序/批量添加后保存内容正确；供应商粘贴填入后字段更新且可保存。

## 范围外

- 删除 Tailwind 或全站去 utility class。
- 扩展 `happier-ui` 组件（Table/Select/Card 等）。
- 后端、IPC、信息架构、业务规则变更。
- 强制桌面侧栏改造成 `HNavBar`/`HTabBar`。

## 技术约束

- 优先保留 `AppDialog` 对外 props，内部适配 `HDialog` 的 `modelValue`。
- wide 与禁用关闭用宿主封装实现，不要求改库。
- 图标仅在确有需要时用 `HIcon` + `@lucide/vue`；本任务可不全面图标化。
- TanStack Form 与 `HInput`/`HCheckbox`：用 `field.state.value` + `field.handleChange`（或兼容 `v-model` 写法若库支持），避免脱离 form 状态的双源 `reactive`。
- 不强制引入 Zod 等校验库；本任务可只做状态托管，校验可沿用现有服务端/保存错误提示。
