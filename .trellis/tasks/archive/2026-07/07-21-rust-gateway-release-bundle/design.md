# 设计：发布内嵌 Rust 网关

## 路径约定

| 角色 | 路径 |
|------|------|
| 构建输出（CI/本地 prepare） | `tools/gateway-rust/model-hub-gateway.exe` |
| 安装资源 | `resource_dir/sidecar/model-hub-gateway.exe` |
| 运行时部署 | `bin_dir/model-hub-gateway.exe` |

常量：`BUNDLED_RUST_SIDECAR_RELATIVE = "sidecar/model-hub-gateway.exe"`。

## 脚本

`scripts/prepare-bundled-gateway-rust.ps1`：

```powershell
cargo build --manifest-path gateway-rust/Cargo.toml --release
Copy-Item gateway-rust/target/release/model-hub-gateway.exe tools/gateway-rust/
# 可选打印 SHA-256
```

## tauri.release.conf.json

```json
"resources": {
  "../tools/octopus/octopus.exe": "sidecar/octopus.exe",
  "../tools/gateway-rust/model-hub-gateway.exe": "sidecar/model-hub-gateway.exe",
  "../third-party/octopus/": "third-party/octopus/"
}
```

## binary.rs

`resolve_rust_binary(bin_dir, resource_dir)`：

1. RUST_BIN env  
2. resource sidecar 存在 → ensure_bundled_deployed → bin_dir  
3. bin_dir 已有文件  
4. error

复用 `ensure_bundled_deployed`。

## workflow

```yaml
- Prepare bundled octopus
- Prepare bundled gateway-rust  # cargo build --release + copy
- rust-cache workspaces: src-tauri + gateway-rust（若插件支持多路径则配置）
- tauri-action ...
```

## .gitignore

`tools/gateway-rust/*.exe` 或整目录类似 tools/octopus。

## 测试

- rust + fake resource 部署到 bin_dir
- octopus 仍不读 rust 资源名
