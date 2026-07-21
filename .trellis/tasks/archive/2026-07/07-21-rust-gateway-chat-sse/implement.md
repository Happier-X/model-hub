# 执行计划：Chat SSE 流式

## 清单

1. [x] 扩展 upstream：stream forward + 合适超时
2. [x] rewrite：stream 路径保留 stream=true
3. [x] v1_chat 分支 stream / non-stream；删除 STREAM_NOT_SUPPORTED
4. [x] 更新既有测；新增 wiremock SSE 集成测
5. [x] README + spec
6. [x] fmt/check/test/clippy（未 commit，交主会话归档）

## 验证

```powershell
cargo fmt --manifest-path gateway-rust/Cargo.toml -- --check
cargo check --manifest-path gateway-rust/Cargo.toml
cargo test --manifest-path gateway-rust/Cargo.toml
cargo clippy --manifest-path gateway-rust/Cargo.toml --all-targets -- -D warnings
```

## 审查门

- 透明 SSE 代理，不半截缓冲
- 鉴权/分组路由与非流式一致
- 密钥不进日志
- 不改 Tauri/前端/发布

## 回滚

回退 stream 相关 upstream/handler 变更。
