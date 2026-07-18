# 执行计划：首次配置向导与闭环引导

## 清单

1. [x] 新增 `src/pages/DashboardPage.tsx`（检查清单 + 客户端示例 + 刷新）
2. [x] `App.tsx`：挂载新仪表盘并传入 running/auth/baseUrl/onNavigate；移除占位 Dashboard
3. [x] 文档与 directory-structure 更新
4. [x] `pnpm lint` / `pnpm build`

## 验证

```bash
pnpm lint
pnpm build
```

## 审查门

- 网关未运行时不得把渠道/分组/Key 标为已完成
- 不自动展示 list 中的完整 Key 到 curl 模板
- 跳转目标与 Sidebar 导航名一致
