# 默认端口改为 8888

## Goal

将代理默认监听端口从 `8080` 改为 `8888`；已持久化的 `shell.json` 端口不受影响。

## Requirements

- R1：`DEFAULT_PORT` 及相关无配置回退值为 `8888`。
- R2：前端概览页端口输入与 Base URL 回退示例与默认一致。
- R3：用户文档与 backend spec 中的默认端口示例同步为 `8888`。
- R4：已有 `gateway_port` 配置继续优先，不强制覆盖为 8888。

## Acceptance Criteria

- [ ] `settings::DEFAULT_PORT == 8888`，相关单测通过。
- [ ] 无配置时代理与 UI 默认端口为 8888。
- [ ] 文档/规范中的默认端口示例为 8888。
- [ ] `cargo test`（settings 相关）、`pnpm typecheck`、`pnpm lint` 通过。
