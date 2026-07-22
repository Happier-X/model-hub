# 实现清单

## 有序步骤

1. **超时常量**（`proxy/forward.rs`）
   - `NON_STREAM_TIMEOUT` → 600s
   - 新增 `STREAM_IDLE_TIMEOUT` = 120s
   - 保持首包 60s、连接 10s

2. **流式静默超时**（`proxy/forward.rs`）
   - 后续 chunk 等待使用 idle timeout
   - 超时结束流、`record_failure`、写请求日志摘要
   - 不重新进入换源循环

3. **HalfOpen 单探测**（`proxy/circuit.rs`）
   - `probe_in_flight` 标志
   - `allow_request` 只放行一个探测
   - `record_success` / `record_failure` 释放标志
   - 更新单元测试

4. **回归测试**
   - circuit：并发半开只一个通过
   - forward：超时常量；尽量覆盖 idle 超时分支
   - 保留 `tests/proxy_failover.rs` 5xx 换源

5. **验证命令**
   ```powershell
   cargo test --manifest-path src-tauri/Cargo.toml
   cargo check --manifest-path src-tauri/Cargo.toml
   pnpm typecheck
   pnpm build
   ```
   Windows 若默认 target 被占用，可用独立 `--target-dir`。

## 风险点

- 流式 body 已提交后记失败：需避免双重日志或 panic on poisoned lock。
- half-open 探测占用若请求取消未调用 record_*，可能长期占锁 —— 需在 attempt 所有路径（含网络错误）保证 record_success/failure 或显式 release。

## 回滚点

- 仅改 proxy 源码与测试；git revert 相关提交即可。
