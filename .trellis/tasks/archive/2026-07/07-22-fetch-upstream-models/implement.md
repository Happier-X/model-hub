# 实现清单

1. [x] 后端：`fetch_provider_models` + URL 拼接与 JSON 解析（可单测纯函数）
2. [x] 注册 command；错误映射中文
3. [x] 前端 API 封装
4. [x] GroupsPage：拉取 / 选择 / 手输兼容
5. [x] （可选）ProvidersPage 测试连接
6. [x] 测试 + `pnpm typecheck` / `pnpm lint` / `cargo test`
7. [x] 必要时一行说明写入 docs/client-integration 或 chat-onboarding

## 验证

```powershell
pnpm typecheck
pnpm lint
cd src-tauri
cargo test
```
