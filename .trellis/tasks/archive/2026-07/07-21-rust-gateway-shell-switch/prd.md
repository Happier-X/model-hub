# Tauri 双实现网关切换

## Goal

在 Tauri 壳中支持 **可选** 启动 `gateway-rust` 实验网关，同时 **默认保持** 内嵌 octopus 侧车路径不变；通过环境变量（及可选 shell 配置）切换实现。

## Background

- `gateway-rust` 已具备鉴权、SQLite、渠道/分组、Chat 非流式/SSE、请求日志。
- 当前 `GatewayRuntime` 固定：`octopus.exe start --config data/config.json` + OCTOPUS_* 环境变量。
- 父任务迁移策略：`MODEL_HUB_GATEWAY_IMPL=rust|octopus`，并行期保留 octopus 回退。

## Requirements

### R1. 实现选择

- 环境变量 `MODEL_HUB_GATEWAY_IMPL`：
  - 缺省 / `octopus` / 未知值 → **octopus**（现行为）
  - `rust`（大小写不敏感）→ 启动 Rust 网关二进制
- 可选：`shell.json` 增加 `gateway_impl`（若存在则低于环境变量优先级？**环境变量优先于 shell.json**；缺省仍 octopus）。为降低风险，本任务 **仅环境变量** 亦可验收；若实现 shell 字段则文档说明。

### R2. Rust 二进制解析

优先级建议：

1. `MODEL_HUB_GATEWAY_BIN`（若已设且文件存在：**两种实现都可覆盖**，与现网一致）
2. 否则当 impl=rust：`MODEL_HUB_GATEWAY_RUST_BIN` → `bin_dir/model-hub-gateway.exe` → 可文档化的开发路径查找（可选）
3. 缺失时给出可行动错误：如何 `cargo build --manifest-path gateway-rust/Cargo.toml` 并设置 env

**不**把 gateway-rust 打进当前 release NSIS（本任务不做去侧车发布）。

### R3. 启动参数差异

| 实现 | 命令 |
|------|------|
| octopus | `{bin} start --config data/config.json` + OCTOPUS_* env |
| rust | `{bin} --config data/config.json`（cwd=gateway_dir） |

- 仍写同一 `data/config.json`（host/port/sqlite 路径）。
- 端口占用、健康检查、停止托管子进程逻辑复用。

### R4. 状态与 UI

- `GatewayStatus` 可增加可选字段 `impl_name: "octopus"|"rust"`（若加字段需前端兼容：可选字段不破坏）。
- 设置页可不改 UI；文档说明如何切换。
- 默认用户路径无感知（仍 octopus）。

### R5. 测试与文档

- 单元测：impl 解析、命令行参数构造、默认 octopus。
- 不强制 CI 真启 octopus/rust 子进程。
- 更新 `gateway/README.md`、`gateway-rust/README.md`、backend spec。

## Acceptance Criteria

- [x] AC1：未设置 env 时行为与现网 octopus 完全一致。
- [x] AC2：`MODEL_HUB_GATEWAY_IMPL=rust` 时使用 rust 启动参数（非 `start` 子命令）。
- [x] AC3：rust 二进制缺失时错误可行动，不崩溃壳。
- [x] AC4：端口设置、stop、health 超时路径仍可用。
- [x] AC5：单测 + cargo fmt/test/check；pnpm lint/build 不回归。
- [x] AC6：发布包默认仍内嵌 octopus，本任务不移除 AGPL 侧车。

## Out of Scope

- 安装包内嵌 model-hub-gateway
- 从 octopus SQLite 自动迁移到 rust schema
- 设置页图形切换器（可后续）
- 删除 octopus 发布路径
