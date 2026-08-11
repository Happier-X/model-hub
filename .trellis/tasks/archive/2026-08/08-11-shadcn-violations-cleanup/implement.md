# shadcn-vue 规范偏差清理 — 执行计划

## 阶段总览（严格按序，每阶段独立验证、可回滚）

```
P0 spec 对齐（先改规约，后改代码）
P1 index.css 新增语义 token（info/success/warning ×2 模式 + @theme）
P2 Field 组件安装 + 表单改造（C 类）
P3 颜色批量替换（A 类，按文件分批）
P4 Card 覆盖删除（B 类）
P5 图标去 :size（D 类）
P6 space-* → gap（E 类）
P7 全量验证 + journal + 提交
```

依赖关系：P1 必须先于 P3（替换依赖 token 存在）；P0 先于 P7（check 用新规约）；其余互相独立可乱序，但按此序最稳。

---

## P0 — spec 对齐

**文件**：`.trellis/spec/frontend/component-guidelines.md`

1. 3.1 映射段：裸 `<label>` 约定 → Field 体系约定。
2. 「Switch/Checkbox label 由外层 `<label>` 承载」→ Field orientation="horizontal"。
3. 新增「语义 token」小节（info/success/warning 用法惯例 + .dark 说明）。
4. 清除残留 `text-slate-*` 写法。
5. 图标尺寸约定（Button 内不设尺寸 / Item 内 size-* 类 / overlay-titlebar 可保留）。

**验证**：`grep -n "text-slate-600\|<label class" .trellis/spec/frontend/component-guidelines.md` → 0 处；通读一遍。

---

## P1 — index.css token

**文件**：`src/index.css`

1. `:root` 追加 6 个 token（design 2.1 浅色值）。
2. `.dark` 追加 6 个（design 2.1 深色值）。
3. `@theme inline` 追加 6 条 `--color-*: var(--*)`。

**验证**：`grep -c -- "--info\|--success\|--warning" src/index.css` → 每类至少 3 处（:root/.dark/@theme）。

---

## P2 — Field 安装 + 表单改造

**命令**：`pnpm dlx shadcn-vue@latest add field`

**改造 9 处**（design C 节）：
- GroupFormPage.vue:533（分组名）、544（思考强度）
- ProvidersPage.vue:303（名称）、314（Base URL）、326（上游 API Key）、340（Checkbox）
- SettingsPage.vue:332（端口）、371（overlay Checkbox）、395（启动检查 Checkbox）

**要点**：
- 垂直输入 → `Field > FieldLabel + Input`（保留 Input 全部现有 props/v-model）。
- 横向 → `Field orientation="horizontal"` + `Checkbox` + `FieldLabel for`（保留 Checkbox v-model 绑定逻辑）。
- 若 FieldLabel 视觉与原 `text-slate-600` 差异明显，回退该处记录。

**验证**：`pnpm typecheck`；`grep -rn "<label class" src --include=*.vue` → 0 处。

---

## P3 — 颜色批量替换（最大工作量，按文件分批）

**顺序**（每批后 typecheck）：
1. `src/utils/statusCode.ts`（10 处，纯函数，先做可顺带改测试）
2. `src/components/groups/GroupCard.vue`（25 处，含 violet 特例决策）
3. `src/components/AppShell.vue`（10 处）+ `src/components/AppTitleBar.vue`（2 处）
4. `src/components/StatsCards.vue`（2 处）
5. `src/pages/HomePage.vue`（2 处）
6. `src/pages/GroupsPage.vue`（3 处）
7. `src/pages/LogsPage.vue`（6 处）
8. `src/pages/ProvidersPage.vue`（15 处）
9. `src/pages/GroupFormPage.vue`（43 处，最大）
10. `src/pages/SettingsPage.vue`（31 处）

**规则**：严格按 design A 节映射表；`bg-white` 一并处理（→bg-card/bg-background）；B 类的 Card 覆盖删除在涉及文件时顺手做但单独记录。

**验证**：每批 `pnpm typecheck`；最终 `grep -rE "(text|bg|border|ring)-(slate|rose|emerald|amber|cyan|violet)-[0-9]" src --include=*.vue --include=*.ts` → 0（排除 ui/）。

---

## P4 — Card 覆盖删除（若 P3 未全覆盖）

**文件**：GroupsPage:132、GroupFormPage:572/680、LogsPage:108、ProvidersPage:363、HomePage:92、SettingsPage:312/365/386/476（10 处 slate）+ GroupFormPage:513（rose 错误卡）。

**规则**：删 `border border-slate-200 bg-white`，保留布局类；513 改为 `border-destructive/20 bg-destructive/10`；HomePage/Settings 空覆盖整删 class。

**验证**：`grep -rn 'border-slate-200 bg-white' src --include=*.vue` → 0。

---

## P5 — 图标去 :size

**5 处**（design D 表）：AppShell:101 删 :size；GroupsPage:143、ProvidersPage:375 删 :size；GroupFormPage:591 删 :size + class="size-3.5"；OverlayApp:187 保留。

**验证**：`pnpm typecheck`；肉眼过图标。

---

## P6 — space-* → gap

**7 处**（design E 表）。

**验证**：`grep -rE 'class="[^"]*space-[xy]-' src --include=*.vue` → 0。

---

## P7 — 全量验证 + 收尾

1. `pnpm typecheck` ✓
2. `pnpm build` ✓
3. `pnpm test:unit` ✓（statusCode 若有测试断言同步）
4. PRD 全部 AC 复跑（4 个 grep 归零 + token 三处 + spec 检查）
5. 人工过 6 页观感（首页/分组/分组表单/供应商/日志/设置）
6. journal 记录（.trellis/workspace/happier/journal-2.md）
7. spec 确认已更新（P0）
8. `git add -A && git commit`（message 遵循项目风格：`feat(frontend): ...` 或 `refactor(frontend): ...`）

---

## 风险与预案

| 风险 | 预案 |
|---|---|
| FieldLabel 视觉与原 label 差异大 | 回退该处，写 journal，spec 记录差异 |
| 某处颜色映射致对比不足 | 按映射表微调（如 bg-info/5→/10），记录 |
| violet 徽章语义不明 | 读取上下文后按 design A.1 决策，记录到 journal |
| Unovis/Chart 相关 ui/ 目录被 grep 命中 | AC 命令排除 `src/components/ui/**`，已验证不涉及 |
