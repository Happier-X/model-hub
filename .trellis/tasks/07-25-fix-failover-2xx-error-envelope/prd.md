# PRD: 修复故障转移耗尽时 2xx 错误信封透传问题

## 问题描述

当上游供应商（如 xAI/Grok）返回 HTTP 200 但 body 为结构化 JSON 错误信封时（如 `{"error":"Invalid or expired credentials..."}`），`is_structured_error_body()` 能正确检测并触发故障转移。但当**所有候选上游均已耗尽且最后一个失败响应是 HTTP 2xx 错误信封**时，exhausted 处理逻辑会**原样透传该 200 响应**给客户端，而不是返回一个明确的状态码（如 502）。

这导致：
- 客户端收到 HTTP 200（成功状态码）但 body 中是错误信息
- 客户端库可能不将 200 视为错误，导致错误被误吞或难以诊断
- 日志虽记录了 failover，但用户侧看到的是误导性的 200

## 影响范围

- `src-tauri/src/proxy/forward.rs` — `forward_with_failover()` 的 exhausted 分支
- 需新增测试覆盖该场景

## 验收条件

1. 如果所有候选上游均耗尽且最后一个错误是 HTTP 2xx 错误信封，则最终响应状态码应为 502（Bad Gateway），而非原样透传 200
2. 响应 body 应包含原始上游错误信息，便于诊断
3. 既有非 2xx HTTP 错误的透传行为保持不变
4. 新增集成测试覆盖「2xx 错误信封耗尽」场景
