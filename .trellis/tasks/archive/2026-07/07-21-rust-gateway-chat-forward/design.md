# 设计：Chat 非流式转发

## 模块

```text
gateway-rust/src/
  router/           # 分组 → channel + model_name 选择
    mod.rs
    round_robin.rs
  upstream/
    mod.rs          # reqwest client + forward_chat
  routes/
    v1_models.rs    # 改为列出 groups
    v1_chat.rs      # POST /v1/chat/completions
```

## 依赖

- `reqwest` = 精确版本，features: `json`, `rustls-tls`（或 `default-tls` Windows 友好）
- 测试：`wiremock` 或自建 `axum` mock 上游；优先 `wiremock` 精确钉扎

## 路由算法

```rust
struct RouteTarget {
  group_id, group_name,
  channel_id,
  upstream_model: String,
  base_url: String,
  channel_key: String,
  custom_header: Value,
}
```

1. `groups.find_by_name(model)`
2. 若 items 空 → error
3. mode==1：`counter.fetch_add(1) % items.len()`；其它 mode：先用 index 0（文档注明后续增强）
4. load channel by id；disabled → 尝试下一项（简单 skip）或 error
5. pick first enabled key + first base_url

`Arc<AtomicU64>` 放 `AppState` 或 `RouterService`。

## Chat handler

```text
ClientAuth → parse Json<Value>
if stream == true → 400 STREAM_NOT_SUPPORTED
resolve route
build upstream body = request with model rewritten
POST upstream
return StatusCode + bytes/json
```

错误响应格式：

```json
{
  "message": "...",
  "error": { "code": "...", "message": "..." }
}
```

上游 4xx/5xx body 原样透传（若非 JSON 仍透传 bytes + content-type）。

## `/v1/models`

```json
{
  "object": "list",
  "data": [
    { "id": "group-name", "object": "model", "owned_by": "model-hub" }
  ]
}
```

## 测试

- unit: round_robin index；stream reject；model rewrite
- integration: wiremock upstream 200；bad group；auth matrix on chat
- 临时 DB + 随机端口

## 回滚

删除 router/upstream/v1_chat，models 回空列表。
