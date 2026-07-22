# 供应商粘贴快速添加

## Goal

供应商页支持粘贴一段配置，自动识别 Base URL、上游 API Key 与建议名称，并填入现有表单供用户确认保存。

## Requirements

- R1：支持 NewAPI 标准分享格式：`{"_type":"newapi_channel_conn","key":"...","url":"https://..."}`。
- R2：支持常见 JSON 字段别名：`base_url/baseUrl/url/endpoint` 与 `api_key/apiKey/key/token`。
- R3：支持环境变量：`OPENAI_BASE_URL` / `OPENAI_API_BASE`、`OPENAI_API_KEY` 等。
- R4：支持 curl：URL + `Authorization: Bearer ...`、`api-key` / `x-api-key`。
- R5：支持普通文本中的 URL + Key（多行或空格分隔）。
- R6：解析仅在前端本地进行；不上传、不打印、不写日志；Key 输入框保持 password。
- R7：识别后只填表，不自动保存；提示识别结果/缺失项。
- R8：Base URL 自动去尾 `/`；不盲目强加 `/v1`（保留用户原 URL）。NewAPI URL 也保持分享值。
- R9：若是新建且名称为空，基于 URL hostname 生成建议名称；编辑供应商时不覆盖名称。
- R10：解析工具写成独立纯函数并有单元测试。

## Acceptance Criteria

- [x] NewAPI 示例正确识别 url/key
- [x] JSON/env/curl/普通文本样例可识别
- [x] 无 Key 泄露到 UI 消息、日志或控制台
- [x] 识别失败中文可行动提示
- [x] typecheck/lint/test 通过
