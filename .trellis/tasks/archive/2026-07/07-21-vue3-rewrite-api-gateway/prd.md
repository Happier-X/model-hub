# Vue3 重写 Model Hub：统一 API 分发与故障转移

## Goal

用 **Vue 3 + Tailwind CSS** 重新实现本机 Model Hub：配置多条上游 AI 供应商（Provider），经本地代理汇聚为 **统一 Base URL + 客户端密钥**；路由与 **故障转移行为对齐 [CC Switch](https://github.com/farion1231/cc-switch)**（故障转移队列、失败自动换源重试、熔断与健康状态）。不兼容旧 React 代码，整仓可重做。

## Background / Confirmed Facts

### 仓库现状（参考，非兼容约束）

- 现有栈：React 19 + Tailwind + Vite + Tauri 2 + `gateway-rust`。
- 已有语义：渠道 / 分组 / 客户端 API Key、OpenAI 兼容 `POST /v1/chat/completions`（含 SSE）、`GET /v1/models`、请求日志。
- 旧路由：请求前按分组选可用渠道；**缺少**上游失败后按队列换源重试与熔断。

### 产品决策（已确认）

- **交付形态：A only（本机优先）** — 本机跑网关 + Vue 管理台；不做公网多租户/服务器部署为 MVP 目标。
- **行为参考：CC Switch 的代理路由 + 故障转移**（非照搬其 CLI 配置改写/多 App 接管全量产品）。
- **前端**：Vue 3 + Tailwind；**不兼容**旧前端与旧代码。
- **对外形态**：统一 URL + 密钥（用户核心诉求）。
- **路由模型：分组 = 对外模型名（A）** — 客户端 `model` 填分组名；每个分组各自维护有序故障转移队列（条目 = 供应商 + 上游 model_name）。
- **故障转移 MVP 深度：B（行为对齐、配置简化）** — 有序队列 + 自动换源重试 + **内置默认熔断**（阈值/恢复等待等固定，不做完整参数面板）+ 健康状态 + 故障转移日志；高级熔断/超时面板后置。
- **实现策略：整仓重写** — 不演进/不兼容现有 `gateway-rust` 与 React UI；路由与故障转移**按 CC Switch 行为照抄**（队列、换源重试、默认熔断、健康、转移日志），访问形态仍是 **统一 URL + 客户端 Key + 分组=model**。
- **进程/壳形态：A（Tauri 桌面 + 内嵌本地代理）** — Vue 3 管理 UI + Rust 本地代理同进程/同应用（类 CC Switch）；本机统一 URL + Key 对外服务。
- **技术栈**：前端 **Vue 3 + Tailwind**；后端 **Tauri 2 + Rust**（代理、故障转移、熔断状态、持久化）。旧 `src/` React、`gateway/`、`gateway-rust` 可废弃或整目录替换，不做兼容层。
- **流式换源边界：照抄 CC Switch**（见其 `forwarder.prepare_success_response_for_failover` / 首包超时）：
  - **非流式**：在 retry 环内读完完整 body 再视为成功；读超时/连接失败可换源重试。
  - **流式**：响应头 + **至少 prime 首个 chunk** 前仍可失败换源；避免上游 200 却不吐 SSE 被误记成功（首包超时可换源）。
  - **向客户端提交流之后**：进入透传；中途静默/断开记失败与熔断；**不向客户端拼接第二家供应商的半截流**（MVP 与 CC Switch 一致：换源发生在提交客户端之前）。
  - 错误分类：可重试（5xx/超时/网络等）才换源；明显客户端/不可重试错误不换源（对齐 CC Switch `ErrorCategory` 思路，细则 design 阶段定表）。

### CC Switch 故障转移 / 路由要点（要对齐的行为）

来源：CC Switch 用户手册「路由 / 故障转移」：

1. **本地代理 / 路由模式**：请求先到本地代理，再转发到当前供应商；可热切换供应商而无需客户端改配置。
2. **故障转移队列**：有序备用供应商列表；主源失败后按优先级依次尝试。
3. **自动故障转移开关**：关 = 只记失败不切换；开 = 失败自动切下一个并重试。
4. **流程**：请求 → 当前供应商 → 失败则记日志 → 查熔断 → 跳过或计失败 → 队列下一源 → 重试 → 队列耗尽则返回错误。
5. **熔断器**：连续失败阈值、恢复等待、半开探测、错误率阈值等（Closed / Open / Half-Open）。
6. **超时**：流式首字节、流式静默、非流式总超时（MVP 可先固定默认值，高级配置可后置）。
7. **健康状态**：健康 / 警告 / 熔断，展示在供应商卡片与队列。
8. **故障转移日志**：时间、原供应商、新供应商、失败原因。

## Requirements

### 已定方向

- R1：本机管理多条上游供应商（Base URL、Key、模型映射等）。
- R2：本地代理对外提供 **统一 Base URL + 客户端 API Key**（OpenAI 兼容调用形态）。
- R3：路由 + 故障转移对齐 CC Switch 模型：**有序队列 + 失败换源重试 + 熔断跳过坏源**。
- R4：管理 UI：Vue 3 + Tailwind；可配置队列顺序、启停自动故障转移、查看健康与相关日志。
- R5：MVP 仅本机；不要求数据从旧版自动迁移。
- R6：客户端 `model` = **分组名**；每组一条有序队列，条目绑定供应商 + 上游模型名。
- R7：交付为 **Tauri 2 桌面应用**（Vue 3 UI + Rust 本地代理）。
- R8：协议 MVP 仅 **Chat**：`POST /v1/chat/completions`（非流式 + SSE）与 `GET /v1/models`（分组名列表）；不含 completions/embeddings/images。
- R9：流式/非流式故障转移边界照抄 CC Switch（首包前可换源；提交客户端后不拼双流）。
- R10：**强制客户端 API Key** — 管理端创建/启停 Key；`/v1/*` 必须 `Authorization: Bearer <key>`（或等价 OpenAI Key 头）；校验失败 401；创建时仅一次展示明文，存储哈希+脱敏。

## Acceptance Criteria

### 访问形态

- [ ] AC1：本机代理默认监听 `127.0.0.1`（端口可配置）；客户端使用统一 `Base URL` + 客户端 API Key 调用。
- [ ] AC2：无有效客户端 Key 访问 `POST /v1/chat/completions` 或 `GET /v1/models` 返回 **401**。
- [ ] AC3：有效 Key 下，`GET /v1/models` 返回已配置**分组名**列表（OpenAI models 形态）。
- [ ] AC4：有效 Key 下，`POST /v1/chat/completions` 中 `model` = 分组名时，转发到该分组队列中的可用上游；上游请求体中的 `model` 替换为队列条目的上游模型名。

### 故障转移（对齐 CC Switch 行为，配置简化）

- [ ] AC5：每个分组维护**有序**故障转移队列（条目 = 供应商 + 上游 model）；UI 可调整顺序、增删条目。
- [ ] AC6：开启自动故障转移时，可重试失败（超时/网络/5xx 等）按队列顺序换源重试，直到成功或队列耗尽；关闭时仅记录失败不换源。
- [ ] AC7：**默认熔断**生效：连续失败达内置阈值后跳过该供应商，恢复等待后半开探测；UI 展示健康 / 警告 / 熔断（无完整熔断参数配置面板）。
- [ ] AC8：**非流式**：在返回客户端前读完上游 body；读失败/超时可换源。**流式**：至少 prime 首包前可换源；一旦向客户端提交流则透传，不拼接第二家半截流。
- [ ] AC9：故障转移事件可查：时间、原供应商、新供应商、失败原因（可与请求日志同区展示）。

### 管理 UI 与桌面

- [ ] AC10：Vue 3 + Tailwind 管理台可 CRUD 供应商、分组（含队列）、客户端 API Key，查看请求/故障转移相关日志与健康状态。
- [ ] AC11：Tauri 2 应用内嵌本地代理；应用内可启停/查看代理运行状态与本机 Base URL。
- [ ] AC12：不依赖旧 React 页面与旧 `gateway-rust` 管理 API 契约；旧数据无自动迁移要求。

## Out of Scope（MVP）

- 公网部署、多租户 SaaS、计费
- 兼容现有 React UI / 旧管理 API 契约
- 旧版 SQLite 自动迁移
- 完整复刻 CC Switch：CLI 配置改写、系统托盘、多 App 接管、MCP/Skills、格式互转整流等
- 完整熔断/超时高级配置面板（后置；MVP 用内置默认值）
- `completions` / `embeddings` / `images` 等非 chat 协议

## Open Questions

（产品决策已收敛；实现细节见 `design.md`。）

## Notes

- 复杂任务：已具备可验收 PRD；需 `design.md` + `implement.md` 经用户评审后再 `task.py start`。
- 行为参考：CC Switch failover queue + circuit breaker + local proxy；访问形态：统一 URL + 客户端 Key + 分组=model。
