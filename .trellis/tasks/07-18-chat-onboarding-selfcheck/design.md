# 设计：真上游 Chat 上手与闭环自检

## 边界

| 层 | 职责 |
|----|------|
| 文档 | 上手步骤 + 错误对照 |
| 前端 Dashboard | 自检表单与结果展示 |
| HTTP | 用**用户输入的** `sk-octopus-...` 打 `/v1/*` |
| 壳 | 无新 invoke |

## HTTP 客户端

扩展 `gatewayHttp.ts`（推荐）：

```ts
gatewayRequest(method, path, body, { auth: 'admin' | 'none' | 'bearer', bearer?: string })
```

或新增：

```ts
clientRequest<T>(method, path, { bearer, body })
```

规则：

- 自检 **禁止** 默认附带 `adminToken`。  
- Base URL 仍来自 `baseUrlProvider`（运行中网关）。

## UI：ClientSelfCheck

```text
props: baseUrl, running
state: key, model, busy, results[]
onRun:
  if !running → 提示先启动
  if !key → 提示
  GET /v1/models with Bearer key
  if model trimmed:
    POST /v1/chat/completions { model, messages:[{role:user,content:"ping"}] }
render: 每步 status + 截断 message
```

不把结果写入 localStorage。

## 文档结构

`docs/chat-onboarding.md`：

1. 你需要准备什么  
2. 五分钟配置  
3. 验证 models  
4. 验证 chat（真上游）  
5. 错误对照  
6. 与仪表盘自检的关系  

链接：`README.md`、`docs/client-integration.md`、`docs/mvp-acceptance.md`。

## 回滚

- 删文档区块与自检组件；还原 gatewayHttp 可选参数。
