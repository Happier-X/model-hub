# 修复分组表单页双栏滚动回归

## Goal

修复 `GroupFormPage.vue` 双栏（可选模型 / 故障转移队列）内容无法滚动的回归。

## Background

- 上一任务（08-06-group-form-happier-ui）把双栏外层容器从手写 `div.rounded-lg.border` 换成 `HCard variant="outlined" padding="none"`。
- HCard 渲染结构：`.h-card`（flex column）> `.h-card__header` + `.h-card__body`（默认 slot 容器）。`styles.css` 中 `.h-card__body` 仅带 padding，**不是 flex 容器**。
- 双栏内部滚动区 `<div class="min-h-0 flex-1 overflow-y-auto p-3">` 的 `flex-1` 依赖父容器为 flex——`.h-card__body` 非 flex 导致 `flex-1` 失效，内容自然撑高、被 `.h-card` 的 `max-h-[32rem]` 截断，`overflow-y-auto` 不产生滚动。
- ProvidersPage 底部已有 `:deep(.h-card)` / `:deep(.h-card__body)` scoped 样式补齐 flex 链，GroupFormPage 改造时漏加。

## Requirements

1. `GroupFormPage.vue` 增加 `<style scoped>`，补齐：
   - `:deep(.h-card)` 为 flex column
   - `:deep(.h-card__body)` 为 flex column + `flex: 1` + `min-height: 0`
2. 不改变双栏现有结构、业务逻辑、组件用法。

## Acceptance Criteria

- [ ] AC1：双栏内部滚动区可正常滚动（内容超出 `max-h-[32rem]` 时出现滚动条）
- [ ] AC2：`pnpm typecheck` / `pnpm lint` / `pnpm build` 全绿
- [ ] AC3：仅样式层改动，无结构/逻辑变更
