# 执行计划：渠道能力补强

## 清单

1. [x] 测试端口探测 v0.9.28 `channel/update`（name/base_urls/model 与 keys_to_update 均 200）
2. [x] `channel.ts`：`updateOpenAiChatChannel`
3. [x] `ChannelsPage`：编辑面板、Key 显示/复制、删除确认、type 说明
4. [x] `pnpm lint` / `pnpm build`；smoke 回归
5. [x] mvp-acceptance AC9
6. [x] 勾选 PRD AC

## 真机 update 结论

- 部分更新 `{id,name}` / `{id,base_urls,model,...}` 可用
- Key：`keys_to_update: [{id, channel_key}]` 可用
