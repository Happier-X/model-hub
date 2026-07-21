# 移除 octopus 兼容侧车代码

## Goal

删除桌面壳与发布/开发脚本中的 **octopus 运行时兼容路径**，仅保留 Rust `model-hub-gateway`。

## 删除范围

- `GatewayImpl` 双实现 / `MODEL_HUB_GATEWAY_IMPL=octopus` 启动分支
- octopus 二进制解析、`start` 子命令、`OCTOPUS_*` env
- `scripts/prepare-bundled-octopus.ps1`、`fetch-octopus-windows.ps1`、`e2e-octopus-smoke.py`
- `package.json` 的 `prepare:octopus` 及文档中的自备回退说明

## 保留

- 客户端 Key 前缀 `sk-octopus-...`（产品 API 历史命名，非侧车进程）
- `gateway-rust migrate-octopus`（一次性从旧库导入数据）
- 历史 release-notes 中已发布叙述（可不改或仅轻量）
- `third-party/octopus/` 可选保留作归档说明，更新 NOTICE 为「代码与发布均不再使用」

## Acceptance Criteria

- [x] 壳只启动 model-hub-gateway；无 octopus 命令行分支
- [x] 相关 prepare/e2e 脚本已删或不再引用
- [x] 主文档不再指导 `IMPL=octopus` 回退
- [x] cargo/pnpm 门禁通过
