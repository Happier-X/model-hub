# 客户端对接（MVP）

## 前提

1. 按 [gateway/README.md](../gateway/README.md) 将 Windows 版 `octopus.exe` 放到应用 `bin_dir`，或设置 `MODEL_HUB_GATEWAY_BIN`。
2. 运行 `pnpm tauri dev`（或发行版 exe），在应用内确认网关状态为 **运行中**。
3. 在 **渠道** 页创建 OpenAI Chat 兼容上游（Base URL + 上游 API Key + 模型名）。
4. 在 **分组** 页创建分组，**分组名** 将作为客户端的 `model`；负载默认 **轮询**。

## 默认地址

| 项 | 值 |
|----|-----|
| 网关 Base | `http://127.0.0.1:8080` |
| OpenAI 兼容根 | `http://127.0.0.1:8080/v1` |
| Chat 路径 | `POST /v1/chat/completions` |

监听地址默认 **仅本机** `127.0.0.1`。

**注意（octopus v0.9.28）**：

- 管理台 JWT（静默 admin 登录）与客户端调用 Key **不是同一套**。
- `/v1/*` 可能要求侧车签发的网关 API Key（前缀常见为 `sk-octopus-`）。若返回 401，请在侧车/上游管理能力中创建 API Key，或后续版本做免鉴权适配。
- 渠道类型字段在该版本为数字；Model Hub 创建 OpenAI Chat 时使用 `type: 0`。

## curl 示例

```bash
curl http://127.0.0.1:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer sk-placeholder" \
  -d "{
    \"model\": \"your-group-name\",
    \"messages\": [{\"role\": \"user\", \"content\": \"你好\"}]
  }"
```

将 `your-group-name` 换成你在 Model Hub 中创建的 **分组名**。

## Python（OpenAI SDK）

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://127.0.0.1:8080/v1",
    api_key="sk-placeholder",
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

业务管理接口形如 `/api/v1/channel/*`、`/api/v1/group/*`、`/api/v1/log/*`，需要 Bearer Token。若你修改了侧车默认密码，请在应用 **设置** 中粘贴有效管理 Token。

## 许可证

侧车 octopus 为 AGPL-3.0。分发或修改时请遵守其许可证并保留致谢。详见 `gateway/README.md` 与根 `README.md`。
