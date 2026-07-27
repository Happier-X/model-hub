# 供应商页面UI调整：标题右侧加号并移除旧版管理条

## Goal

优化供应商页面布局：去掉独立的"供应商管理"顶部卡片，将"新建供应商"功能改为"+"号图标按钮，放到"供应商列表"标题右侧，使页面更简洁。

## Requirements

1. 移除页面顶部的"供应商管理" HCard（包含标题"供应商管理"和"新建供应商"按钮的那条）
2. 将"供应商列表"标题改为"供应商"
3. 在"供应商"标题右侧添加一个"+"号图标按钮（HButton isIconOnly），点击触发新建供应商（复用现有 `openCreate()` 函数）
4. 样式参照 AppTitleBar 的图标按钮风格：`variant="ghost"`、`size="sm"`、圆形或方形、使用 lucide Plus 图标
5. 弹窗内的新建/编辑逻辑完全不变
6. 页面整体 spacing 合理调整，去掉被移除的 HCard 后保持视觉层次清晰

## Acceptance Criteria

- [ ] 页面不再显示"供应商管理"顶部 HCard 条
- [ ] "供应商列表"卡片标题改为"供应商"
- [ ] 标题右侧有"+"号图标按钮，点击弹出新建供应商对话框
- [ ] 对话框功能（新建/编辑/粘贴快速添加）完全正常
- [ ] `npm run build` 通过，无类型错误

## Notes

- 轻量级 UI 调整，仅涉及 ProvidersPage.vue 模板部分
- 图标使用 @lucide/vue 的 Plus 组件，参照 AppTitleBar.vue 的使用方式