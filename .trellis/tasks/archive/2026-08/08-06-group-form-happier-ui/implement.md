# 执行计划

## 1. 引入组件与替换顶部加载/错误态
- [ ] 顶部 import 补齐：`HCard`, `HCell`, `HEmpty`, `HTag`, `HLoading`
- [ ] 替换加载分组：`HLoading mode="local"`
- [ ] 替换加载失败：`HCard variant="outlined" class="border-rose-200 bg-rose-50"`

## 2. 替换双栏外层容器
- [ ] 左侧「可选模型」：`<div class="flex... max-h-[32rem]">` → `<HCard>`，标题行进 `#header`
- [ ] 右侧「故障转移队列」：同上，保留其间的逻辑与按钮

## 3. 替换左侧栏内部元素
- [ ] 供应商手风琴触发器：`<button>` → `<HCell clickable :show-chevron="false">`，图标进 `#prefix`，数量进 `#suffix`
- [ ] 拉取加载中：纯文本 → `<HLoading mode="local" size="sm" label="正在拉取模型…">`
- [ ] 上游无模型：纯文本 → `<HEmpty class="app-empty-compact" title="上游未返回模型" />`
- [ ] 无供应商提示：纯文本 → `<HEmpty class="app-empty-compact" title="暂无供应商，请先到「供应商」页添加" />`

## 4. 替换右侧队列内部元素
- [ ] 分数标签：`<span class="bg-emerald-50...">` → `<HTag size="sm" :variant="...">`
- [ ] 删除按钮：`<button class="text-rose-600...">` → `<HButton variant="ghost" size="sm" class="text-rose-600">`
- [ ] 队列为空：纯文本 → `<HEmpty class="app-empty-compact" :title="isBound ? '绑定...' : '队列为空...'" />`

## 5. 同步 spec
- [ ] 修改 `.trellis/spec/frontend/component-guidelines.md`，将 HTag/HCell 移入已启用，写明限制；登记 HLoading。

## 6. 质量检查
- [ ] 检查无未使用导入、无类型报错
- [ ] `pnpm typecheck` / `pnpm lint` / `pnpm test:unit` / `pnpm build`
