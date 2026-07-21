# 网关目录（已废弃）

> **自 Vue3 + 内嵌代理重写起，本目录与 `gateway-rust` 侧车不再作为运行时依赖。**

当前架构：

- 代理逻辑在 **`src-tauri` 进程内**（axum HTTP `/v1/*`）
- 管理面走 **Tauri commands**
- 不再需要 `model-hub-gateway.exe` / `pnpm prepare:gateway-rust` / 侧车资源打包

历史脚本 `scripts/prepare-bundled-gateway-rust.ps1` 与 `gateway-rust/` 源码可保留作参考，但 **`pnpm tauri dev` / 安装包均不依赖它们**。

请以仓库根目录 [README.md](../README.md) 为准。
