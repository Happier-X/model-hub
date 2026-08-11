# shadcn-vue 规范偏差清理

## Goal

把前端代码里剩余的 shadcn-vue 规范偏差清理干净，使 `.trellis/spec/frontend/component-guidelines.md`（项目规约）与 `.agents/skills/shadcn-vue/rules/*`（官方规范）不再冲突，并让「暗色模式」在未来可用（本任务不实现暗色开关本身）。

上一轮已修完两处硬性偏差（图表 → `Chart`/Unovis、segment → `ToggleGroup`），本任务处理剩余四类。

## 背景：审计结论（已核实数据）

| 类别 | 量 | 分布 |
|---|---|---|
| A 原始颜色类 | 约 145 处颜色类 + 18 处 `bg-white` | 11 个文件：GroupFormPage 43、SettingsPage 31、GroupCard 25、ProvidersPage 15、statusCode.ts 10、AppShell 10、LogsPage 6、GroupsPage 3、HomePage 2、StatsCards 2、AppTitleBar 2 |
| B Card 冗余覆盖 | 12 处 `border border-slate-200 bg-white` | GroupsPage、GroupFormPage ×3、LogsPage、ProvidersPage、HomePage、SettingsPage ×4 |
| C 表单裸 `<label>` | 9 处 | GroupFormPage ×2、ProvidersPage ×4、SettingsPage ×3 |
| D 图标 `:size=` | 9 处 | AppTitleBar ×4、AppShell、OverlayApp、GroupFormPage、GroupsPage、ProvidersPage |
| E `space-x/y-*` | 7 处 | GroupCard、GroupFormPage ×2、HomePage、ProvidersPage、SettingsPage ×2 |

## 关键约束（已核实，决定方案边界）

1. **项目 token 是纯灰阶**：`components.json` baseColor 为 `neutral`，`--primary: oklch(0.205 0 0)`（近黑）、`--chart-1..5` 全部 chroma=0。因此 cyan/emerald/amber **无法**映射到 `primary` 而不改变视觉。
2. **缺 success / warning 语义 token**：`index.css` 只有 `--destructive`，没有 `--success`/`--warning`/`--info`。emerald（成功/已连接）、amber（4xx 警告）、cyan（品牌/提示）当前无处可归。
3. **rose 可直接映射**：`--destructive` 已存在，rose → `destructive` 无损。
4. **`.dark` 已定义但不可达**：`index.css:376` 有 `.dark` 段，但代码中无任何切换 `.dark` class 的逻辑。硬编码颜色目前不产生 bug，只是**阻塞**未来暗色模式。
5. **Card 覆盖是纯冗余**：`Card` 自带 `bg-card border`，而 `--card` = 白、`--border` ≈ slate-200，删除覆盖后视觉不变。
6. **项目 spec 与官方 skill 冲突**：`component-guidelines.md` 明确记载了「统一 `<label class="block text-sm"><span class="mb-1 block text-slate-600">` 模式」与 `text-slate-*` 写法。改代码前必须先改 spec，否则 `trellis-check` 会按旧 spec 判失败。

## Requirements

### 必须
- R1 **先对齐 spec**：更新 `component-guidelines.md`，用 Field 体系替代裸 `<label>` 约定、用语义 token 替代 `text-slate-*` 约定，并写明新增 token 的用法。spec 改动必须先于代码改动。
- R2 **颜色 token 化**：所有 `text|bg|border|ring-{slate,rose,emerald,amber,cyan,violet}-*` 替换为语义 token 或 shadcn 组件 variant。含 `statusCode.ts` 返回的 class 字符串。
- R3 **补齐缺失语义 token**：在 `index.css` 的 `:root` 与 `.dark` 同时定义 R2 所需的新 token（至少 success / warning，cyan 的归属见「待决策」），并在 `@theme inline` 暴露给 Tailwind。
- R4 **删除 Card 冗余覆盖**：12 处 `border border-slate-200 bg-white` 移除，保留布局类（`flex`/`min-h-0`/`flex-1` 等）。
- R5 **表单 Field 化**：安装 `field` 组件，9 处裸 `<label>` 改为 `Field`/`FieldLabel`/`FieldDescription`；Checkbox/Switch 行改为 `Field orientation="horizontal"`。
- R6 **`space-*` → `gap`**：7 处改为 flex/grid + `gap-*`。
- R7 **视觉零回归**：除「待决策」中用户明确批准的变化外，界面观感必须与改动前一致（token 值需按现有色值定义，而非套用 shadcn 默认）。

### 不做（明确排除）
- 不实现暗色模式切换开关（本任务只解除阻塞，开关另开任务）。
- 不改 `src/components/ui/**` 内 shadcn 生成的组件源码。
- 不动图标 `:size=`（类别 D）中 **AppTitleBar 的 4 处窗口控制按钮**——属桌面标题栏定制场景，`desktop-titlebar.md` 另有约定。其余 5 处改为由组件 CSS 控制尺寸。
- 不重构组件结构、不调整布局间距数值。

## 待决策（阻塞 R3，需用户确认）

**cyan（14 处）的归属**——它当前承担品牌强调 + info 提示双重角色（顶部提示条、"正在编辑"、"保存中…"、更新可用框、hover/选中态）。三个选项：

- **选项 1（推荐）**：新增 `--info` / `--info-foreground` token，值取现有 cyan 色值。`--primary` 保持近黑不动。→ 视觉 100% 不变，token 化达成，暗色可用。
- **选项 2**：把 `--primary` 改成 cyan 值，cyan 类 → `primary`。→ 品牌统一，但**所有 Button/Badge 默认态从近黑变青**，视觉剧变。
- **选项 3**：cyan → 现有 `accent` token。→ 无需新增 token，但 `--accent` 是浅灰（`oklch(0.97 0 0)`），**cyan 全部变灰**，提示条失去辨识度。

## Acceptance Criteria

- [ ] `grep -rE "(text|bg|border|ring)-(slate|rose|emerald|amber|cyan|violet)-[0-9]" src --include=*.vue --include=*.ts` 返回 0 处
- [ ] `grep -rE "bg-white" src --include=*.vue` 返回 0 处
- [ ] `grep -rE "class=\"[^\"]*space-[xy]-" src --include=*.vue` 返回 0 处
- [ ] `grep -rn "<label class=" src --include=*.vue` 返回 0 处
- [ ] `index.css` 中新增 token 在 `:root`、`.dark`、`@theme inline` 三处均有定义
- [ ] `component-guidelines.md` 不再包含 `text-slate-*` / 裸 `<label>` 约定，且新增 token 与 Field 用法有记载
- [ ] `pnpm typecheck` 通过
- [ ] `pnpm build` 通过
- [ ] `pnpm test:unit` 全绿（含 `statusCode` 若有测试）
- [ ] 人工确认五个页面（首页/分组/分组表单/供应商/日志/设置）观感与改动前一致

## Notes

- 分阶段推进，每阶段独立可验证、可回滚：spec 对齐 → token 定义 → 颜色替换 → Card/Field/space/icon。
- 颜色替换量大且跨文件，按文件分批，每批跑一次 typecheck + build。
