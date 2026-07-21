# 修复 v0.0.4 发布脚本

## Goal

修复 `scripts/prepare-bundled-gateway-rust.ps1` 在 GitHub Actions Windows runner 上的解析失败（中文/编码导致 ParserError），并重新触发 `v0.0.4` 发布。

## Acceptance Criteria

- [x] 脚本使用 ASCII 消息，可被 pwsh 解析
- [ ] 推送修复后重新跑 release-windows（移动 tag 或新 tag）
- [ ] Actions 通过 prepare-bundled-gateway-rust 步骤

## Notes

根因：脚本 UTF-8 中文在 Actions 默认解析下变成乱码并打断字符串/语句。
