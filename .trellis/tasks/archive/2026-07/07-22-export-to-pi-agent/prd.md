# API Key 页一键配置到 Pi Agent

## Goal

在 **API Key 页** 提供一键按钮：把本机代理 Base URL 与全部分组写入 `~/.pi/agent/models.json` 的 `model-hub` 供应商，便于 Pi `/model` 直接选用。Key **可选**。

## Requirements

- R1：API Key 页「配置到 Pi Agent」按钮。
- R2：可选粘贴/填写客户端 Key；留空则写入占位 `apiKey`（如 `model-hub`）。
- R3：`baseUrl` = 当前代理 Base URL 的 `/v1` 形态（与概览一致）。
- R4：`models` = 当前全部**分组名**（id = 分组名）。
- R5：合并写入 `~/.pi/agent/models.json`，仅覆盖 `providers.model-hub`，保留其他供应商。
- R6：成功/失败中文提示；目录不存在则创建 `.pi/agent`。
- R7：不改代理协议；单测覆盖 JSON 合并与模型列表生成。

## Acceptance Criteria

- [x] Key 页可一键配置。
- [x] 无 Key 也能写入配置。
- [x] 不破坏 models.json 其他 providers。
- [x] `cargo test` 相关 + 前端门禁通过。

## Out of Scope

- 自动新建专用 Key。
- 写入 auth.json。
- 启动 Pi 进程。
