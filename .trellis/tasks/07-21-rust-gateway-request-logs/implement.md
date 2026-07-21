# 执行计划：请求日志

## 清单

1. [x] migrate v2 request_logs
2. [x] LogStore/Service + list/clear
3. [x] RouteTarget 或查询补充 channel_name
4. [x] v1_chat 写入日志（非流式 + 流式尽力）
5. [x] routes `/api/v1/log/list` `/api/v1/log/clear`
6. [x] 单测 + 集成测
7. [x] README/spec
8. [x] fmt/check/test/clippy；提交归档（不 commit，交主会话）

## 验证

```powershell
cargo fmt --manifest-path gateway-rust/Cargo.toml -- --check
cargo check --manifest-path gateway-rust/Cargo.toml
cargo test --manifest-path gateway-rust/Cargo.toml
cargo clippy --manifest-path gateway-rust/Cargo.toml --all-targets -- -D warnings
```

## 审查门

- UI 字段兼容
- page_size ≤100
- 无密钥/messages 落库
- 不改 Tauri/前端/发布
