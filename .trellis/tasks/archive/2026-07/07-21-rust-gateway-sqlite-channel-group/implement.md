# 执行计划：SQLite 与渠道分组

## 清单

1. [x] 钉扎 `rusqlite`（bundled）与 tempfile（dev）
2. [x] `db`：打开路径、migrate v1、foreign_keys
3. [x] `SqliteApiKeyStore` 实现 trait；`AppState` 默认走 SQLite
4. [x] channel 模型/存储/路由（list/create/update/enable/delete）
5. [x] group 模型/存储/路由（list/create/update/delete）
6. [x] 单测：迁移、Key 持久化、渠道 Key 轮换、分组 rebind
7. [x] 集成测：随机端口全链路
8. [x] README + backend spec
9. [x] fmt/check/test/clippy；提交归档（本 agent 不 commit，仅验证）

## 验证

```powershell
cargo fmt --manifest-path gateway-rust/Cargo.toml -- --check
cargo check --manifest-path gateway-rust/Cargo.toml
cargo test --manifest-path gateway-rust/Cargo.toml
cargo clippy --manifest-path gateway-rust/Cargo.toml --all-targets -- -D warnings
```

结果（实现时）：全部通过；35 项测试（30 unit + 5 integration）。

## 审查门

- 客户端 Key 仍哈希存储
- type/mode 数字
- UI 字段兼容 keys_to_* / items_to_*
- 不改 Tauri/前端/发布
- 测试临时库，不杀 octopus

## 回滚

回退本子任务 gateway-rust 变更。
