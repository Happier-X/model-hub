# 去侧车发布准备

## Goal

将 Model Hub **默认网关实现切换为 `gateway-rust`**，并从 Windows 发布包中**移除内嵌 octopus 二进制与 AGPL 合规附件**；保留可选 env 回退 octopus（开发/高级用户自备二进制）。

## Background

- 双轨阶段：默认 octopus + 内嵌 rust 实验网关。
- Rust 网关已具备鉴权、SQLite、渠道/分组、Chat/SSE、日志、迁移工具与安装态资源部署。
- 父任务目标：去掉外部 AGPL 二进制依赖。

## Requirements

### R1. 默认实现

- `MODEL_HUB_GATEWAY_IMPL` 缺省 → **rust**
- 显式 `octopus` → 仍可启动 octopus（需自备二进制或 `MODEL_HUB_GATEWAY_BIN`）
- 未知值：按 **rust** 处理（或文档明确）；推荐未知值 → rust 与默认一致

### R2. 发布资源

- `tauri.release.conf.json`：**删除** octopus.exe 与 `third-party/octopus/` 资源映射；仅保留 `sidecar/model-hub-gateway.exe`
- CI：删除 prepare octopus 步骤与 AGPL 合规资产上传
- `package.json` `release:windows`：仅 prepare gateway-rust
- 二进制仍不进 Git

### R3. 运行时解析

- rust 路径：资源部署为主（与现有一致）
- octopus 路径：不再依赖安装资源；缺失时给出「默认已改为 rust / 若需 octopus 请自备并设 IMPL」的可行动错误
- 健康检查、端口、stop 逻辑不变

### R4. 文档与合规

- 更新 README、gateway/README、gateway-rust/README、NOTICE 叙事：
  - 默认 rust；发布包不再分发 octopus 二进制
  - `third-party/octopus/` 可保留在仓库作历史参考，或更新 NOTICE 声明「发布包不再内嵌」
- 客户端 Key 前缀仍可为 `sk-octopus-`（兼容，文档说明历史命名）
- 数据：升级用户需 migrate-octopus 或新建库；勿混用

### R5. 测试

- 默认 resolve → rust
- command_args 默认路径
- octopus 无资源时错误文案
- cargo/pnpm 门禁

## Acceptance Criteria

- [x] AC1：未设 env 时 `resolve_gateway_impl() == Rust`
- [x] AC2：release conf 不再包含 octopus/third-party 资源
- [x] AC3：CI 不再 download octopus / 上传 AGPL 附件
- [x] AC4：显式 IMPL=octopus 仍可解析自备二进制
- [x] AC5：文档声明默认 rust 与迁移注意
- [x] AC6：单测 + cargo/pnpm 门禁通过

## Out of Scope

- 自动为每位用户迁移 db
- 设置页 UI 切换
- 改 Key 前缀为 sk-model-hub
- 删除仓库内 third-party/octopus 历史文件（可选保留）
