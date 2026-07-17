# MVP 管理 UI

## Goal

在无登录桌面壳上，通过 **HTTP** 对接网关侧车管理 API，提供渠道 / 分组 / 日志 / 设置页面，完成「配置上游 → 建分组 → 查看请求日志」闭环（对齐父任务 M1 的 AC1 与部分 AC2–AC4）。

## Parent

- 父任务：`07-17-tauri-port-octopus`
- 依赖：`mvp-scaffold`、`mvp-gateway-sidecar`（已归档）

## Background（上游 API 已探查）

octopus 管理 API（前端 `web/src/api`）：

| 域 | 方法 | 路径 |
|----|------|------|
| 渠道列表 | GET | `/api/v1/channel/list` |
| 渠道创建 | POST | `/api/v1/channel/create` |
| 渠道更新 | POST | `/api/v1/channel/update` |
| 渠道删除 | DELETE | `/api/v1/channel/delete/{id}` |
| 渠道启用 | POST | `/api/v1/channel/enable` |
| 分组列表 | GET | `/api/v1/group/list` |
| 分组创建 | POST | `/api/v1/group/create` |
| 分组更新 | POST | `/api/v1/group/update` |
| 分组删除 | DELETE | `/api/v1/group/delete/{id}` |
| 日志列表 | GET | `/api/v1/log/list` |
| 日志清空 | DELETE | `/api/v1/log/clear` |

- 客户端使用 `Authorization: Bearer <token>`；401 触发登出。
- 本产品 **无管理登录 UI**（D3）：本任务必须提供 **本机会话适配**（例如启动后静默登录默认 admin、或可配置 token、或文档化临时策略），使业务 API 可调用；不向用户展示登录页。
- 渠道类型 MVP 优先：`openai/chat_completions`。
- 分组 mode：`RoundRobin=1` 等（至少支持一种，文档写明）。

## Requirements

### R1. HTTP 客户端

- `src/api/gatewayHttp.ts`（或等价）：baseURL 来自 `gatewayStatus().base_url`；统一解析 `{ data }` 包装与错误 message。
- 请求注入适配后的 Bearer token（无 UI 登录）。
- 网关未 running 时页面展示「请先启动网关」，不假装空列表成功。

### R2. 渠道页

- 列表：loading / empty / error。
- 创建 OpenAI Chat 渠道：名称、Base URL、API Key（password 输入）、启用。
- 删除或启用/禁用（最少：创建 + 列表 + 删除或禁用其一）。
- 不展示完整 Key（遮罩）。

### R3. 分组页

- 列表；创建分组：名称、mode（默认轮询或故障转移）、绑定至少一个渠道 + model_name。
- 删除分组。
- 说明：客户端 `model` 参数使用分组名。

### R4. 日志页

- 分页或简单列表展示最近请求日志（字段：时间、模型、渠道、tokens、错误摘要）。
- 可选：清空日志。SSE 实时推送可后置，MVP 轮询 list 即可。

### R5. 设置页

- 保留路径与网关启停（已有）。
- 展示 base_url；简短客户端对接提示（占位 api_key、model=分组名）。
- 若有静默登录状态，展示「管理 API 已就绪 / 失败原因」，无密码输入框作为主路径（可开发者调试字段可选）。

## Out of Scope

- 价格管理、Dashboard 图表、多协议渠道类型齐全
- 像素级复刻 octopus UI
- 完整 E2E Chat 文档（下一子任务 `mvp-e2e-docs`）
- 修改上游 Go 源码大改（优先配置/静默登录适配）

## Acceptance Criteria

- [x] AC1：无登录页；实现静默 `/api/v1/user/login` + 手动 Token 兜底；running 后可加载业务页。
- [x] AC2：渠道页可创建 OpenAI Chat 渠道并列表/删除/启停。
- [x] AC3：分组页可创建轮询分组并列表/删除；UI 说明 model=分组名。
- [x] AC4：日志页轮询 list（可为空）。
- [x] AC5：`GatewayGate` 在未运行/鉴权失败时明确提示。
- [x] AC6：`pnpm build` / `pnpm lint` 通过。

说明：完整真侧车联调依赖本机 `octopus.exe`；代码路径与上游 API 已对齐。

## Open Risks

- 静默 admin 登录是否稳定、默认密码是否可配置。
- 若上游强制改密后静默失败：设置页需展示错误与手动 token 兜底。
