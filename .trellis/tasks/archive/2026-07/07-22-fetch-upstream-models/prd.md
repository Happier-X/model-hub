# 供应商拉取上游 models 列表

## Goal

在配置分组队列的 `upstream_model` 时，可从上游供应商的 OpenAI 兼容 `GET …/models` 拉取模型 id 列表并选择填充，减少手输错误。

## 背景

- 当前分组页 `upstream_model` 为纯文本输入，默认占位 `gpt-4o-mini`。
- 代理对外 `GET /v1/models` 返回的是**分组名**，不能替代上游模型列表。
- 供应商表已有 `base_url`、`api_key`；`base_url` 约定为含 `/v1` 的 OpenAI 兼容根（创建时 trim 尾斜杠）。

## Requirements

- R1：新增 Tauri 命令（建议名 `fetch_provider_models`），对指定供应商发起上游模型列表请求。
- R2：请求 URL 为 `{base_url}/models`（`base_url` 已含 `/v1` 时即为 `/v1/models`）；Header 使用 `Authorization: Bearer <上游 api_key>`。
- R3：解析常见 OpenAI 响应：`{ "data": [ { "id": "..." }, ... ] }`；返回有序、去重的 id 字符串列表。
- R4：支持两种入参形态（至少一种必须可用，推荐都支持）：
  - **已保存供应商**：`provider_id`，从库读取 base_url/api_key；
  - **表单草稿**（可选）：直接传 `base_url` + `api_key`，便于未保存时探测。
- R5：网络/鉴权/非 JSON 失败时返回可行动的中文错误（不回显完整上游 Key）。
- R6：合理超时（建议 15s 量级）；不写入 request_logs 业务表。
- R7：分组页每条队列：在选定供应商后可「拉取模型」；列表可选填入 `upstream_model`，仍允许手改。
- R8（可选增强）：供应商页「测试连接 / 拉取模型」展示数量或前若干 id。
- R9：集成或单元测试：mock HTTP 返回 data[].id 可解析；错误状态可映射。

## Acceptance Criteria

- [x] 已保存供应商可通过 IPC 拉取模型 id 列表。
- [x] 分组队列可将选中的上游模型写入 `upstream_model`。
- [x] 失败时 UI 展示中文错误，无完整 Key 泄露。
- [x] 测试覆盖解析与至少一种失败路径（或 wiremock/httpmock）。
- [x] `pnpm typecheck`、`pnpm lint`、`cargo test`（相关）通过。

## Out of Scope

- 缓存模型列表到 SQLite。
- 自动探测非 OpenAI 格式的私有 catalog API。
- 修改代理对外 `/v1/models` 语义。
- 按 token 计费或模型能力元数据 UI。

## 已确认倾向（可在实现中微调）

- 主入口：**分组页**（配置 upstream 的痛点）。
- 供应商页测试连接为可选增强。
- 不改熔断/转发逻辑。
