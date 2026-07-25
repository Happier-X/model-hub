# 代理转发剥离 tools[].function.strict 字段

## 背景

model-hub 作为 OpenAI 兼容代理网关，把客户端（如 pi、其他 LLM Agent）的
`/v1/chat/completions` 请求转发给下游 provider。

pi 底层使用较新版本 OpenAI SDK，构造 tools 时会带上 `strict: true` 字段
（Structured Outputs 特性，`gpt-4o-2024-08-06+` 才支持）。当下游 provider 不认
`strict` 字段时会直接返回错误：

```
tool.function.strict is not supported
```

当前 `src-tauri/src/proxy/forward.rs` 中的 `rewrite_model` 只重写了顶层 `model`
字段，`tools` 数组被原样透传，导致该错误从下游一路穿透回客户端。

## 目标

在网关侧统一剥离 `tools[].function.strict` 字段，让不支持该字段的下游 provider
能正常处理请求；对支持 `strict` 的下游只是失去了严格结构化输出，功能层面不会
新增故障。

## 需求

- 在请求体转发给上游之前，若 `tools` 存在且为数组，遍历每个 element，若其
  `function` 是对象且包含 `strict` key，则删除该 key。
- 同样规则处理 `tool_choice` 里可能出现的 `function.strict`（部分 SDK 会写在这里）。
- 处理逻辑对 **流式** 与 **非流式** 两条路径都要生效（当前两条路径都走
  `rewrite_model`）。
- 处理必须是幂等且宽容的：`tools` 不存在 / 不是数组 / `function` 不是对象 /
  没有 `strict` 都要安全跳过，不能 panic 或误改其它字段。
- 除 `model` / `tools[].function.strict` / `tool_choice.function.strict` 外，
  body 其余字段保持原样透传。

## 非目标

- 不解析 `parameters` 内部 JSON Schema，也不移除 `additionalProperties: false`
  等 strict 相关的 schema 约束。这些字段单独存在时对绝大多数下游无害。
- 不做 provider 级开关（例如"仅对某 provider 剥离"），当前一律剥离即可，未来
  如需保留由后续任务处理。
- 不改客户端行为，也不动 pi 侧配置。

## 验收标准

- [ ] `forward.rs` 中新增/改造后的函数会剥离 `tools[].function.strict` 与
      `tool_choice.function.strict`。
- [ ] 新增单元测试覆盖：
  - [ ] `tools` 数组中每个 `function.strict` 均被删除。
  - [ ] `function` 中的其它字段（`name`、`description`、`parameters`）保持原值。
  - [ ] `tool_choice` 为 `{ "type": "function", "function": { ... "strict": true } }`
        时，`strict` 被删除。
  - [ ] `tools` 不存在 / `tools` 非数组 / `function` 非对象 时不 panic 且 body 不变。
  - [ ] 顶层 `model` 仍被重写为 `upstream_model`（既有行为不回归）。
- [ ] `cargo test`（`src-tauri` 目录）全部通过。
- [ ] 手动使用 pi 触发一次带 tools 的对话，不再看到
      `tool.function.strict is not supported` 报错。

## 备注

- 修改集中在 `src-tauri/src/proxy/forward.rs`。当前 `rewrite_model` 可以直接扩展，
  或改名为 `prepare_upstream_body`；两种任选，只要两条路径都调用即可。
- 只在网关做这层清洗，不需要新配置项。
