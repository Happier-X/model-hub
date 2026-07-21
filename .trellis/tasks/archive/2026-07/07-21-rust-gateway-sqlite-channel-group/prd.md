# Rust 网关 SQLite 与渠道分组

## Goal

在 `gateway-rust` 接入 **SQLite** 持久化，实现与现有 Model Hub UI 兼容的 **渠道 / 分组 CRUD**，并将 API Key 从内存存储切换为 SQLite；仍不实现 Chat 转发与 Tauri 切换。

## Background

- 鉴权与内存 API Key 已完成；`/v1/models` 仅占位。
- UI 契约（v0.9.28 对齐）：
  - 渠道：`type` 为数字；`create` 含 base_urls/keys；`update` 支持 `keys_to_update` / `keys_to_add`；`enable`/`delete`。
  - 分组：`mode` 数字；`items`；`update` 支持 `items_to_delete` / `items_to_add`。
  - 成功响应 `{ data }`；管理路径需 JWT。
- 配置已有 `database.type/path`（默认 sqlite + `data/data.db`）。

## Requirements

### R1. SQLite 基础设施

- 启动时按配置路径打开/创建 SQLite（相对路径相对进程 cwd）。
- 自动迁移/建表：api_keys、channels、channel_keys、channel_base_urls、groups、group_items。
- 仅支持 `database.type = sqlite`；其它类型启动失败。
- 测试可用临时文件库或 `:memory:`（若 :memory: 多连接受限则用 tempfile）。

### R2. API Key 持久化

- 用 SQLite 实现 `ApiKeyStore`，替换默认 `MemoryApiKeyStore`。
- 仍只存哈希 + 脱敏；完整 Key 仅 create 返回。
- 重启进程后 list / find_by_raw_key 行为一致。

### R3. 渠道管理 API（管理 JWT）

- `GET /api/v1/channel/list`
- `POST /api/v1/channel/create`（对齐 smoke/UI：name, type, enabled, base_urls, keys, model, custom_model, proxy, auto_sync, auto_group, custom_header）
- `POST /api/v1/channel/update`（部分更新 + keys_to_update / keys_to_add）
- `POST /api/v1/channel/enable` `{ id, enabled }`
- `DELETE /api/v1/channel/delete/:id`
- `type` 必须为数字；列表/详情返回数字 type。
- 上游 `channel_key` 可明文存于本机 SQLite（与侧车同类本机数据；**客户端** sk-octopus 仍只存哈希）。日志禁止打印完整上游 Key。

### R4. 分组管理 API（管理 JWT）

- `GET /api/v1/group/list`（含 items）
- `POST /api/v1/group/create`（name, mode, match_regex, items[]）
- `POST /api/v1/group/update`（name + items_to_delete / items_to_add）
- `DELETE /api/v1/group/delete/:id`
- 默认 mode 支持 `1`（轮询）；其它 mode 数字可存，本任务不做负载调度。

### R5. 边界

- `/v1/models` 鉴权不变；本任务可不根据分组填充模型列表（仍可空列表），或可选返回分组名列表（若实现简单则加，非 AC 强制）。
- 不实现 Chat、日志 list、数据从 octopus 迁移导入。
- 不修改 Tauri 壳/前端/发布。

## Acceptance Criteria

- [x] AC1：配置 sqlite 路径，重启后 API Key 仍可校验。
- [x] AC2：渠道 create/list/update/enable/delete 与 UI 字段兼容；`type` 为数字。
- [x] AC3：分组 create/list/update/delete 与 items 增删兼容。
- [x] AC4：管理路径无 JWT 401；成功包 `{data}`。
- [x] AC5：单测 + 随机端口集成测；fmt/check/test/clippy 通过。
- [x] AC6：不改变现有 octopus/Tauri 发布链路。

## Out of Scope

- Chat/SSE 转发与路由负载
- octopus SQLite 一键迁移
- 请求日志表
- 前端/壳切换网关实现
