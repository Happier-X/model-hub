# 上游供应商访问约定

> 防止对用户配置的上游做测活/预热/后台探测，降低封号风险。

---

## Scenario: 用户供应商 HTTP 访问

### 1. Scope / Trigger

- Trigger：任何会使用 `providers.base_url` + `providers.api_key`（或表单草稿等价字段）向**用户配置的上游**发起 HTTP 的代码路径。
- 目标：只允许「真实业务」与「用户明确点击」两类访问。

### 2. Signatures / 入口

| 入口 | 允许条件 |
|------|----------|
| `proxy/forward.rs` 转发 Chat | 仅处理客户端发来的真实 `/v1/chat/completions` |
| `fetch_provider_models` IPC | 仅管理台用户**主动点击**「拉取模型」/批量添加时调用 |
| `list_health` | **禁止**打上游；只读进程内熔断状态 |
| 本机 `GET /health` | 仅本机代理自检，不使用供应商 Key |
| OpenRouter 榜单 `leaderboard` | 固定公共 URL，**禁止**附带用户供应商 Key |

### 3. Contracts

**允许**

1. 真实用户 Chat → 代理按队列故障转移转发上游（含熔断 HalfOpen 在**该业务请求**上占用探测位）。
2. 用户在分组页点击「拉取模型」或「批量添加供应商模型」→ `GET {base}/models`（或兼容路径）。
3. OpenRouter 公共 Models API（无用户 Key）。

**禁止**

1. 应用启动、定时器、后台任务对用户供应商做连通性检查。
2. 供应商页「测试连接」、空 chat、假 health、预热请求。
3. 打开供应商/分组页、保存供应商时**自动**拉 `/models`。
4. 为恢复熔断而**单独**发起上游请求（HalfOpen 只能挂在真实业务请求上）。
5. AI/自动化联调默认打用户上游；须用户**明确授权某次**操作。

### 4. Validation & Error Matrix

| 条件 | 行为 |
|------|------|
| 代码路径为启动/定时/测活 | **不得**发起上游 HTTP |
| 用户未点击拉取模型 | 不得调用 `fetch_provider_models` |
| 熔断 Open | 跳过该供应商；**不**另发探测请求 |
| 熔断 HalfOpen | 至多一个**真实业务请求**作为探测 |
| 错误日志 | 不得打印完整上游 Key |

### 5. Good / Base / Bad Cases

- **Good**：用户 Chat 失败后换队列下一源；用户点「拉取模型」后填入 datalist。
- **Base**：`list_health` / 「刷新健康」只显示内存态，零上游流量。
- **Bad**：供应商表单「测试连接」；保存供应商时自动 GET models；每分钟 ping 上游。

### 6. Tests Required

- 代理集成：故障转移不依赖独立测活接口。
- 审计/评审：无 `setInterval`/启动钩子调用 `fetch_provider_models` 或对 `providers.base_url` 发空请求。
- 前端：供应商页无「测试连接」类按钮；分组页拉取仅 `@click`。

### 7. Wrong vs Correct

#### Wrong

```ts
// 保存或 onMounted 自动测活
onMounted(() => fetchProviderModels({ provider_id }))
await createProvider(form)
await fetchProviderModels({ base_url, api_key }) // 测试连接
```

#### Correct

```ts
// 仅用户点击
async function pullModels(index: number) {
  await fetchProviderModels({ provider_id: form.items[index].provider_id })
}
// 健康展示
await listHealth() // 不打上游
```

---

## Anti-Patterns

- 把「刷新健康」实现成对每个供应商请求 `/v1/models` 或 chat。
- 用用户 Key 请求 OpenRouter 或其它第三方做测活。
- 在 AI 会话中未经用户同意对真实上游做联调请求。
