# Model Hub

Model Hub 是一个 Windows 优先的 Tauri 2 桌面应用脚手架，用于后续承载本地网关侧车与管理 UI。本阶段仅包含无登录主窗口、中文占位导航和应用数据目录路径契约，不包含网关进程启停或业务 CRUD。

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
- `gateway_dir`：网关数据目录
- `bin_dir`：后续侧车二进制目录

前端设置区会展示这些路径，用于验证桌面壳与 UI 的基础通信。

## 致谢与许可证提示

本项目后续将参考 octopus 相关实现与产品经验。若后续引入、分发或修改 AGPL 许可证覆盖的源码/二进制，请保留原项目致谢，遵守 AGPL 的源码提供与许可证传递要求，并在对应文档中说明来源、版本与许可证。
