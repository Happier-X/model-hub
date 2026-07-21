# 执行计划：Chat 非流式转发

## 清单

1. [x] 钉扎 reqwest（及测试 mock 依赖）
2. [x] RouterService：按分组名解析渠道/上游模型；轮询
3. [x] UpstreamClient：非流式 POST 转发、超时、错误映射
4. [x] `GET /v1/models` 返回分组列表
5. [x] `POST /v1/chat/completions` 鉴权 + 转发；拒绝 stream
6. [x] 单测 + wiremock/随机端口集成测
7. [x] README + backend spec
8. [x] fmt/check/test/clippy（未 commit，交主会话归档）

## 验证

```powershell
cargo fmt --manifest-path gateway-rust/Cargo.toml -- --check
cargo check --manifest-path gateway-rust/Cargo.toml
cargo test --manifest-path gateway-rust/Cargo.toml
cargo clippy --manifest-path gateway-rust/Cargo.toml --all-targets -- -D warnings
```

## 审查门

- 客户端 model = 分组名
- 上游 model = item.model_name
- 密钥不进日志
- stream 不半吊子代理
- 不改 Tauri/前端/发布

## 回滚

回退本子任务 gateway-rust 变更。
