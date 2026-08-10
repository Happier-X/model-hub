# Journal - happier (Part 2)

> Continuation from `journal-1.md` (archived at ~2000 lines)
> Started: 2026-08-10

---



## Session 41: 首页统计总览：token/耗时记录 + 聚合（总计+今日）

**Date**: 2026-08-10
**Task**: 首页统计总览：token/耗时记录 + 聚合（总计+今日）
**Branch**: `master`

### Summary

转发链路记录 token（非流式解析 usage + 流式注入 include_usage 旁路解析）；request_logs 加 input_tokens/output_tokens；get_request_overview 统计成功请求（总计+今日）；首页顶部统计总览两行卡片替换今日请求卡片

### Main Changes

- 后端：request_logs 加 input/output_tokens（ensure 幂等迁移 + insert 双写兼容旧库）
- 转发：非流式响应解析 usage；流式注入 stream_options.include_usage（OpenAI 兼容家族白名单）+ 透传流旁路解析 chunk usage
- 统计：get_request_overview 返回 total/today（成功口径 2xx 且 error 空），cost 预留恒 0
- 前端：首页统计总览卡片（两行×6 指标：请求次数/输入/输出/总 tokens/耗时/费用-）+ formatDuration 工具

### Git Commits

(No commits - planning session)

### Testing

- [OK] cargo test --lib 133 全绿（新增 overview 3 + usage/注入 7）；node test 33 全绿（formatDuration 4）

### Status

[OK] **Completed**

### Next Steps

- 用户 dev 验证：发非流式+流式请求后首页数字增长
