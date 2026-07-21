# 设计：去侧车发布

## impl_kind

```rust
// 缺省 / 未知 → Rust
// 仅 "octopus" → Octopus
match env::var(IMPL_ENV) {
  Ok(v) if v.eq_ignore_ascii_case("octopus") => Octopus,
  _ => Rust,
}
```

更新单测：`default_and_unknown_resolve_to_rust`；`explicit_octopus`。

## binary 错误文案

octopus 缺失：提示默认已是 rust；若需 octopus 设 `MODEL_HUB_GATEWAY_IMPL=octopus` 并提供 bin / `MODEL_HUB_GATEWAY_BIN`。  
不再写「安装包应自带 octopus」。

## release conf

```json
"resources": {
  "../tools/gateway-rust/model-hub-gateway.exe": "sidecar/model-hub-gateway.exe"
}
```

## CI

- 删除 Prepare bundled octopus
- 删除 Copy AGPL NOTICE 到 release-assets（可改为仅 SHA256SUMS + release notes）
- releaseBody 去掉 AGPL 侧车措辞
- rust-cache 保留 gateway-rust

## package.json

```json
"release:windows": "pnpm prepare:gateway-rust && tauri build ..."
```

`prepare:octopus` 可保留供本地兼容，文档标为可选回退。

## NOTICE

更新 `third-party/octopus/NOTICE.md` 顶部说明：当前发布包**不再内嵌**该二进制；文件仅历史/开发回退参考。

## 兼容

- `sk-octopus-` 前缀保留
- migrate-octopus CLI 保留
