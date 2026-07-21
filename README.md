# Model Hub

Model Hub 是一个 Windows 优先的 Tauri 2 桌面应用，用于承载本地网关（**gateway-rust / model-hub-gateway**）与管理 UI：渠道、分组、**API 密钥**、日志与设置。

**当前版本：0.0.6** — Windows 安装包内嵌 `model-hub-gateway`。客户端 Key 前缀为 `sk-modelhub-...`。详见 [v0.0.6 发布说明](./docs/release-notes-v0.0.6.md)。

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

## 准备内嵌网关（开发 / 发布）

二进制**不进 Git**：

```powershell
pnpm prepare:gateway-rust
# 或
powershell -ExecutionPolicy Bypass -File scripts/prepare-bundled-gateway-rust.ps1
```

开发覆盖（可选）：

```powershell
$env:MODEL_HUB_GATEWAY_BIN = "$PWD\tools\gateway-rust\model-hub-gateway.exe"
# 或
$env:MODEL_HUB_GATEWAY_RUST_BIN = "$PWD\tools\gateway-rust\model-hub-gateway.exe"
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

```powershell
cargo run --manifest-path gateway-rust/Cargo.toml -- --config gateway-rust/testdata/config.json
cargo test --manifest-path gateway-rust/Cargo.toml
```

详见 [Rust 网关说明](./gateway-rust/README.md) 与 [网关文档](./gateway/README.md)。

## Windows 发布构建（NSIS + 内嵌 gateway-rust）

```powershell
pnpm release:windows
```

等价于 prepare **gateway-rust** 后执行：

```text
tauri build --bundles nsis -c src-tauri/tauri.release.conf.json
```

推送 `v*.*.*` tag 将触发 GitHub Actions：`.github/workflows/release-windows.yml`，产出 NSIS、Updater `.sig`、`latest.json`、SHA-256 与发布说明并创建 Release。发布前必须配置 `TAURI_SIGNING_PRIVATE_KEY` Secret。

## 应用内更新

设置页提供“检查更新”：仅在用户点击时访问正式 GitHub Release，更新包必须通过签名校验，用户确认后才下载、安装和重启。v0.0.1/v0.0.2 需先手动安装更新基线版本。详见 [应用内更新说明](./docs/in-app-updater.md)。

## 数据目录契约

Rust 侧提供 `get_paths` 命令，首次调用会确保以下目录存在：

- `app_data_dir`：应用数据根目录
- `config_dir`：配置目录
- `gateway_dir`：网关数据目录（配置、SQLite）
- `bin_dir`：网关二进制目录（运行时从内嵌资源部署 `model-hub-gateway.exe`）

前端设置区会展示这些路径，用于验证桌面壳与 UI 的基础通信。

## 网关

桌面壳已集成启停（`gateway_start` / `gateway_stop` / `gateway_status`）：

1. **安装版**：无需手工放置 exe；自动部署内置 `model-hub-gateway`。
2. **开发版**：运行 `pnpm prepare:gateway-rust`，或设置 `MODEL_HUB_GATEWAY_BIN` / `MODEL_HUB_GATEWAY_RUST_BIN`。
3. 运行 `pnpm tauri dev`，在应用内查看状态条或设置页启动/停止。
4. 默认监听 `http://127.0.0.1:8080`（本机绑定）；**打开应用默认启动网关**。可在设置页修改监听端口，保存后写入 `shell.json` 并**自动重启**网关。
5. 在 **渠道 / 分组 / API 密钥 / 日志** 完成配置（无登录页；静默管理鉴权）。
6. 客户端调用 `/v1/*` 时须使用 **网关 API Key**（`sk-modelhub-...`），与管理 JWT 不同。详见 [客户端对接](./docs/client-integration.md)。

解析优先级：`MODEL_HUB_GATEWAY_BIN` → `MODEL_HUB_GATEWAY_RUST_BIN` → 安装资源内嵌按哈希部署到 `bin_dir` → 已有 `bin_dir` 副本。

## 文档

- [应用内更新说明](./docs/in-app-updater.md)
- [v0.0.6 发布说明](./docs/release-notes-v0.0.6.md)
- [v0.0.5 发布说明](./docs/release-notes-v0.0.5.md)
- [v0.0.4 发布说明](./docs/release-notes-v0.0.4.md)
- [v0.0.3 发布说明](./docs/release-notes-v0.0.3.md)
- [v0.0.2 发布说明](./docs/release-notes-v0.0.2.md)
- [v0.0.1 发布说明](./docs/release-notes-v0.0.1.md)
- [Chat 上手与故障排查](./docs/chat-onboarding.md)
- [客户端对接](./docs/client-integration.md)
- [M1 验收清单](./docs/mvp-acceptance.md)
- [网关说明](./gateway/README.md)
- [Rust 网关说明](./gateway-rust/README.md)

## 许可证

- **默认网关**为仓库内 `gateway-rust`。
- Model Hub 桌面壳与管理 UI 源码以本仓库为准。
