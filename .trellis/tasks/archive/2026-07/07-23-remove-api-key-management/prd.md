# 移除 API Key 管理功能

## Goal

彻底移除**客户端 API Key** 的管理与鉴权：页面、IPC、领域代码、代理校验、数据库表/迁移与相关测试/文档一律删除。本机 `/v1/*` 不再校验客户端 Key（有无 Authorization 均放行）。**供应商上游 Key**（`providers.api_key`）保留。

## Requirements

- R1：移除 API 密钥页、路由与导航入口。
- R2：移除前端客户端 API Key 类型与 `list/create/update/delete` 封装。
- R3：移除 Tauri commands：`list_api_keys` / `create_api_key` / `update_api_key` / `delete_api_key`。
- R4：移除 `domain/apikey` 及 Stores 上客户端 Key CRUD / `validate_raw_key`。
- R5：代理 **删除** `require_key` / `extract_client_key` 及对 `api_keys` 表校验；不再因无效客户端 Key 返回 401。请求可无头、可带任意 `Authorization`（代理忽略客户端鉴权头）。
- R6：删除 Pi 占位 Key「特殊放行」分支（因已无客户端校验，无需再特判 `model-hub` / `none`）。Pi 导出仍可写固定 `apiKey: "model-hub"` 供 Pi UI 显示模型（非用户管理能力）。
- R7：数据库：**删除** `api_keys` 表创建、迁移兼容、领域访问与相关测试；**不**再兼容旧库客户端 Key 结构。新库不创建该表；启动路径不再依赖该表。
- R8：更新文档/README/组件规范中「可选客户端 Key / API 密钥页」表述；curl/SDK 示例改为无客户端 Key。
- R9：供应商页 `api_key`、上游 `Authorization: Bearer <供应商Key>` **保留**。
- R10：单测与集成测试：删除「无效客户端 Key 401 / 有效 Key / 占位 Key / api_keys 迁移」类用例；保留无 Key 访问与故障转移等。

## Out of Scope

- 不删除供应商上游 API Key 配置与转发。
- 不为旧 `api_keys` 数据做迁移/导出。
- 不改变 Pi `models.json` 占位字段写入策略（仍可写死 `model-hub`）。

## Acceptance Criteria

- [ ] UI/导航无客户端 API Key 管理
- [ ] 无客户端 Key 相关 IPC/domain
- [ ] 新 schema 不含 `api_keys`；无旧 Key 迁移兼容代码
- [ ] `/v1/*` 不校验客户端 Key（带错误 Key 也不 401）
- [ ] 供应商上游 Key 与故障转移仍正常
- [ ] 文档与示例已同步
- [ ] typecheck / lint / cargo test（含集成）通过
