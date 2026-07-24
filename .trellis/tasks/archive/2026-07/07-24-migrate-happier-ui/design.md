# 技术设计：渐进接入 happier-ui

## 设计目标

1. 以 npm `happier-ui@0.0.1` 作为可映射控件的统一来源。
2. 保留 Tailwind 承担布局、表格、select/textarea 与侧栏壳。
3. 用薄封装吸收 `HDialog` 与现有 `AppDialog` 契约差异，避免四页大面积重写对话框 API。

## 依赖与入口

| 项 | 方案 |
|----|------|
| 安装 | `pnpm add happier-ui@0.0.1 @lucide/vue @tanstack/vue-form` |
| 样式 | `main.ts`：`import "happier-ui/style.css"`、`import "happier-ui/tokens.css"`；保留现有 `index.css` + Tailwind |
| Vite | 继续 `@tailwindcss/vite`；无需特殊 alias |
| 表单 | `@tanstack/vue-form` 的 `useForm`；供应商/分组对话框必用 |

## 组件映射

| 现网模式 | 目标 |
|----------|------|
| 实心主按钮 | `HButton variant="primary"` |
| 描边/次要按钮 | `HButton variant="outline"` 或 `secondary` |
| 文字链接式操作 | `HButton variant="ghost"` / `tertiary` |
| 危险删除 | `HButton variant="danger"` 或 `danger-soft` |
| 小按钮 | `size="sm"` |
| 单行 input | `HInput`（`type`、`label`、`v-model`） |
| 启用/筛选布尔 | `HSwitch` 或 `HCheckbox`（带 label） |
| 空列表文案 | `HEmpty title="暂无…"` |
| `AppDialog` | 内部用 `HDialog`；对外保留 `open`/`title`/`size`/`closeDisabled`/`@close` |
| 表格 / select / textarea / 侧栏 | 不替换 |

## AppDialog 适配

现网 props：

```ts
open: boolean
title: string
size?: "default" | "wide"
closeDisabled?: boolean
// emit close
```

`HDialog`：

```ts
modelValue: boolean
title?: string
closeOnOverlay?: boolean
closeOnEsc?: boolean
// slots: default, actions, title, description
// emit update:modelValue, close
```

适配策略：

1. `modelValue` ↔ `open`；`@update:modelValue` 在 `closeDisabled` 时忽略关闭。
2. `closeOnOverlay` / `closeOnEsc` 在 `closeDisabled` 时为 `false`。
3. `size="wide"`：宿主 class 或包装层约束 `max-width` / 高度与内部滚动（HDialog 默认居中；用 CSS 覆盖或外层 class）。
4. 页内底部「保存/取消」可继续放在 default 槽，或迁到 `#actions`（优先少改页面结构：内容仍 default 槽）。
5. 焦点循环/焦点恢复：以 `HDialog` 内置行为为准；若缺 wide 滚动，用包装 CSS 补齐。若 `HDialog` 无法满足 wide 滚动，保留 Teleport 包装仅做尺寸约束。

## TanStack Form 集成

### 供应商表单

```ts
const form = useForm({
  defaultValues: { name: "", base_url: "...", api_key: "", enabled: true },
  onSubmit: async ({ value }) => { /* create/updateProvider */ },
})
```

- 打开新建：`form.reset()` 到默认值。
- 打开编辑：`form.reset(providerFields)` 或逐字段 `setFieldValue`。
- 粘贴识别：对 `base_url`/`api_key`/`name` 调用 `setFieldValue`。
- 模板：`form.Field` + `HInput`/`HCheckbox` 绑定 `field.state.value` / `handleChange`。
- 保存按钮：`@click` → `form.handleSubmit()` 或 `type="submit"` + `@submit.prevent`。

### 分组表单

```ts
defaultValues: { name: "", items: [] as QueueItemDraft[] }
```

- 队列用 `form.Field name="items" mode="array"` 或对 `items` 整体 `setFieldValue` 以支持拖拽/排序/批量添加（整体替换数组更简单，可与数组 field 混用）。
- 单行 `provider_id` / `upstream_model` 用 `items[${i}].provider_id` 字段名，或继续整体 `items` 状态 + 子控件改数组后 `setFieldValue('items', next)`。
- **推荐**：复杂队列操作（拖拽、排序、bulk）统一读 `form.state.values.items`（或 store 订阅），写回 `form.setFieldValue('items', next)`；名称字段用 `form.Field name="name"`。
- 保存过滤空条目逻辑保持：`items.filter(i => provider_id > 0 && upstream_model.trim())`。

### 非目标

- 日志筛选、概览端口/偏好：可不迁；保持 `ref`。

## 页面改造顺序

1. 入口依赖与样式导入（可先编译通过）。
2. 重写 `AppDialog` 适配层。
3. `ProvidersPage`：按钮、输入、启用、空状态、对话框 + TanStack Form。
4. `GroupsPage`：同上 + wide 对话框 + TanStack Form（含 items）。
5. `LogsPage`：筛选按钮、复选、空状态；select 保持原生（可不迁 Form）。
6. `OverviewPage`：代理/更新区按钮与 checkbox；统计卡片布局保留 Tailwind。
7. `AppShell` 本任务可不改（无对等侧栏组件）。

## 样式共存

- Tailwind 与 `happier-ui` CSS 同时加载；优先避免用 Tailwind 覆盖 `H*` 内部类。
- 宿主仅在对话框 wide、页面间距、表格上使用 Tailwind。
- 全局 `index.css` 的 focus outline 可保留，注意不与库焦点环严重冲突；冲突时以库为主、缩小全局 button/input 规则范围。

## 风险

| 风险 | 缓解 |
|------|------|
| HDialog 无 wide | 宿主 CSS / 包装 |
| closeDisabled 语义 | 适配层拦截 model 更新 |
| 视觉不一致 | 主按钮统一 primary；危险统一 danger |
| peer 缺失 | 显式安装 `@lucide/vue` |
| 体积/样式顺序 | 先 token/style 再 index.css 或按文档顺序 |

## 回滚

移除依赖与导入，恢复 `AppDialog` 纯 Tailwind 实现与页面原生控件；无数据迁移。
