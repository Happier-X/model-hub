# 设计：网关 API Key 与 Chat 可验收

## 边界

| 层 | 职责 |
|----|------|
| 前端 | 管理 API Key CRUD UI、客户端提示修正 |
| 侧车 | 签发/校验 Key、`/v1/*` 转发（不改二进制） |
| 壳 | 无新 invoke；仅复用已有 base_url / 鉴权 |
| 脚本/文档 | 真机冒烟与对接说明 |

## 上游契约（v0.9.28 / 源码对照）

### 管理 API（Bearer JWT）

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/v1/apikey/create` | body：`name` 等；服务端生成 `api_key` |
| GET | `/api/v1/apikey/list` | 返回数组 |
| POST | `/api/v1/apikey/update` | body 含 `id` 等；不改 `api_key` 字符串 |
| DELETE | `/api/v1/apikey/delete/:id` | 删除 |

模型字段（源码）：

```text
id, name, api_key, enabled, expire_at?, max_cost?, supported_models?
```

### 客户端 API（API Key）

- Header：`Authorization: Bearer sk-octopus-...` 或 `x-api-key`
- 前缀约束：`sk-` + APP_NAME + `-`（APP_NAME 为 `octopus`）
- 路由：`/v1/models`、`/v1/chat/completions` 等

### 与现有前端关系

- 管理请求继续走 `gatewayHttp` + `adminToken`。
- 客户端探测请求**不能**误用 admin JWT；冒烟与文档使用网关 Key。
- `gatewayRequest` 可扩展可选 `authMode: 'admin' | 'none' | 'bearer'` 或单独 `clientRequest`，避免污染管理 token。

## UI

1. **导航**：`Sidebar` 增加「API 密钥」。
2. **ApiKeysPage**：
   - 列表：name / 脱敏 key / enabled / 操作（删除）
   - 创建表单：名称（必填）；可选 enabled 默认 true
   - 创建成功对话框/横幅：完整 Key + 复制按钮 + 警示「只完整显示一次」
3. **设置 ClientHintPanel**：改为要求网关 Key；链接文案指向「API 密钥」页。

## 数据流

```text
UI 创建 → POST /api/v1/apikey/create (JWT)
       → 展示 data.api_key
客户端 → GET /v1/models (Bearer sk-octopus-...)
       → 非 401 即鉴权闭环
Chat   → POST /v1/chat/completions
       → 需分组 model + 渠道上游；无上游时业务错误可接受
```

## 风险与兼容

| 风险 | 缓解 |
|------|------|
| v0.9.28 create body 与 dev 不一致 | 冒烟先测最小 `{name, enabled:true}` |
| list 返回 null | 与 channel 一致 `?? []` |
| 用户混淆 JWT 与 sk- | UI/文档双处强调 |
| 误杀本机 octopus | 冒烟只 free 测试端口 |

## 回滚

- 仅前端/文档/脚本；删除导航与页面即可回退；无库迁移。
