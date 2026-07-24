# 实施计划：渐进接入 happier-ui

## 1. 依赖与入口

- [ ] `pnpm add happier-ui@0.0.1 @lucide/vue @tanstack/vue-form`
- [ ] `src/main.ts` 导入 `happier-ui/style.css` 与 `happier-ui/tokens.css`
- [ ] 确认 `vite.config.ts` 仍保留 Tailwind 插件

## 2. 对话框适配

- [ ] 改写 `src/components/AppDialog.vue`：内部使用 `HDialog`
- [ ] 映射 `open` ↔ `modelValue`；`closeDisabled` 时禁止关闭
- [ ] 实现 `size="wide"` 宿主样式（最大宽度/高度/内容滚动）
- [ ] 供应商页、分组页调用方式尽量不改 props

## 3. 页面控件替换

- [ ] `ProvidersPage.vue`：`HButton` / `HInput` / `HCheckbox|HSwitch` / `HEmpty` + **`useForm` 管理对话框字段**
- [ ] `GroupsPage.vue`：同上；队列操作小按钮用 `size="sm"`；textarea/select 保留；**`useForm` 管理 name + items**
- [ ] `LogsPage.vue`：筛选与分页按钮、`HEmpty`；select 保留（Form 可选）
- [ ] `OverviewPage.vue`：主要操作按钮与布尔控件；统计卡片布局保留 Tailwind
- [ ] 不强制改 `AppShell` 侧栏

## 3.1 TanStack Form

- [ ] 安装 `@tanstack/vue-form`
- [ ] 供应商：去掉对话框 `reactive(form)`，改为 `useForm`；reset/编辑/粘贴/提交走 form API
- [ ] 分组：去掉对话框 `reactive({ name, items })` 为真源，改为 form；拖拽/排序/bulk/`addItem` 写回 form
- [ ] 保存失败保留 values 与 `editing*Id`；保存中仍 `closeDisabled`

## 4. 验证

```powershell
pnpm lint
pnpm test:unit
pnpm typecheck
pnpm build
```

- [ ] 全局确认仍存在 `tailwindcss` 依赖
- [ ] 抽查四页：对话框保存中不可关、空状态、主要表单可输入

## 风险与回滚点

- 先完成 `AppDialog` 适配并 typecheck，再批量换按钮/输入。
- 若 `HDialog` 尺寸难控，回退为「HDialog 内容 + 外层 wide class」而不是改库。
