# 执行计划：MVP 管理 UI

## Checklist

1. [x] 确认 login API；实现静默鉴权 + Token 兜底
2. [x] gatewayHttp + channel/group/log API
3. [x] 渠道页
4. [x] 分组页（轮询）
5. [x] 日志页轮询
6. [x] 设置页鉴权与客户端提示
7. [x] GatewayGate 门禁
8. [x] pnpm lint / build
9. [x] 更新 frontend directory-structure spec

## Validation

```bash
pnpm lint
pnpm build
# 有 octopus.exe 时：启动网关 → 渠道 → 分组 → 日志
```
