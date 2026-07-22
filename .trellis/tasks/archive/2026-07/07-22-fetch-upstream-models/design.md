# 设计：拉取上游 models

## 边界

| 层 | 职责 |
|----|------|
| `domain` 或 `proxy` 旁路小模块 | HTTP GET + 解析，不经故障转移队列 |
| `commands.rs` | `fetch_provider_models` 薄封装 |
| `src/api/tauri.ts` | 前端类型与 invoke |
| `GroupsPage.vue` | 拉取按钮 + 选择列表 / datalist |

## 契约

```text
fetch_provider_models(payload) -> string[]

payload:
  - { provider_id: number }                    // 优先
  - { base_url: string, api_key: string }      // 草稿探测（可选）

错误：Business / 专用 code，message 中文，如
  - 供应商不存在
  - 上游返回 401：请检查 API Key
  - 无法解析模型列表
  - 请求超时
```

## 数据流

```text
UI 选 provider_id
  → IPC fetch_provider_models
  → 读 Provider(base_url, api_key)
  → GET {base_url}/models
  → 解析 data[].id
  → 返回 Vec<String>
  → UI 展示可选列表，用户点选写入 upstream_model
```

## 安全

- 日志与错误信息禁止完整 Key。
- 仅本机 IPC 触发；不新增对外 HTTP 管理接口。

## 测试

- 解析函数单测：标准 JSON、空 data、缺字段。
- 可选：wiremock 集成测 command 或 fetch 函数。
