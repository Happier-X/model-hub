# 设计：真机侧车联调

## 二进制

| 项 | 值 |
|----|-----|
| 版本 | v0.9.28 |
| Asset | `octopus-windows-x86_64.zip` |
| URL | `https://github.com/bestruirui/octopus/releases/download/v0.9.28/octopus-windows-x86_64.zip` |
| 本地缓存（开发） | `tools/octopus/` 或 `%TEMP%`，通过 `MODEL_HUB_GATEWAY_BIN` 指向 |
| 生产放置 | `{bin_dir}/octopus.exe` |

提供 `scripts/fetch-octopus-windows.ps1` 下载并解压，默认输出到 `tools/octopus/octopus.exe`（gitignore）。

## 验证方法

1. 下载二进制
2. 设置 `MODEL_HUB_GATEWAY_BIN` 或复制到 app bin_dir
3. 用 Rust 集成或 CLI 手测：
   - 直接 `octopus.exe start` + env 注入
   - 或 `cargo test` 无法覆盖真进程时，用 PowerShell 脚本 + curl
4. 修 `process.rs` / `config.rs` 与上游 cwd 约定不一致处

## 风险

- octopus 可能 daemon 化导致父进程退出（已有兼容提示）
- 配置文件路径相对 cwd（gateway_dir）
- 默认 host 被 env 覆盖为 127.0.0.1
