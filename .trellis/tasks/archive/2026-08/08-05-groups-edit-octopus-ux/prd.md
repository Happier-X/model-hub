# 分组编辑对齐 octopus 交互

## Goal

将分组页的编辑交互对齐 [bestruirui/octopus](https://github.com/bestruirui/octopus) 的分组管理体验：

1. **卡片内**即可完成常见队列调整（拖拽排序、删成员）与删除分组确认；
2. **新建 / 完整编辑对话框**改为 octopus 式**双栏选模**（左选模型、右队列排序）；
3. 不引入 octopus 的多负载 mode / 权重 / match_regex 等与本产品域模型不符的能力。

## 背景与现状

### model-hub 当前

- 列表已是响应式卡片网格（`GroupsPage.vue`，注释已标明 octopus 风格展示）。
- 点「编辑」打开 `AppDialog` 大表单；队列只在对话框内可改，保存后才落库。
- 删除分组使用浏览器 `confirm()`。
- 后端 `update_group` 为全量替换：`name` + `thinking_effort` + `items[]` + `source_provider_id`（`replace_items` 先删后插）。
- 上游模型列表**仅用户点击「拉取模型」**触发（`fetchProviderModels`）；禁止 onMounted / 保存时自动拉取（见 frontend `component-guidelines` §11 / backend `upstream-access`）。

### octopus 参考（已拉取源码对照）

- 卡片内：成员拖拽排序 / 删除 / 权重变更 → **即时 mutate 保存**。
- 删除分组：卡片内二次确认（动画覆盖），不用系统 confirm。
- 完整编辑：MorphingDialog + `GroupEditor` 双栏（左 `ModelPickerSection` 按渠道手风琴选模型 + 自动添加；右 `SortSection` 队列拖拽）。
- 卡片上可快速切换 mode（轮询/随机/故障转移/加权）—— **本产品不实现**（固定故障转移队列语义）。
- octopus 左侧依赖全局已同步的 `modelChannels`；本产品无此全局目录，需按供应商按需拉取。

## 已确认决策

| # | 决策 | 结论 |
|---|------|------|
| D1 | MVP 深度 | **方案 B**：卡片内即时操作 + 编辑对话框双栏选模 |
| D2 | mode / 权重 / Morphing 动画 | **不做**（Morphing 非 MVP；mode/权重与域模型不符） |
| D3 | 后端增量 API | **不做**；继续全量 `items` 替换 |
| D4 | 双栏左侧模型加载 | **L1**：左侧按供应商手风琴；**首次展开某供应商**时才 `fetchProviderModels`；会话内缓存；提供「刷新」再拉；禁止打开对话框全量预拉 |
| D5 | 卡片内删成员确认 | **M1**：直接删除并即时保存；**无**二次确认。删整组仍二次确认。对话框右侧队列同样点删即去 |
| D6 | 对话框批量添加 | **仅左侧**：展开后提供「全部加入」；**不再**保留独立「批量添加供应商全部模型」条。未展开时用户须先展开（触发拉取）再全部加入，或点该供应商的刷新后再全部加入 |

## Requirements

### A. 卡片内即时编辑

1. **拖拽排序**：非绑定分组卡片内拖动手柄调整故障转移优先级，松手后立即 `updateGroup` 全量写回。
2. **删除成员**：非绑定分组卡片内删除某一队列项后**立即保存、无二次确认**（D5=M1）。
3. **删除分组**：卡片内二次确认后才 `delete_group`；取消保持不变；**去掉** `window.confirm`。
4. **绑定自动同步只读**：`source_provider_id` 有值时卡片内禁止拖拽/删成员；完整编辑里模型队列仍只读；仍允许「配置到 Pi」、改名/思考强度/解绑等既有能力。
5. **并发与失败**：该卡保存中禁用冲突操作或展示保存中；失败有错误提示并 `refresh` 回滚，禁止假成功本地态。

### B. 双栏编辑对话框（新建 + 完整编辑共用）

6. **布局**：对话框加宽（`AppDialog` `size="wide"`，必要时再加宽 CSS）；上方保留本产品字段（分组名、思考强度、绑定供应商）；下方双栏：
   - **左**：按供应商手风琴；**首次展开**某供应商才 `fetchProviderModels`（D4=L1）；会话内按 `provider_id` 缓存；可「刷新」；关键词过滤**已加载**模型；点模型加入右侧；已选（同 provider_id + upstream_model）不可再点；展开后可「全部加入」未选模型（D6）。
   - **右**：已选故障转移队列（拖拽排序、删除、清空）；可保留 OpenRouter 匹配分展示与「按模型能力排序 / 强制刷新榜单」（仅改表单不自动保存）。
7. **提交语义不变**：稳定 `editingGroupId`；`getGroupSaveMode` → create / update；保存失败保留表单与编辑 id。
8. **既有能力迁移**：
   - 「按模型能力排序」「强制刷新榜单」保留在右侧工具条。
   - 独立「批量添加供应商全部模型」条**移除**，能力由左侧「展开 + 全部加入」覆盖（D6）。
   - 绑定态：左侧选模与右侧改队列禁用；编辑已有绑定时「立即同步」仍可用。
9. **上游访问**：打开对话框不预拉；仅展开 / 刷新 /（若实现显式拉取按钮）用户动作触发 `fetchProviderModels`。

### 明确不做

- 轮询 / 随机 / 加权 mode 与权重字段。
- Morphing 卡片→对话框变形动画。
- `match_regex` / 首 token 超时 / session keep 等 octopus 专用字段。
- 后端 `items_to_add|update|delete` 增量 API。
- 供应商页交互改造。
- 像素级复刻 octopus 视觉与 i18n 文案。

## 约束

- 前端：Vue 3 + happier-ui + Tailwind；可映射控件优先 `H*`；业务对话框表单继续 `@tanstack/vue-form` 约定（或拆子组件时仍保证 editingId 稳定）。
- 通信：仅 Tauri `invoke`；卡片即时保存与对话框保存均走现有 `update_group` / `create_group`。
- 绑定分组 24h 同步与「立即同步」语义不变。
- 文案：简体中文。

## Acceptance Criteria

### 卡片

- [ ] AC1：非绑定分组卡片内拖拽调整队列，松手后列表与库 `sort_order` 一致，无需开对话框。
- [ ] AC2：非绑定分组卡片内删除成员后**立即**列表与库同步消失，**无**二次确认，无需开对话框。
- [ ] AC3：绑定分组卡片不可拖拽/删成员。
- [ ] AC4：删除分组卡片内二次确认；取消不删；无系统 `confirm()`。
- [ ] AC5：卡片内保存失败有错误提示，列表恢复服务端数据。

### 双栏对话框

- [ ] AC6：新建 / 编辑对话框为双栏：左可选模型（按供应商手风琴），右已选队列可拖拽/删除/清空（非绑定）。
- [ ] AC7：用户可从左侧将模型加入右侧；已选不可重复添加（同 provider_id + upstream_model）；展开后「全部加入」只添加未在队列中的模型。
- [ ] AC8：打开对话框不对任何供应商预拉；仅展开/刷新时才请求该供应商；已缓存供应商再展开不重复请求（除非点刷新）。
- [ ] AC9：绑定态对话框队列只读；立即同步仍可用（编辑已有绑定时）。
- [ ] AC10：create/update 路径与 `editingGroupId` / `groupSaveMode` 行为保持正确；相关单测通过。
- [ ] AC11：按模型能力排序仍只改表单不自动保存（非绑定）。
- [ ] AC12：无独立「批量添加供应商全部模型」条；批量能力由左侧「全部加入」覆盖。

### 质量

- [ ] AC13：`pnpm typecheck` / lint 与相关单测通过。

## Open Decisions

全部关闭：D1=B，D2=不做 Morphing/mode/权重，D3=全量 items，D4=L1，D5=M1，D6=仅左侧全部加入。

## Notes

- 参考：octopus `web/src/components/modules/group/{Card,Editor,ItemList,Create,index}.tsx`。
- 主要改动面：`src/pages/GroupsPage.vue` 及可能的 `src/components/groups/*` 拆分；后端预计零改。
- 复杂任务：见同目录 `design.md` / `implement.md`。
