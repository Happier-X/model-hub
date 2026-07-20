# Rust 网关 HTTP 服务骨架

## Goal

为后续 Rust 原生网关建立独立、可测试的 HTTP 服务进程骨架，不替换当前 octopus 侧车，不改变现有发布版；先冻结本机监听、配置、健康检查、错误格式和优雅退出契约。

## Background

- 当前业务由内嵌 octopus v0.9.28 提供，Tauri 壳负责路径、启停和健康检查。
- 父任务要求 Rust 网关最终保持现有管理 UI / OpenAI 客户端 HTTP 契约。
- 当前仓库只有 Tauri Rust 壳，没有独立 Rust 网关 crate；本子任务应新建独立目录，避免把业务实现塞进桌面壳。

## Requirements

### R1. 独立进程与配置

- 新建独立 Rust crate，建议目录 `gateway-rust/`。
- 可执行文件从命令行读取配置路径，默认使用 `data/config.json`。
- 读取 `server.host`、`server.port`；默认 `127.0.0.1:8080`。
- 配置错误、端口非法或 host 为空时启动失败并返回可读错误。
- 仅绑定配置指定地址；默认不得监听 `0.0.0.0`。

### R2. HTTP 路由

- 提供 `GET /health`，成功返回 JSON，例如：
  ```json
  {"status":"ok","service":"model-hub-gateway","version":"0.1.0"}
  ```
- 未知路径返回 JSON 错误体和 404，不返回 HTML。
- 暂不实现鉴权、SQLite、渠道、分组、Chat 转发；为后续模块预留 router/state 边界。

### R3. 生命周期

- 支持 Ctrl-C/Windows Ctrl-Break 等优雅退出信号。
- 退出时停止 HTTP 接收并释放监听 socket。
- 不按进程名清理，不操作 Tauri 现有 octopus 进程。

### R4. 可测试性

- 使用异步 HTTP runtime 和最小依赖。
- 单元测试覆盖配置默认值、配置校验、健康响应、未知路由错误响应。
- 集成测试启动随机本机端口，访问 `/health` 和未知路径后优雅退出。

### R5. 与壳集成边界

- 本任务不修改 Tauri `GatewayRuntime` 的默认侧车启动路径。
- README 说明本 crate 是实验性 Rust 网关骨架，不能用于替代当前发布版。
- 后续鉴权/业务子任务通过稳定 HTTP contract 接入，不复制 Tauri app data 业务库。

## Acceptance Criteria

- [x] AC1：独立 crate 可在 Windows 上 `cargo run -- --config data/config.json` 启动并监听 `127.0.0.1`。
- [x] AC2：`GET /health` 返回稳定 JSON 200；未知路径返回 JSON 404。
- [x] AC3：默认配置为 `127.0.0.1:8080`，非法配置不会静默回退到公网监听。
- [x] AC4：Ctrl-C 可优雅退出，测试不结束其他 octopus 进程。
- [x] AC5：单测/集成测试、fmt、check 通过，且当前 Tauri/octopus 发布链路不变。

## Out of Scope

- 管理 JWT、客户端 `sk-octopus-...` 鉴权。
- SQLite、渠道/分组、模型列表、Chat/SSE、日志。
- Tauri 壳切换、安装包内嵌 Rust 网关、移除 octopus。
