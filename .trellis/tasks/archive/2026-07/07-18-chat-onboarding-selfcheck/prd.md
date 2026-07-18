# 真上游 Chat 上手与闭环自检

## Goal

补齐「配置完成后如何真正跑通 Chat」的最后一公里：提供 **故障对照上手文档**，并在 **仪表盘** 增加可选的 **客户端路径自检**（用用户粘贴的网关 Key 探测 `/v1/models` 与可选 chat），降低 401/502/model 错误时的排查成本。

## Background

- 鉴权闭环、渠道/分组编辑、仪表盘检查清单已完成。
- 现有 `client-integration.md` 偏接口说明；缺「按顺序操作 + 错误码含义 + 真上游成功条件」。
- 冒烟脚本用假上游 Key，chat 常 502 属预期；用户需要区分 **鉴权失败** vs **上游/路由业务失败**。

## Requirements

### R1. 上手文档

新增或扩展文档（建议 `docs/chat-onboarding.md`，并在 README / client-integration / mvp-acceptance 链接）：

1. 端到端步骤：启动 → 渠道（**真实上游 Key**）→ 分组（model=分组名）→ API Key → curl models → curl chat  
2. 错误对照表至少覆盖：

| 现象 | 常见原因 | 处理 |
|------|----------|------|
| 401 | 错用 JWT/占位 Key/Key 未创建 | API 密钥页复制 `sk-octopus-...` |
| models 200 但 data 空 | 尚无分组或未同步 | 建分组并绑定渠道 |
| chat 4xx/5xx 业务错误（非 401） | 上游 Key 无效、Base URL 错、model_name 不匹配、无路由 | 查渠道/分组绑定与上游 |
| 连接失败 | 网关未启动/端口错 | 设置启动；核对 Base URL |

3. 明确：**无真实上游 Key 时不保证 chat 200**；鉴权以 models 非 401 为准。

### R2. 仪表盘自检（可选 UI）

在 `DashboardPage` 增加「客户端路径自检」区块：

- 输入：网关 API Key（password 框，不落库、不写 localStorage）  
- 可选：分组名（默认空则只测 models）  
- 按钮：运行自检  
- 步骤结果：
  1. `GET {base}/v1/models`（Bearer 用户输入 Key）→ 状态码 + 是否 401  
  2. 若填写了分组名：`POST .../chat/completions` 极简 body → 状态码；**401 标失败**；非 401 标「鉴权通过（业务结果另见）」  
- 使用 **客户端 Key**，不得误用管理 JWT（可用独立 fetch 或 gatewayRequest bearer 模式）。

### R3. 脚本（可选增强）

- 保持现有 smoke 不要求真上游 200。  
- 可选：文档说明 `MODEL_HUB_UPSTREAM_KEY` 等环境变量做扩展探测（**非必须实现**；若实现则默认关闭）。

### R4. 约束

- 不提交真实 Key；自检 Key 仅内存。  
- 不改 octopus 二进制。  
- 清理策略不变。

## Out of Scope

- 自动填入 list 中的完整 Key 到自检框。  
- 保证任意供应商 chat 200。  
- Dashboard 图表 stats。  
- 多协议（Anthropic 等）客户端示例。

## Acceptance Criteria

- [x] AC1：存在上手文档含端到端步骤 + 错误对照，并从 README 或 client-integration 可链到。  
- [x] AC2：仪表盘可粘贴网关 Key 探测 `/v1/models` 是否非 401。  
- [x] AC3：可选 chat 探测：有分组名时发起请求；401 明确失败，其它状态可解读为业务层。  
- [x] AC4：自检不使用管理 JWT 作为客户端 Key。  
- [x] AC5：`pnpm lint` / `pnpm build` 通过；既有 smoke 不回归。

## Decisions Log

| # | 决策 | 选择 | 日期 |
|---|------|------|------|
| D1 | 文档 | 独立 `docs/chat-onboarding.md` + 互链 | 2026-07-18 |
| D2 | 自检位置 | 仪表盘区块 | 2026-07-18 |
| D3 | 真上游 | 用户环境；代码不强制 200 | 2026-07-18 |

## Notes

- 中等任务：design + implement；批准后 `task.py start`。
