# 设计：MVP 管理 UI

## 架构

```
React 页面 ──HTTP──► http://127.0.0.1:PORT/api/v1/*
                ▲
                │ Bearer（静默/适配获取，无登录页）
Tauri invoke ──┘ 仅 gateway_status 提供 base_url / running
```

## 鉴权适配（关键，已确认上游）

- `POST /api/v1/user/login`
  - Body：`{ username, password, expire }`（`expire` 为秒）
  - 响应 data：`{ token, expire_at }`
- `GET /api/v1/user/status`（需 Bearer）校验会话
- 业务 API 请求头：`Authorization: Bearer <token>`

适配策略：

1. 网关 `running` 后静默 `login({ username: 'admin', password: 'admin', expire: 86400 })`，token 存内存（zustand 可选，**不展示登录页**）。
2. 失败：设置页显示错误 +「管理 API Token」粘贴兜底。
3. 默认凭据仅本机侧车；改密后需更新兜底 token 或后续可配置凭据。

## 前端结构

```
src/
  api/
    tauri.ts
    gatewayHttp.ts      # baseURL + token + 错误
    channel.ts
    group.ts
    log.ts
  features/ 或 pages/
    channels/
    groups/
    logs/
    settings/           # 复用现有启停
  hooks/
    useGatewayReady.ts
```

依赖：可引入 `@tanstack/react-query`（与上游一致）；或先用轻量 useEffect+fetch，以可维护为准。推荐 Query。

## 渠道创建最小载荷

对齐上游 `CreateChannelRequest`：

- `name`, `type: 'openai/chat_completions'`
- `base_urls: [{ url, delay: 0 }]`
- `keys: [{ enabled: true, channel_key }]`
- `model`: 用户填写上游模型名字符串（可多模型逗号策略按上游约定）

## 分组创建最小载荷

- `name`, `mode: 1`（RoundRobin）
- `items: [{ channel_id, model_name, priority: 1, weight: 1 }]`
- `match_regex`: 按上游默认（可空字符串若允许）

## 错误 UX

- 401：提示鉴权适配失败，引导设置 token/检查侧车
- 网络错误：提示网关是否 running
