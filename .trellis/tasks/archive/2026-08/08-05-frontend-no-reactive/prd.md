# 前端状态约束：禁用 reactive，仅用 ref

## Goal

在项目 spec 中固化一条代码规范：**组件/页面状态一律使用 `ref`（含 `shallowRef` / `computed`），禁止使用 `reactive`**。统一状态声明风格，避免 `reactive` 的深层代理心智负担与模板解包差异。

## 背景与现状

- 当前代码库已无 `reactive(` 使用（`rg "reactive\(" src/` 无命中），约束只是把现状固化为规范，防止回潮。
- spec 中仍有两处文档将 `reactive` 作为合法选项提及：
  1. `.trellis/spec/frontend/state-management.md`：
     - 状态归属表「页面业务数据」示例 `ref / reactive`
     - 规则 6「禁止放入深层 `ref` / `reactive`」
     - 规则 7「`computed` / 回调内禁止与外层 `ref` / `reactive` 同名局部变量」
  2. `.trellis/spec/frontend/component-guidelines.md`：
     - 3.2「禁止再用独立 `reactive` 作为提交字段真源」（保留，它强调的仍是 TanStack Form 合同）
     - 状态与生命周期「局部交互使用 `ref` / `reactive` / `computed`」

## 明确不做

- 不改现有代码（当前无 `reactive` 使用）。
- 不动 TanStack Form 表单合同（3.2 的「禁止用 reactive 作提交真源」仍有效且与新约束方向一致）。
- 不引入 lint 规则 / eslint 插件（仅 spec 规范，如需强制可后续单独任务）。

## Acceptance Criteria

- [ ] AC1：`state-management.md` 状态归属表、规则 6、规则 7 中 `reactive` 表述全部改为仅 `ref`（规则 6 保持 `shallowRef`/`markRaw` 语义，改为「禁止放入深层 `ref`」）。
- [ ] AC2：`component-guidelines.md` 状态与生命周期一节改为「局部交互使用 `ref` / `computed`」，并新增一句话约束「**禁止使用 `reactive`，一律用 `ref`**」。
- [ ] AC3：全仓库（spec + src）不再出现将 `reactive` 作为合法选项的表述（3.2 表单真源禁令、测试文件名等除外，说明理由）。
- [ ] AC4：`pnpm typecheck` / lint 不受影响（无代码改动时跳过，若顺手清理则不引入回归）。

## Notes

- 轻量任务：PRD-only，无 design.md / implement.md。
- 涉及文件：`.trellis/spec/frontend/state-management.md`、`.trellis/spec/frontend/component-guidelines.md`。
