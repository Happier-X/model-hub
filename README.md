# Model Hub

Model Hub 是一个 Windows 优先的 Tauri 2 桌面应用，用于承载本地网关侧车（**默认 gateway-rust**，兼容历史 octopus 管理契约）与管理 UI：渠道、分组、**API 密钥**、日志与设置。

**当前版本：0.0.4** — Windows 安装包**默认内嵌** `model-hub-gateway`（gateway-rust）；**不再内嵌** octopus 二进制与 AGPL 合规附件。客户端 Key 前缀仍为 `sk-octopus-...`（历史命名，兼容）。详见 [v0.0.4 发布说明](./docs/release-notes-v0.0.4.md)。

## Windows 开发前置

- Windows 10/11，建议已安装 Microsoft Edge WebView2 Runtime（多数 Windows 环境已内置）。
- Node.js 20+（当前工程使用 pnpm）。
- pnpm 10+。
- Rust stable 与 Cargo。
- Tauri 2 的 Windows 构建依赖：Microsoft C++ Build Tools / Visual Studio Build Tools（含 Windows SDK）。

## 安装依赖

```bash
pnpm install
```

## 前端开发与构建

```bash
pnpm dev
pnpm build
pnpm lint
```

## 准备内嵌侧车（开发 / 发布）

二进制**不进 Git**。发布与默认路径只需准备 **gateway-rust**：

```powershell
pnpm prepare:gateway-rust
# 或
powershell -ExecutionPolicy Bypass -File scripts/prepare-bundled-gateway-rust.ps1
```

可选：本地回退 octopus（**不会**打进发布包）：

```powershell
# 或
$env:MODEL_HUB_GATEWAY_BIN = "$PWD\tools\gateway-rust\model-hub-gateway.exe"
```

开发覆盖（可选）：

```powershell
# 默认即为 rust，一般无需设置 IMPL
$env:MODEL_HUB_GATEWAY_BIN = "$PWD\tools\gateway-rust\model-hub-gateway.exe"
```

## Tauri 桌面开发

```bash
pnpm tauri dev
```

Rust 壳单独检查：

```bash
cd src-tauri
cargo check
```

## 默认 Rust 网关

独立 crate [`gateway-rust/`](./gateway-rust/) 为**默认**网关实现，Windows 安装包仅内嵌 `sidecar/model-hub-gateway.exe`。从旧版 octopus 升级时：请使用 `migrate-octopus` 导入到新库，或新建 `data/data.db`；**勿与 octopus 混用同一库文件**。

```powershell
cargo run --manifest-path gateway-rust/Cargo.toml -- --config gateway-rust/testdata/config.json
cargo test --manifest-path gateway-rust/Cargo.toml
```

详见 [Rust 网关说明](./gateway-rust/README.md) 与 [网关侧车文档](./gateway/README.md)。

## Windows 发布构建（NSIS + 内嵌 gateway-rust）

```powershell
pnpm release:windows
```

等价于 prepare **gateway-rust** 后执行：

```text
tauri build --bundles nsis -c src-tauri/tauri.release.conf.json
```

推送 `v*.*.*` tag 将触发 GitHub Actions：`.github/workflows/release-windows.yml`，产出 NSIS、Updater `.sig`、`latest.json`、SHA-256 与发布说明并创建 Release。发布前必须配置 `TAURI_SIGNING_PRIVATE_KEY` Secret。**发布包不再下载/上传 octopus 与 AGPL 附件。**

## 应用内更新

从首个集成 Tauri Updater 的版本开始，设置页提供“检查更新”：仅在用户点击时访问正式 GitHub Release，更新包必须通过签名校验，用户确认后才下载、安装和重启。v0.0.1/v0.0.2 需先手动安装该基线版本。密钥、CI Secret、发布和轮换见 [应用内更新说明](./docs/in-app-updater.md)。

## 数据目录契约

Rust 侧提供 `get_paths` 命令，首次调用会确保以下目录存在：

- `app_data_dir`：应用数据根目录
- `config_dir`：配置目录
- `gateway_dir`：网关数据目录（配置、SQLite）
- `bin_dir`：网关二进制目录（运行时从内嵌资源部署 `model-hub-gateway.exe`）

前端设置区会展示这些路径，用于验证桌面壳与 UI 的基础通信。

## 网关侧车

桌面壳已集成侧车启停（`gateway_start` / `gateway_stop` / `gateway_status`）：

1. **安装版**：无需手工放置 exe；默认自动部署内置 `model-hub-gateway`。
2. **开发版**：运行 `pnpm prepare:gateway-rust`，或设置 `MODEL_HUB_GATEWAY_BIN` / `MODEL_HUB_GATEWAY_RUST_BIN`。
3. 运行 `pnpm tauri dev`，在应用内查看状态条或设置页启动/停止。
4. 默认监听 `http://127.0.0.1:8080`（本机绑定）；可在设置页停止网关后修改监听端口，配置保存于应用配置目录的 `shell.json`，下次手动启动生效。
5. 在 **渠道 / 分组 / API 密钥 / 日志** 完成配置（无登录页；静默管理鉴权）。
6. 客户端调用 `/v1/*` 时须使用 **网关 API Key**（`sk-octopus-...`，历史前缀兼容），与管理 JWT 不同。详见 [客户端对接](./docs/client-integration.md)。

解析优先级（默认 rust）：`MODEL_HUB_GATEWAY_BIN` → `MODEL_HUB_GATEWAY_RUST_BIN` → 安装资源内嵌按哈希部署到 `bin_dir` → （开发无内嵌时）已有 `bin_dir` 副本。

## 文档

- [应用内更新说明](./docs/in-app-updater.md)
- [v0.0.4 发布说明](./docs/release-notes-v0.0.4.md)
- [v0.0.3 发布说明](./docs/release-notes-v0.0.3.md)
- [v0.0.2 发布说明](./docs/release-notes-v0.0.2.md)
- [v0.0.1 发布说明](./docs/release-notes-v0.0.1.md)
- [Chat 上手与故障排查](./docs/chat-onboarding.md)（端到端 + 错误对照 + 仪表盘自检）
- [客户端对接](./docs/client-integration.md)
- [M1 验收清单](./docs/mvp-acceptance.md)
- [网关侧车说明](./gateway/README.md)
- [Rust 网关说明](./gateway-rust/README.md)
- [第三方 octopus NOTICE / 源码（历史/可选回退）](./third-party/octopus/)

## 致谢与许可证提示

- **默认网关**为仓库内 `gateway-rust`，发布包**不再**分发 octopus 二进制。
- 可选自备的 octopus 来自 [bestruirui/octopus](https://github.com/bestruirui/octopus)（**AGPL-3.0**）。仓库 `third-party/octopus/` 仅作历史与开发回退参考；自行分发该二进制时请遵守 AGPL。
- Model Hub 桌面壳与管理 UI 源码以本仓库为准。**本文不构成法律意见。**
