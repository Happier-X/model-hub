# 实施计划

1. 代理删除客户端鉴权。
2. 删除 `domain/apikey.rs`、module、Stores 方法。
3. 删除 commands 与 lib 注册。
4. 迁移/schema 去掉 `api_keys` 及全部 Key 兼容逻辑与测试。
5. 前端删页/路由/导航/API。
6. 更新集成测试与文档/spec。
7. `cargo test` + `pnpm typecheck` + `pnpm lint`。
