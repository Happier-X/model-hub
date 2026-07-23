# 分组页一键配置到 Pi（无 Key）

## Goal

在分组页为每个分组提供「配置到 Pi」；写入本机 `~/.pi/agent/models.json` 时模型名=分组名，**不展示、不要求、不传入客户端 Key**（与 CC Switch 类似：固定占位 apiKey 即可让 Pi 认模型）。

## Requirements

- R1：入口在**分组页**；每个已保存分组有「配置到 Pi」操作（列表行级）。
- R2：写入的模型 `id` 与 `name` 均为**该分组名**。
- R3：**无 Key UI / 无 Key 入参**；IPC 固定写入占位 `apiKey: "model-hub"`（与现 `DEFAULT_PLACEHOLDER_KEY` 及代理放行一致）。
- R4：Base URL = 当前代理 `base_url` 规范为含 `/v1`。
- R5：**单一** `providers.model-hub`；按分组名 **upsert** 模型条目（同 `id` 替换，其它 model-hub 模型保留；其它 providers 保留）。同时刷新该 provider 的 `baseUrl`/`api`/`apiKey` 为当前代理与占位 Key。
- R6：**移除** API 密钥页「配置到 Pi」区块与相关 Key 输入；文档改为分组页入口。
- R7：分组须已存在（按 id 查找）；代理 Base URL 无效时报可行动错误。
- R8：分组改名**不**自动删除 Pi 中旧 `id` 条目（用户对新名再点配置即可；旧 id 残留可接受，不在本次做清理 UI）。

## Out of Scope

- 分组级持久化 Key、Pi OAuth、`/login` 联动。
- 改代理鉴权语义。
- 从 Pi 配置中删除/重命名旧模型条目的专用 UI。

## Acceptance Criteria

- [ ] 分组页可对单个分组一键写入 Pi
- [ ] 模型名为分组名；无用户 Key 流程
- [ ] 其它 providers 与 model-hub 内其它模型不被整表覆盖清空
- [ ] API 密钥页无全局导出入口
- [ ] 单测 + typecheck/lint/cargo test 通过
- [ ] `docs/client-integration.md` 等说明已更新
