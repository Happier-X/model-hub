# MVP 网关侧车集成

## Goal

在已有 Tauri 脚手架上集成 **可替换网关侧车进程**：钉扎 Windows 二进制获取方式、启停、健康检查、优雅退出；默认绑定 `127.0.0.1`；数据目录注入到既有 `gateway/` 路径契约。为后续管理 UI 与 Chat 转发提供「网关可运行」底座。

## Parent

- 父任务：`07-17-tauri-port-octopus`
- 依赖：`07-17-mvp-scaffold`（已归档）提供的 `get_paths` 与窗口壳

## Background

源项目 octopus：

- 启动：`./octopus start`
- 配置：`data/config.json`（首启可自动生成）
- 环境变量覆盖：`OCTOPUS_SERVER_HOST` / `OCTOPUS_SERVER_PORT` / `OCTOPUS_DATABASE_*` / `OCTOPUS_LOG_LEVEL`
- 默认 host 为 `0.0.0.0`；本产品强制默认 `127.0.0.1`（D6 安全模型）
- 正常退出需 SIGTERM/Ctrl+C 以刷落统计；禁止把 kill -9 当正常路径
- 上游管理台默认 admin 登录、客户端示例带 sk-；本产品 D3/D6 要求无管理登录与本机免鉴权——**本任务至少完成进程契约**；鉴权适配以「配置/探测/文档」最小改动为目标，若上游无法关闭则记录缺口并保证壳侧不强制登录 UI

## Requirements

### R1. 侧车获取与钉扎

- 提供 `gateway/README.md`：版本钉扎、Windows x64 下载/放置路径、许可证（AGPL）提示。
- 运行时从 `bin_dir`（或项目约定路径）解析可执行文件；缺失时返回可行动错误（如何放置）。

### R2. 配置与数据目录

- 使用 `get_paths` 的 `gateway_dir` / `bin_dir`。
- 启动前写入或更新侧车配置，至少：
  - `server.host = 127.0.0.1`
  - `server.port` 可配置（默认 8080）
  - `database.type = sqlite`，path 落在 `gateway_dir` 下
  - `log.level` 合理默认
- 优先环境变量注入（`OCTOPUS_*`）与/或 `config.json`，以侧车实际行为为准。

### R3. 进程生命周期

- 状态机：`idle | starting | running | stopping | error`
- 命令：`gateway_start` / `gateway_stop` / `gateway_status`（snake_case）
- `gateway_status` 返回：`state`、`host`、`port`、`pid`（可选）、`last_error`、`base_url`、`data_dir` 等
- 应用退出时尝试优雅停止（超时后强制结束并 warn）
- MVP 建议：应用启动后可自动尝试 start（失败不崩溃，状态进 error）

### R4. 健康检查

- running 判定：进程存活 + HTTP 探活（TCP/HTTP 到本机 port；路径按侧车可达根或文档化路径）
- 端口占用：start 失败信息明确，提示改端口或结束占用进程

### R5. 前端状态

- 全局状态条从「未集成」改为真实状态：未运行 / 启动中 / 运行中(port) / 停止中 / 错误信息
- 设置页：展示 host/port、base_url、侧车数据目录；提供启动/停止按钮
- 无登录页（本应用 UI 仍无 auth）

### R6. 鉴权适配（尽力）

- 记录并尽量配置：默认 127.0.0.1；文档说明客户端占位 api_key
- 若 octopus 强制管理登录/网关 Key：本任务不强制改 Go 源码大改；在 `gateway/README.md` 与 status/设计备注中写明适配策略与后续任务衔接

## Out of Scope

- 渠道/分组/日志业务 CRUD UI（下一子任务）
- OpenAI Chat E2E 完整验收文档（再下一子任务可覆盖）
- 自研 Rust 网关替换侧车
- MySQL/PG、全协议矩阵

## Acceptance Criteria

- [x] AC1：`gateway/README.md` 说明 Windows 二进制版本钉扎、放置路径、AGPL 提示。
- [ ] AC2：在 `bin_dir` 放置兼容 exe 后，`gateway_start` 可使状态变为 running，健康检查通过。（代码路径已实现；本机未放置真实 octopus.exe 做集成冒烟）
- [x] AC3：默认监听 `127.0.0.1` 与端口 8080；`gateway_status` / 设置页可见 base_url。
- [x] AC4：`gateway_stop` 与应用 `Exit` 路径停止托管进程；超时内轮询 wait/kill。
- [x] AC5：端口占用或二进制缺失时，错误消息可行动（单测覆盖缺失二进制）。
- [x] AC6：状态条展示真实网关状态；无登录页。
- [x] AC7：`pnpm build`、`cargo check`/`test` 通过；状态/config/health/binary 单测通过。

## Dependencies

- 脚手架 paths 契约已落地。
- 本机需用户提供或脚本下载的 octopus Windows 二进制（CI 可无真二进制时用 mock/缺省路径测试状态机）。
