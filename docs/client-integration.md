# 客户端对接（MVP）

> 端到端步骤、错误对照与仪表盘自检见 **[Chat 上手与故障排查](./chat-onboarding.md)**。

## 前提

1. **安装版**无需自备 `octopus.exe`（内嵌 v0.9.28，启动时自动部署）。**开发**请见 [gateway/README.md](../gateway/README.md)：`pnpm prepare:octopus` 或设置 `MODEL_HUB_GATEWAY_BIN`。
2. 运行发行版或 `pnpm tauri dev`，在应用内确认网关状态为 **运行中**。
3. 在 **渠道** 页创建 OpenAI Chat 兼容上游（Base URL + 上游 API Key + 模型名）。
4. 在 **分组** 页创建分组，**分组名** 将作为客户端的 `model`；负载默认 **轮询**。
5. 在 **API 密钥** 页创建网关客户端 Key（前缀 `sk-octopus-`），创建成功后**完整复制一次**明文。

## 两套凭证（务必区分）

| 用途 | 凭证 | 获取方式 |
|------|------|----------|
| 管理 API（`/api/v1/*`） | 管理 JWT | 应用静默 `admin` 登录；设置页可粘贴 Token 兜底 |
| 客户端 OpenAI 兼容（`/v1/*`） | 网关 API Key（`sk-octopus-...`） | 应用 **API 密钥** 页创建；Header `Authorization: Bearer ...` 或 `x-api-key` |

**不要**把管理 JWT 当作客户端 `api_key`；**不要**使用任意占位字符串——错误 Key 会返回 **401**。

## 默认地址

| 项 | 值 |
|----|-----|
| 网关 Base | `http://127.0.0.1:8080` |
| OpenAI 兼容根 | `http://127.0.0.1:8080/v1` |
| Chat 路径 | `POST /v1/chat/completions` |
| 模型列表 | `GET /v1/models` |

监听地址默认 **仅本机** `127.0.0.1`。

**注意（octopus v0.9.28）**：

- 管理台 JWT 与客户端网关 Key **不是同一套**。
- `/v1/*` **必须**使用侧车签发的网关 API Key（前缀 `sk-octopus-`）。
- 渠道类型字段在该版本为数字；Model Hub 创建 OpenAI Chat 时使用 `type: 0`。

## curl 示例

将 `sk-octopus-YOUR_KEY` 换成你在 **API 密钥** 页复制的完整 Key；将 `your-group-name` 换成分组名。

```bash
# 探测鉴权（期望非 401；空模型列表可接受）
curl http://127.0.0.1:8080/v1/models \
  -H "Authorization: Bearer sk-octopus-YOUR_KEY"

# Chat 转发（需已配置渠道 + 分组 + 真实上游 Key）
curl http://127.0.0.1:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer sk-octopus-YOUR_KEY" \
  -d "{
    \"model\": \"your-group-name\",
    \"messages\": [{\"role\": \"user\", \"content\": \"你好\"}]
  }"
```

## Python（OpenAI SDK）

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://127.0.0.1:8080/v1",
    api_key="sk-octopus-YOUR_KEY",  # 网关 API Key，非管理 JWT
)

completion = client.chat.completions.create(
    model="your-group-name",
    messages=[{"role": "user", "content": "你好"}],
)
print(completion.choices[0].message.content)
```

## 管理 API 说明

桌面管理 UI **无登录页**，在网关运行后会静默调用：

`POST /api/v1/user/login`（默认 `admin` / `admin`）

业务管理接口形如 `/api/v1/channel/*`、`/api/v1/group/*`、`/api/v1/apikey/*`、`/api/v1/log/*`，需要 Bearer **管理 Token**。若你修改了侧车默认密码，请在应用 **设置** 中粘贴有效管理 Token。

创建网关客户端 Key：

`POST /api/v1/apikey/create`，body 最小示例：`{"name":"local-client","enabled":true}`（需管理 JWT）。响应中的 `api_key` 仅完整展示一次。

## 许可证

侧车 octopus 为 AGPL-3.0。分发或修改时请遵守其许可证并保留致谢。详见 `gateway/README.md` 与根 `README.md`。
