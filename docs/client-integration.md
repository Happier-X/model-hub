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

监听地址默认 **仅本机** `127.0.0.1`。MVP 对外 LLM API **不强制网关 Key**；若 SDK 必填 `api_key`，可填任意占位字符串。

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
