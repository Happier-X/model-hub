# Quality Guidelines（Frontend）

## Standards

1. **无登录路径**。
2. 首页展示代理状态、Base URL、监听地址、今日统计、最近成功请求和接入指引；设置页展示并维护端口、数据目录（`get_paths`）、应用更新和自动检查偏好。
3. 代理未运行时状态明确。
4. 不展示完整上游密钥（可遮罩）。
5. MVP 文案简体中文。
6. 自动检查更新只在应用启动后执行一次；失败保持静默，发现新版本只提示前往设置，下载安装必须由用户在设置页确认。

## Lint / Typecheck

```powershell
pnpm lint
pnpm typecheck
```

## Forbidden

- 提交 `.env` 含真实 Key。
- 引入 Next 专用 API。
- 引入与 Vue 3 当前架构重复的前端运行时或状态框架。
