# 执行计划：MVP 管理 UI

## Checklist

1. [ ] 调研/确认 user 登录路径与请求体；实现静默鉴权适配
2. [ ] `gatewayHttp` + channel/group/log API 模块与类型
3. [ ] 渠道页 CRUD 最小集
4. [ ] 分组页创建/列表/删除
5. [ ] 日志页列表（轮询）
6. [ ] 设置页鉴权状态与客户端提示
7. [ ] 网关未运行门禁
8. [ ] `pnpm lint` / `pnpm build`；有侧车时手工点通
9. [ ] 更新 frontend spec（api 目录、无登录 + 静默 token）

## Validation

```bash
pnpm lint
pnpm build
# 放置 octopus.exe 并启动网关后：创建渠道 → 分组 → 看日志
```

## Rollback

移除 `src/api/*` 业务模块与页面，保留网关启停壳。
