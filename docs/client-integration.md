# 客户端对接

Model Hub 对外提供统一的本机 OpenAI 兼容地址。管理操作通过桌面应用完成，外部客户端只访问 `/v1/*`。

## 接入前提

1. 在应用内配置供应商。
2. 创建分组，并为分组配置有序队列。分组名就是对外模型名。
3. 创建客户端 API Key；明文仅在创建成功时展示一次。
4. 在概览页确认代理正在运行，并复制 Base URL。

## 地址与鉴权

| 项 | 默认值 |
|----|--------|
| Base URL | `http://127.0.0.1:8080` |
| OpenAI SDK Base URL | `http://127.0.0.1:8080/v1` |
| 模型列表 | `GET /v1/models` |
| Chat | `POST /v1/chat/completions` |

`/v1/*` **默认可不带**客户端 Key（本机场景）。若请求携带了 Key，则会校验有效且启用。可选：

```http
Authorization: Bearer sk-modelhub-...
```

同时兼容 `api-key` 或 `x-api-key` 请求头。代理默认仅监听 `127.0.0.1`。

## curl 示例

```bash
curl http://127.0.0.1:8080/v1/models \
  -H "Authorization: Bearer sk-modelhub-..."

curl http://127.0.0.1:8080/v1/chat/completions \
  -H "Authorization: Bearer sk-modelhub-..." \
  -H "Content-Type: application/json" \
  -d '{"model":"你的分组名","messages":[{"role":"user","content":"你好"}]}'
```

## OpenAI Python SDK

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://127.0.0.1:8080/v1",
    api_key="sk-modelhub-...",
)

completion = client.chat.completions.create(
    model="你的分组名",
    messages=[{"role": "user", "content": "你好"}],
)
print(completion.choices[0].message.content)
```

流式调用只需增加 `stream=True`。Model Hub 在首个数据块提交给客户端之前可以自动换源；提交后只透传当前上游，不会混合两路响应。

## 路由规则

- 客户端 `model` 对应分组名，而不是上游模型名。
- 队列从第一项开始按顺序尝试。
- 自动故障转移开启后，网络错误、超时和可重试的上游错误会触发下一项。
- 明确不可重试的请求错误不会盲目换源。
- 连续失败会触发默认熔断；恢复等待结束后进行半开探测。

桌面管理数据只经 Tauri IPC 读写，不提供旧式 HTTP 管理接口。
