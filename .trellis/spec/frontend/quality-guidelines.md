# Quality Guidelines（Frontend）

## Standards

1. **无登录路径**。
2. 概览页展示监听地址、端口、数据目录（`get_paths`）。
3. 代理未运行时状态明确。
4. 不展示完整上游/客户端密钥（可遮罩）。
5. MVP 文案简体中文。

## Lint / Typecheck

```powershell
pnpm lint
pnpm typecheck
```

## Forbidden

- 提交 `.env` 含真实 Key。
- 引入 Next 专用 API。
- 引入与 Vue 3 当前架构重复的前端运行时或状态框架。
