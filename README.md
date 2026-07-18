# Model Hub

Model Hub 是一个 Windows 优先的 Tauri 2 桌面应用，用于承载本地网关侧车（octopus 兼容）与管理 UI：渠道、分组、**API 密钥**、日志与设置。

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

## Tauri 桌面开发

```bash
pnpm tauri dev
```

Rust 壳单独检查：

```bash
cd src-tauri
cargo check
```

## 数据目录契约

Rust 侧提供 `get_paths` 命令，首次调用会确保以下目录存在：

- `app_data_dir`：应用数据根目录
- `config_dir`：配置目录
- `gateway_dir`：网关数据目录（配置、SQLite）
- `bin_dir`：侧车二进制目录（放置 `octopus.exe`）

前端设置区会展示这些路径，用于验证桌面壳与 UI 的基础通信。

## 网关侧车

桌面壳已集成侧车启停（`gateway_start` / `gateway_stop` / `gateway_status`）：

1. 按 [gateway/README.md](./gateway/README.md) 下载 Windows 版 octopus 并放到 `bin_dir`，或设置 `MODEL_HUB_GATEWAY_BIN`。
2. 运行 `pnpm tauri dev`，在应用内查看状态条或设置页启动/停止。
3. 默认监听 `http://127.0.0.1:8080`（本机绑定）。
4. 在 **渠道 / 分组 / API 密钥 / 日志** 完成配置（无登录页；静默管理鉴权）。
5. 客户端调用 `/v1/*` 时须使用 **网关 API Key**（`sk-octopus-...`），与管理 JWT 不同。详见 [客户端对接](./docs/client-integration.md)。

缺少二进制时应用仍可打开，并显示可行动错误提示。

## 文档

- [Chat 上手与故障排查](./docs/chat-onboarding.md)（端到端 + 错误对照 + 仪表盘自检）
- [客户端对接](./docs/client-integration.md)
- [M1 验收清单](./docs/mvp-acceptance.md)
- [网关侧车说明](./gateway/README.md)

## 致谢与许可证提示

本项目后续将参考 octopus 相关实现与产品经验。若后续引入、分发或修改 AGPL 许可证覆盖的源码/二进制，请保留原项目致谢，遵守 AGPL 的源码提供与许可证传递要求，并在对应文档中说明来源、版本与许可证。
