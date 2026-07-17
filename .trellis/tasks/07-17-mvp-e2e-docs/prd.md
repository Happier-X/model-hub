# MVP E2E 与客户端文档

## Goal

补齐 M1 对外文档与验收清单：如何放置侧车、启动应用、配置渠道/分组、用 OpenAI 兼容客户端调用；汇总 AC 状态与 AGPL 致谢。

## Requirements

- [x] `docs/client-integration.md`：base_url、model=分组名、占位 api_key、curl/Python 示例
- [x] `docs/mvp-acceptance.md`：AC1–AC6 对照与手工验收步骤
- [x] 根 README 链接上述文档
- [x] 许可证/致谢不遗漏

## Acceptance Criteria

- [x] 文档可独立指导用户完成「装 exe → 启应用 → 配渠道分组 → 调 Chat」
- [x] 标明未在本环境完成真实 octopus 二进制联调的边界
