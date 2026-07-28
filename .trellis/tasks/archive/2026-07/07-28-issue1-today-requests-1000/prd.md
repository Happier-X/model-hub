# Issue #1: 首页今日请求总显示 1000

## Goal

修复 GitHub Issue [#1](https://github.com/Happier-X/model-hub/issues/1) "首页的今日请求
的总请求为什么总是显示 1000"。根因是后端日志保留策略 `LOG_MAX_ROWS = 1000`（硬编码常量）
与统计语义冲突：每天请求超过 1000 条时，今日旧请求被 purge 删除，导致 `COUNT(*) WHERE
time >= 今日0点` 触顶 1000。修复方案：将默认上限提高到 10000（方案 A），覆盖 99% 日活
场景，最小改动、最低回归风险。

## 根因（研究已确认）

- `src-tauri/src/domain/log.rs:63` 常量 `pub const LOG_MAX_ROWS: i64 = 1000;`
- `purge_expired_logs()` 调用 `purge_logs(LOG_RETENTION_DAYS, LOG_MAX_ROWS)`：
  第二步 `DELETE FROM request_logs WHERE id NOT IN (SELECT id ... ORDER BY id DESC LIMIT ?1)`
  强行只保留最新 1000 行。
- 每次写日志 `append_log` 会 best-effort 触发 `purge_expired_logs_best_effort()`。
- 今日发了 >1000 条请求时，库里只剩 1000 行（最新），今天的旧请求被删，导致首页"
  今日请求/总请求" 永远触顶 1000。
- `request_stats_today()` 查询本身无 LIMIT，逻辑正确；问题不在查询而在数据被裁剪。

## 范围内

- 修改 `src-tauri/src/domain/log.rs` 中的 `LOG_MAX_ROWS` 常量：`1000` → `10000`。
- 检查是否有测试硬编码断言依赖 `LOG_MAX_ROWS = 1000` 的具体数值，若有则同步更新。
- 检查文档 / UI 文案是否有"上限 1000 条"字样需要同步更新（v0.0.7 changelog 提到
  "保留最新 {{ maxRows }} 条"，若有硬编码要同步）。

## Out of Scope

- 不改 purge 策略本身（不引入"今日日志完整保留"高级语义；那是方案 C，留给后续独立子任务）。
- 不把 `LOG_MAX_ROWS` 改成设置页可配置（方案 B，改动面大，留给后续）。
- 不动 `LOG_RETENTION_DAYS`。
- 不动前端展示逻辑（前端本身无 bug）。

## Requirements

- `LOG_MAX_ROWS` 常量值 = `10000`。
- 任何硬编码断言 `LOG_MAX_ROWS == 1000` 或 `retained, 1000` 的测试同步更新。
- 文档中硬编码"1000 条"字样（如有）同步更新为"10000 条"。

## Acceptance Criteria

- [ ] `src-tauri/src/domain/log.rs` 中 `LOG_MAX_ROWS` = `10000`
- [ ] 相关测试 `cargo test --manifest-path src-tauri/Cargo.toml` 通过
- [ ] 无其他测试 / 文档硬编码 "1000" 与 `LOG_MAX_ROWS` 语义冲突
- [ ] `npm run build` / `npm run lint` 通过（前端无改动，理论上不受影响，作为冒烟)
- [ ] 已提交 `fix(backend): 日志保留上限 1000 → 10000 修复今日统计触顶 (#1)`

## Notes

- 仅后端 Rust 常量改动；前端无改动。
- 测试若用 `purge_logs(7, 1000)` 这种显式传参的不会受 `LOG_MAX_ROWS` 常量变化影响；只有
  依赖 `LOG_MAX_ROWS` 常量值默认行为的断言会受影响，需逐个核对。
- 上限从 1000 提到 10000，SQLite 增量约 10× 行（极小，对运行时性能无实质影响）。