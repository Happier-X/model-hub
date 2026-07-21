# 设计：壳双实现切换

## 模块

```text
src-tauri/src/gateway/
  impl_kind.rs   # GatewayImpl enum + from_env
  binary.rs      # 扩展 rust 名与 RUST_BIN env
  process.rs     # 按 impl 构造 Command
  state.rs       # 可选 impl_name 字段
```

## GatewayImpl

```rust
pub enum GatewayImpl {
  Octopus,
  Rust,
}

pub fn resolve_gateway_impl() -> GatewayImpl {
  match env::var("MODEL_HUB_GATEWAY_IMPL") {
    Ok(v) if v.eq_ignore_ascii_case("rust") => GatewayImpl::Rust,
    _ => GatewayImpl::Octopus,
  }
}
```

## 二进制

- `DEFAULT_RUST_BINARY_NAME = "model-hub-gateway.exe"`
- `RUST_BINARY_ENV = "MODEL_HUB_GATEWAY_RUST_BIN"`
- 解析逻辑：

```text
if MODEL_HUB_GATEWAY_BIN set & file → use it (any impl)
else if Octopus → existing resolve (resource sidecar / bin_dir)
else if Rust:
  MODEL_HUB_GATEWAY_RUST_BIN → bin_dir/model-hub-gateway.exe → error
```

## 启动

```rust
match impl {
  Octopus => cmd.arg("start").arg("--config").arg(rel).envs(OCTOPUS_*),
  Rust => cmd.arg("--config").arg(rel), // 可不设 OCTOPUS_*；config.json 已含 server/database
}
```

`write_config_file` 共用。

## 状态

```rust
pub impl_name: String, // "octopus" | "rust"
```

前端 TS 类型加可选 `impl_name?: string` 避免破坏。

## 测试

- resolve_gateway_impl 默认与 rust
- build_command 参数快照（提取纯函数 `command_args(impl, config_relative) -> Vec<String>`）
- binary 解析在 rust 模式下找 rust 名

## 风险

- rust 与 octopus **数据库 schema 不兼容**：同一 `gateway_dir` 切换可能损坏体验。文档警告：切换实现请使用独立数据目录或清空 `data/data.db`。可选在 rust 启动时若检测到未知表仅依赖 migrate——rust 用自有 schema，octopus 用自己的；**混用同一 db 文件危险**。  
  缓解：文档明确；可选 rust 默认 `data/rust-data.db`——但会偏离 config 契约。  
  **本任务**：保持同一 config path，README 大字警告勿混用生产数据。
