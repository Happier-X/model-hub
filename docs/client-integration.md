# 客户端对接

Model Hub 对外提供统一的本机 OpenAI 兼容地址。管理操作通过桌面应用完成，外部客户端只访问 `/v1/*`。

## 接入前提

1. 在应用内配置供应商（含上游 API Key）。
2. 创建分组，并为分组配置有序队列。分组名就是对外模型名。
3. 在概览页确认代理正在运行，并复制 Base URL。
4. （可选）在「分组」页对某个分组点「配置到 Pi」，按分组名 upsert 写入本机 `~/.pi/agent/models.json` 的 `model-hub`。

## 地址与鉴权

| 项 | 默认值 |
|----|--------|
| Base URL | `http://127.0.0.1:8888` |
| OpenAI SDK Base URL | `http://127.0.0.1:8888/v1` |
| 模型列表 | `GET /v1/models` |
| Chat | `POST /v1/chat/completions` |

本机 `/v1/*` **不校验**客户端 API Key：请求可不带 `Authorization`，也可带任意值（代理忽略客户端鉴权头）。默认仅监听 `127.0.0.1`。

供应商上游 Key 仍由应用在转发时写入 `Authorization: Bearer <供应商Key>`，与客户端无关。

## curl 示例

```bash
curl http://127.0.0.1:8888/v1/models

curl http://127.0.0.1:8888/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model":"你的分组名","messages":[{"role":"user","content":"你好"}]}'
```

## OpenAI Python SDK

```python
from openai import OpenAI

client = OpenAI(
    base_url="http://127.0.0.1:8888/v1",
    # 本机不校验客户端 Key；SDK 常要求非空字符串，可用任意占位如 model-hub
    api_key="model-hub",
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

## 一键配置到 Pi Agent

| 项 | 说明 |
|----|------|
| 入口 | 管理台「分组」页 → 列表行 **配置到 Pi**（按分组） |
| 目标文件 | `~/.pi/agent/models.json`（Windows：`%USERPROFILE%\.pi\agent\models.json`） |
| 写入节点 | 单一 `providers.model-hub`（upsert 模型；保留其它供应商与其它 model-hub 模型） |
| baseUrl | 当前代理 Base URL 规范为含 `/v1`，并刷新 provider 级地址 |
| models | 该条 `id`/`name` = **分组名**；同 id 替换，不同分组累积 |
| apiKey | 固定占位 `model-hub`（仅供 Pi UI 显示模型；代理不校验） |

配置后在 Pi 中使用 `/model` 选择 `model-hub/<分组名>`。代理改口后请对该分组再点一次「配置到 Pi」。分组改名不会自动删除 Pi 中旧 id 条目。
