# 设计：Vue3 重写 + CC Switch 式路由/故障转移

## 1. 目标与边界

- **目标**：本机 Tauri 桌面应用 = Vue 3 管理 UI + Rust 内嵌本地代理；统一 `Base URL` + 强制客户端 Key；分组 = 对外 `model`；分组内有序故障转移 + 默认熔断（行为对齐 CC Switch，配置简化）。
- **不做**：旧 React / `gateway-rust` 兼容；CLI 配置改写；多 App 接管；完整熔断参数面板；非 chat 协议；旧库迁移。

## 2. 架构总览

```
┌─────────────────────────────────────────────────────────┐
│  Tauri 2 进程                                           │
│  ┌──────────────────┐    IPC / 本机 HTTP                │
│  │ Vue 3 + Tailwind │◄──────────────────────────────┐  │
│  │ 管理 UI          │                               │  │
│  └──────────────────┘                               │  │
│           │ Tauri commands（路径、代理启停、状态）    │  │
│           ▼                                         │  │
│  ┌──────────────────────────────────────────────────┴─┐│
│  │ Rust 后端                                          ││
│  │  · 配置 / SQLite 持久化                            ││
│  │  · 本地 HTTP 代理（axum 或等价）                    ││
│  │    - 管理 API（/api/v1/* 或仅 IPC，见下）           ││
│  │    - 客户端 OpenAI 兼容 /v1/*                      ││
│  │  · 路由 / 故障转移 / 熔断 / 上游转发                ││
│  └────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────┘
         ▲ 客户端（Cursor / SDK / curl）
         │  http://127.0.0.1:<port>/v1/...
         │  Authorization: Bearer sk-...
```

### 与旧架构差异

| 旧 | 新 |
|----|----|
| React UI | Vue 3 + Tailwind |
| 独立 `gateway-rust` 侧车 + 壳启停 | **代理内嵌 Tauri 进程**（类 CC Switch） |
| 请求前选渠，无失败换源 | 有序队列 + 失败换源 + 默认熔断 |
| 本地 `/v1` 可无 Key | **强制**客户端 Key |

### 管理面访问方式（选型）

- **推荐**：管理数据走 **Tauri commands**（类型安全、无 CORS）；客户端 `/v1/*` 走本机 HTTP（外部工具必须 HTTP）。
- **备选**：管理也暴露 `127.0.0.1` HTTP（便于调试）；若做 HTTP 管理 API，仍默认本机绑定，可不做管理员登录。
- MVP 可 **IPC 为主 + 可选同进程 HTTP 管理路由**；实现时二选一写死，避免双源真相。

## 3. 领域模型

### 3.1 供应商 Provider

- `id`, `name`, `base_url`, `api_key`（本机可存明文；日志脱敏）, `enabled`, `created_at`…
- MVP：OpenAI 兼容上游（`/v1/chat/completions`）。
- 可选：探测 `GET {base}/models` 拉模型列表（旧版已有，可后置）。

### 3.2 分组 Group（对外模型）

- `id`, `name`（= 客户端 `model`，唯一）, `auto_failover`（bool）, 有序 `items[]`。
- `GroupItem`：`provider_id`, `upstream_model`, `sort_order`（越小越优先）。
- 不再需要旧版复杂 mode/权重轮询作为 MVP 主路径；**顺序即故障转移优先级**（队列从头试到尾）。

### 3.3 客户端 API Key

- 创建返回一次明文；库内 `key_hash` + `masked`；`enabled`。
- 校验：`Authorization: Bearer` 或 `api-key` 头（OpenAI 习惯，实现定一种主路径 + 兼容）。

### 3.4 运行时健康 / 熔断（内存为主）

按 **provider**（或 provider+base_url）维护：

- 连续失败次数、熔断状态 Closed/Open/Half-Open、打开时间、半开成功计数。
- **默认常量**（对齐 CC Switch 通用档，可写死）：失败阈值 4、恢复等待 60s、半开成功 2、错误率阈值/最小请求数可先不做或简化为仅连续失败熔断。
- 进程重启后熔断状态清空（MVP 可接受）。

### 3.5 日志

- **请求日志**：时间、分组、最终/尝试供应商、上游 model、状态、耗时、错误摘要（不落 messages / 完整 Key）。
- **故障转移日志**：时间、原供应商、新供应商、原因（可嵌入请求日志字段或独立表）。

## 4. 请求数据流

### 4.1 Chat 转发

1. 校验客户端 Key → 401 if fail。
2. 解析 body：`model` = 分组名；`stream` 标志。
3. 加载分组队列；过滤 disabled / 熔断 Open 的条目。
4. 若自动故障转移关：只试当前首选（队列第一可用）；失败记日志返回。
5. 若开：`forward_with_retry`：
   - 对每个候选：改写 body.model → upstream_model；带上游 Key 请求。
   - 成功判定（照抄 CC Switch 精神）：
     - 非流式：成功 status + 读完 body（超时/断连 → 可重试）。
     - 流式：响应头成功 + **prime 首个 chunk**；首包超时 → 可重试。
   - 可重试错误：计失败、更新熔断、记转移日志、下一候选。
   - 不可重试（如明确 400 请求体错误）：不换源（或仅对 401/403 上游鉴权失败是否换源：建议 **可换源**，因 Key/供应商错误）。
6. 队列耗尽 → 返回聚合错误（含最后错误信息）。
7. 流式成功提交客户端后：透传剩余 SSE；静默超时记失败（可后置完整 idle timeout，先固定默认）。

### 4.2 Models

- `GET /v1/models`：鉴权后返回分组名列表（`data[].id = group.name`）。

## 5. 模块划分（Rust）

建议 `src-tauri/src/` 内清晰分层（名称可调整）：

| 模块 | 职责 |
|------|------|
| `db` | SQLite 连接、迁移 |
| `domain` / stores | provider / group / apikey / log CRUD |
| `proxy::server` | 绑定 host:port、路由注册 |
| `proxy::auth` | 客户端 Key 校验 |
| `proxy::router` | 分组 → 有序候选列表 |
| `proxy::circuit` | 熔断状态机 |
| `proxy::forward` | 上游 HTTP、retry 环、流式 prime |
| `commands` | Tauri IPC：CRUD、代理 start/stop/status、健康快照 |
| `paths` | 数据目录 |

**废弃**：`gateway/`（旧 Go/Node 若有）、整仓替换 `gateway-rust` 侧车依赖；壳不再 `sidecar` 拉独立网关二进制（代理即进程内任务）。

## 6. 前端（Vue 3）

- Vite + Vue 3 + TypeScript + Tailwind 4（与现仓 Tailwind 版本可对齐）。
- 页面（MVP）：
  1. **概览 / 代理**：Base URL 展示、端口、启停状态、复制调用示例。
  2. **供应商**：列表/表单。
  3. **分组**：名称、自动故障转移开关、拖拽队列。
  4. **API Key**：创建（展示一次明文）、列表启停删除。
  5. **日志**：请求 + 故障转移信息。
- 状态：轻量（pinia 可选）；数据经 `@tauri-apps/api` invoke。
- 删除旧 `src/` React 树，新建 Vue 入口与路由。

## 7. 持久化

- SQLite 路径：应用数据目录下（沿用 Tauri `app_data_dir` / `gateway_dir` 语义，可简化为单 `data.db`）。
- 迁移：v1 建表即可；**不**读旧版 schema 迁移数据。
- 配置：`server.host` / `server.port`（默认 127.0.0.1:随机或 8080，实现时固定默认端口并在 UI 可改）。

## 8. 错误与安全

- 日志禁止完整上游 Key / 客户端 Key / 完整 messages。
- 仅绑定本机；强制客户端 Key。
- 管理面若仅 IPC，则无 HTTP 管理攻击面。

## 9. 默认超时 / 熔断（MVP 写死）

对齐 CC Switch **通用默认**量级（可微调，不进 UI）：

| 项 | 默认 |
|----|------|
| 连续失败熔断阈值 | 4 |
| 恢复等待 | 60s |
| 半开成功关断阈值 | 2 |
| 流式首包超时 | 60s |
| 非流式总超时 | 600s（可先 120s 更严，实现时选定并文档化） |
| 流式静默超时 | 120s（可后置） |

## 10. 兼容与回滚

- **无兼容**旧安装数据；用户重装/清空数据目录。
- 回滚：Git 回退；不发布破坏性迁移。
- 发布：后续再接 NSIS/updater；MVP 以 `pnpm tauri dev` / 本地 build 验收。

## 11. 验证策略

- Rust：单元测熔断状态机、队列顺序、错误是否可重试；集成测 mock 上游（失败→换源→成功）。
- 手工：Tauri 启代理 → 建两供应商一分组 → 停主源 → chat 仍成功；无 Key 401；流式首包超时场景（mock）。
- 前端：页面 CRUD 与队列排序 smoke。

## 12. 风险

| 风险 | 缓解 |
|------|------|
| 内嵌代理与 WebView 同进程，长流占用 | async runtime；超时；后续可再拆线程 |
| 流式边界复杂 | 严格「提交前可换源」；参考 CC Switch prime |
| 整仓重写工期 | MVP 砍熔断面板、非 chat、迁移 |
| 旧 spec 仍写 React/侧车 | 实现中/完成后 `trellis-update-spec` |
