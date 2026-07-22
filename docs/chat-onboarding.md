# Chat 上手与故障排查

本文介绍如何把 OpenAI 兼容客户端接入本机 Model Hub，并验证模型列表、非流式 Chat 与流式 Chat。

## 准备工作

1. 安装并启动 Model Hub，或在源码目录运行 `pnpm tauri dev`。
2. 在「供应商」页创建至少一个 OpenAI 兼容上游，填写 Base URL、上游 API Key。
3. 在「分组」页创建分组：
   - 分组名就是客户端请求中的 `model`；
   - 队列条目选择供应商并填写对应的上游模型名（可点「拉取模型」从上游 `/v1/models` 选择，仍可手改）；
   - 多个条目按从上到下的顺序尝试；
   - 需要失败后自动尝试下一条时，开启自动故障转移。
4. 在「API 密钥」页创建客户端 Key，并立即保存仅展示一次的明文。
5. 在「概览」页确认代理为「运行中」，复制 Base URL。默认地址是 `http://127.0.0.1:8080`。

## 验证模型列表

将示例 Key 替换为刚创建的完整客户端 Key：

```bash
curl -i http://127.0.0.1:8080/v1/models \
  -H "Authorization: Bearer sk-modelhub-..."
```

期望返回 HTTP 200，`data[].id` 是已配置的分组名。没有有效 Key 时应返回 HTTP 401。

## 验证非流式 Chat

```bash
curl http://127.0.0.1:8080/v1/chat/completions \
  -H "Authorization: Bearer sk-modelhub-..." \
  -H "Content-Type: application/json" \
  -d '{"model":"你的分组名","messages":[{"role":"user","content":"你好"}]}'
```

客户端的 `model` 必须填写分组名。Model Hub 会在转发前将其替换为当前队列条目的上游模型名。

## 验证流式 Chat

```bash
curl -N http://127.0.0.1:8080/v1/chat/completions \
  -H "Authorization: Bearer sk-modelhub-..." \
  -H "Content-Type: application/json" \
  -d '{"model":"你的分组名","stream":true,"messages":[{"role":"user","content":"你好"}]}'
```

流式请求在向客户端提交首个数据块前允许换源；一旦开始向客户端输出，就不会拼接另一家供应商的后半段响应。

## 常见问题

| 现象 | 常见原因 | 处理方式 |
|------|----------|----------|
| 无法连接 | 代理未运行或端口错误 | 在概览页启动代理并核对 Base URL |
| HTTP 401 | Key 缺失、错误、禁用或已删除 | 创建或启用客户端 Key，并发送 `Authorization: Bearer ...` |
| 模型列表为空 | 尚未创建分组 | 创建分组并配置至少一个队列条目 |
| 提示模型不存在 | `model` 使用了上游模型名 | 改为分组名 |
| Chat 返回上游错误 | Base URL、上游 Key 或模型名错误 | 检查供应商与队列条目；在日志页查看错误摘要 |
| 主供应商失败后未换源 | 自动故障转移关闭或没有备用条目 | 开启自动故障转移并配置多个有序条目 |
| 某供应商被跳过 | 连续失败触发熔断 | 在健康状态中确认；等待恢复窗口后会进入半开探测 |

日志不会保存完整客户端 Key、上游 Key 或消息正文。

## 相关文档

- [客户端对接](./client-integration.md)
- [MVP 验收清单](./mvp-acceptance.md)
- [当前架构](./current-architecture.md)
