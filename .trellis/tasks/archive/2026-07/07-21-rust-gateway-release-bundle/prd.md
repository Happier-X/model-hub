# 发布内嵌 Rust 网关

## Goal

在 Windows 发布链路中**额外内嵌** `model-hub-gateway.exe`（与 octopus 并存），使安装后可通过 `MODEL_HUB_GATEWAY_IMPL=rust` 零手工放置二进制试用 Rust 网关；**默认仍启动 octopus**。

## Background

- 壳已支持 `MODEL_HUB_GATEWAY_IMPL=rust|octopus` 与 rust 二进制解析，但安装态 rust 无资源部署，仅能靠 env/本地文件。
- 发布配置 `tauri.release.conf.json` 仅资源：`sidecar/octopus.exe` + third-party。
- CI `release-windows` 会 prepare octopus 再 tauri-action 打包。

## Requirements

### R1. 构建产物

- 新增脚本或 workflow 步骤：`cargo build --release --manifest-path gateway-rust/Cargo.toml`，产物复制到可被 Tauri resources 引用的路径（建议 `tools/gateway-rust/model-hub-gateway.exe`，gitignore 二进制）。
- 不在 git 提交 exe。

### R2. 安装包资源

- `tauri.release.conf.json` 增加 resource：  
  `tools/.../model-hub-gateway.exe` → `sidecar/model-hub-gateway.exe`（或 `sidecar/rust/model-hub-gateway.exe`，与代码常量一致）。
- 仍保留 octopus 与 AGPL 合规材料。

### R3. 运行时解析

- `IMPL=rust` 时解析优先级：
  1. `MODEL_HUB_GATEWAY_BIN`
  2. `MODEL_HUB_GATEWAY_RUST_BIN`
  3. 安装资源 `sidecar/model-hub-gateway.exe` → 按哈希部署到 `bin_dir/model-hub-gateway.exe`
  4. 已有 `bin_dir/model-hub-gateway.exe`
- 默认 octopus 路径不变。

### R4. CI

- release workflow 在 Tauri 打包前构建 gateway-rust（与 prepare octopus 并列）。
- rust-cache 可覆盖 `gateway-rust` workspace/path。

### R5. 文档

- README / gateway / gateway-rust：说明安装包已内嵌实验网关；默认 octopus；切换方式与 **勿混用 data.db**。
- 不宣称已移除 AGPL 侧车。

## Acceptance Criteria

- [x] AC1：release conf 同时声明 octopus 与 rust sidecar 资源映射。
- [x] AC2：CI workflow 在打包前 release 构建 gateway-rust。
- [x] AC3：rust 解析支持从 resource 哈希部署到 bin_dir。
- [x] AC4：默认 impl 与 octopus 解析行为无回归。
- [x] AC5：单测覆盖 rust 资源部署；cargo/pnpm 门禁通过。
- [x] AC6：未删除 octopus 内嵌与 AGPL 材料。

## Out of Scope

- 默认改为 rust
- 删除 octopus / 更新 NOTICE 去 AGPL
- octopus→rust 数据迁移
- 本机完整 NSIS 发包（CI 为准）
