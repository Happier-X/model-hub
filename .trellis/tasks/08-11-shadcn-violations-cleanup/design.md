# shadcn-vue 规范偏差清理 — 技术设计

## 1. 决策基线

- cyan 归属 = **选项 1**：新增 `--info` / `--info-foreground`（cyan 色值），`--primary` 不动。
- 同步新增 `--success` / `--success-foreground`（emerald 色值）、`--warning` / `--warning-foreground`（amber 色值）。
- rose → 复用现有 `--destructive`，零新增。
- slate → 全部映射到现有语义 token（`foreground` / `muted-foreground` / `border-border` / `bg-muted` / `bg-card` / `bg-background`），**不新增** slate 专属 token——shadcn 语义化即如此，slate 与 neutral 的色相差异（slate 微偏蓝、chroma≈0.04）在 UI 中不可感知。
- 暗色模式：本次只把 token 在 `.dark` 段补齐定义，**不实现切换开关**（PRD 已排除）。

## 2. 新增 token（index.css）

### 2.1 取值（Tailwind v4 默认色，oklch 原值）

| 语义 | :root（浅色） | .dark（深色） | 来源 |
|---|---|---|---|
| `--info` | `oklch(60.9% 0.126 221.723)` | `oklch(78.9% 0.154 211.53)` | cyan-600 / cyan-400 |
| `--info-foreground` | `oklch(98.4% 0.019 200.873)` | `oklch(30.2% 0.056 229.695)` | cyan-50 / cyan-950 |
| `--success` | `oklch(59.6% 0.145 163.225)` | `oklch(76.5% 0.177 163.223)` | emerald-600 / emerald-400 |
| `--success-foreground` | `oklch(97.9% 0.021 166.113)` | `oklch(26.2% 0.051 172.552)` | emerald-50 / emerald-950 |
| `--warning` | `oklch(66.6% 0.179 58.318)` | `oklch(82.8% 0.189 84.429)` | amber-600 / amber-400 |
| `--warning-foreground` | `oklch(96.2% 0.059 95.617)` | `oklch(27.9% 0.077 45.635)` | amber-100 / amber-950 |

dark 取值用更亮的 400 档：深色背景上 600 档文字对比度不足，400 档与浅色模式的 600 档感知亮度接近（dark 模式 shadcn 惯例）。

### 2.2 注册三处

1. `:root {}` 追加 6 个 `--info/-success/-warning` 及 `-foreground`
2. `.dark {}` 追加同样 6 个（dark 值）
3. `@theme inline {}` 追加 `--color-info: var(--info)` 等 6 条（供 Tailwind 生成 `bg-info` / `text-info` / `border-info` / `ring-info` 及 `/opacity` 变体）

### 2.3 使用惯例（写入 spec）

- 浅底提示/强调：`bg-info/10` + `text-info`（替代 `bg-cyan-50` + `text-cyan-800`）；更浅用 `/5`、更深用 `/15`。
- 边框：`border-info/20`（≈cyan-200）、`/30`（≈cyan-300）、`/40`（≈cyan-400）。
- 成功：`text-success`（≈emerald-700/600）、`bg-success/10`（≈emerald-50）、`bg-success/15`（≈emerald-100）。
- 警告：`text-warning`（≈amber-800）、`bg-warning/15`（≈amber-100）。
- 错误：`text-destructive` / `bg-destructive/10`（≈rose）。
- 遵循 shadcn 惯例：**主色实底、浅底用透明度变体**，不用深浅色阶堆砌。

## 3. 逐类改造方案

### A. 颜色类替换（约 145 处 + 18 处 bg-white）

映射表（实现时按此逐处替换，同类批量）：

| 原始类 | → 语义类 | 备注 |
|---|---|---|
| `text-slate-900/800/700` | `text-foreground` | 标题/主体文字（700 原本偏浅，略加深属规范提升） |
| `text-slate-600/500/400` | `text-muted-foreground` | 说明/次要文字（600→muted 略变浅，规范层级） |
| `border-slate-200/300` | `border-border` | `--border`≈slate-200，视觉一致 |
| `border-slate-100` | `border-border` | 同上 |
| `bg-slate-100/50` | `bg-muted` | `--muted`≈slate-100，视觉一致 |
| `bg-white` | `bg-card`（Card 内）/ `bg-background`（AppShell 头部/侧栏） | --card/--background 均白 |
| `bg-white/95` | `bg-card/95` | GroupCard 删除浮层 |
| `bg-white/80` | `bg-card/80` | SettingsPage 更新日志 |
| `text-cyan-950/900/800/700` | `text-info` | 提示条/强调文字 |
| `bg-cyan-50` | `bg-info/10` | 提示条底 |
| `bg-cyan-50/40、/30` | `bg-info/5` | GroupCard hover/供应商提示 |
| `bg-cyan-100` | `bg-info/15` | GroupFormPage 映射按钮 hover |
| `border-cyan-200` | `border-info/20` | 提示条边框 |
| `border-cyan-300` | `border-info/30` | 虚线提示框 |
| `border-cyan-400` | `border-info/40` | 选中态 |
| `ring-cyan-300` | `ring-info/30` | 选中态 |
| `text-emerald-800/700` | `text-success` | 已连接/成功文本 |
| `bg-emerald-50` | `bg-success/10` | |
| `bg-emerald-100` | `bg-success/15` | |
| `border-emerald-200` | `border-success/20` | |
| `text-amber-800` | `text-warning` | |
| `bg-amber-100` | `bg-warning/15` | |
| `text-rose-600/700` | `text-destructive` | 删除按钮/错误 |
| `bg-rose-50` | `bg-destructive/10` | |
| `bg-rose-100` | `bg-destructive/15` | |
| `border-rose-200` | `border-destructive/20` | GroupFormPage 加载失败卡 |
| `text-violet-700` + `bg-violet-50` | 见 A.1 | GroupCard 单处徽章 |

**A.1 特例——GroupCard.vue:136 violet 徽章**：实现时读取该处上下文文本（估计为「模型数/同步状态」类徽章），映射到 `bg-secondary text-secondary-foreground`（中性徽章）或 `bg-info/10 text-info`（信息徽章），二选一后写入 spec 与 journal。

**A.2 statusCode.ts**：`bg-slate-100 text-slate-600`（无状态）→ `bg-muted text-muted-foreground`；`bg-emerald-100 text-emerald-700`（2xx）→ `bg-success/15 text-success`；`bg-amber-100 text-amber-800`（4xx）→ `bg-warning/15 text-warning`；`bg-rose-100 text-rose-700`（5xx）→ `bg-destructive/15 text-destructive`。若存在对应测试同步更新断言。

**A.3 排除（保持原样）**：
- `src/components/ui/**`（shadcn 生成源码）
- `OverlayApp.vue` 的 `<style>` 内 hex 色（`#94a3b8`/`#eab308` 等）——overlay 是独立渲染上下文（非 shadcn 组件树），与 AppTitleBar 同理保留。
- `--chart-1..5`（图表色，已灰度，符合项目中性品牌）。

### B. Card 冗余覆盖（12 处）

- 10 处 `class="... border border-slate-200 bg-white"` → 删 `border border-slate-200 bg-white`，**保留布局类**（`flex` / `min-h-0` / `flex-1` / `flex-col`）。
- GroupFormPage.vue:513 加载失败卡 `border border-rose-200 bg-rose-50` → `border-destructive/20 bg-destructive/10`。
- HomePage.vue:92 / SettingsPage ×4（空覆盖，仅 `border border-slate-200 bg-white`）→ 整个 `class` 删空。

### C. 表单 Field 化（9 处裸 `<label>`）

安装：`pnpm dlx shadcn-vue@latest add field`（生成 `src/components/ui/field/*`，含 Field/FieldLabel/FieldDescription/FieldError/FieldGroup/FieldContent/FieldSet 等，reka-ui 已具备）。

替换规则：
1. **垂直输入**（GroupFormPage:533 分组名、544 思考强度；ProvidersPage:303 名称、314 Base URL、326 上游 Key；SettingsPage:332 端口）：
   ```vue
   <!-- 旧 -->
   <label class="block text-sm">
     <span class="mb-1 block text-slate-600">分组名（对外 model）</span>
     <Input ... />
   </label>
   <!-- 新 -->
   <Field>
     <FieldLabel>分组名（对外 model）</FieldLabel>
     <Input ... />
   </Field>
   ```
   `FieldLabel` 自带 `text-sm font-medium text-foreground` 样式（原 `text-slate-600` 为次要文字，FieldLabel 是标准 label 视觉，规范提升）。
2. **横向 Checkbox**（ProvidersPage:340 供应商 enable；SettingsPage:371 overlay、395 启动检查）：
   ```vue
   <!-- 旧 -->
   <label class="flex items-center gap-2 text-sm"> <Checkbox :model-value="..."/> 文本 </label>
   <!-- 新 -->
   <Field orientation="horizontal">
     <Checkbox id="xxx" :model-value="..." @update:model-value="..." />
     <FieldLabel for="xxx">文本</FieldLabel>
   </Field>
   ```
   `orientation="horizontal"` 提供 flex items-center gap-2 布局；**保留 Checkbox 原有 v-model 绑定逻辑**（field.state.value 等不动）。
3. 若某处 label 同时有说明文字，用 `FieldDescription`。

**注意**：`Checkbox` 的 id 需与 `FieldLabel for` 一致；无 id 时 reka-ui 自动关联也可（Field 注入），实现时以最简单可靠为准。

### D. 图标 `:size=`（5 处）

| 位置 | 图标 | 环境 | 处理 |
|---|---|---|---|
| AppShell.vue:101 | `X` | shadcn `Button size="icon"` | 删 `:size="16"`，Button 规则 `[&_svg:not([class*=size-])]:size-4` 接管（16px 不变） |
| GroupsPage.vue:143 | `Plus` | shadcn `Button` | 删 `:size="18"` → 默认 16px（18→16，2px 缩进，视觉可忽略，规范优先） |
| ProvidersPage.vue:375 | `Plus` | shadcn `Button` | 同上 |
| GroupFormPage.vue:591 | `ChevronDown` | `Item` 内（非 Button，无 svg 规则） | 删 `:size="14"`，改 `class="size-3.5"`（14px 等价，必须有显式尺寸否则 24px 默认） |
| OverlayApp.vue:187 | `ExternalLink` | 原生 `<button>` + 自定义 CSS | **保留 `:size="14"`**（非 shadcn 上下文，overlay 独立渲染） |

AppTitleBar 4 处：**不改**（用户确认，窗口控制按钮定制场景）。

### E. `space-*` → `gap`（7 处）

| 位置 | 旧 | 新 |
|---|---|---|
| GroupCard.vue:149 | `<ol class="... space-y-1 ...">` | `<ol class="... flex flex-col gap-1 ...">`（确认无 list-style 依赖；有则补 `list-none`） |
| GroupFormPage.vue:580 | `space-y-2` | `flex flex-col gap-2` |
| GroupFormPage.vue:707 | `space-y-1` | `flex flex-col gap-1` |
| HomePage.vue:89 | `space-y-6` | `flex flex-col gap-6` |
| ProvidersPage.vue:415 | `<span class="space-x-2">` | `<span class="inline-flex items-center gap-2">` |
| SettingsPage.vue:311 | `space-y-6` | `flex flex-col gap-6` |
| SettingsPage.vue:453 | `space-y-1` | `flex flex-col gap-1` |

`flex-col` 容器内子元素宽度会被拉伸——均为原有 block 布局，无影响。

## 4. spec 更新（R1，先于代码）

`.trellis/spec/frontend/component-guidelines.md` 修改：
1. 3.1 映射段「Input（无 label prop，用 `<label>` 包裹 + span 文本）」→ 改为「Field 体系（`Field > FieldLabel + Input`；Checkbox/Switch 用 `Field orientation="horizontal"`）」。
2. 「Switch/Checkbox label 文本由外层 `<label class="flex items-center gap-2">` 承载」→ 同上改 Field。
3. 新增小节「语义 token」：`--info/--success/--warning` 及使用惯例（2.3 表）、`.dark` 已定义但开关未做。
4. 清除任何残留 `text-slate-600` 写法约定（若有）。
5. 图标段：Button 内图标不设尺寸（组件规则接管）；非 Button 上下文（Item 内）用 `class="size-*"` 显式；overlay/titlebar 定制场景可保留 `:size`。

## 5. 验证策略

- 每个文件批次改完 → `pnpm typecheck`。
- 全部完成 → `pnpm build` + `pnpm test:unit`。
- AC 验收命令（PRD 已列）：颜色类、bg-white、space-*、裸 label 四个 grep 归零（排除 `src/components/ui/**`）。
- 人工过 6 页观感。

## 6. 边界与回滚

- 不改 `src/components/ui/**`；不动 OverlayApp CSS；不动 AppTitleBar。
- 每类改造相互独立（A 颜色 / B Card / C Field / D 图标 / E space / spec），可单独回滚。
- 颜色替换基于映射表机械执行 + typecheck；若某处映射导致布局/对比明显异常，回退该处并在 journal 记录。
