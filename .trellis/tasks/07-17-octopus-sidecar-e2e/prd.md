# 真机侧车联调

## Goal

在 Windows 本机钉扎并运行真实 **octopus v0.9.28** Windows x64 二进制，验证 Model Hub 壳与侧车的端到端：启停、静默鉴权、管理 API、（可选）Chat 转发；修复联调中发现的兼容问题，并更新 `gateway/README.md` 精确版本。

## Requirements

### R1. 二进制钉扎
- 使用 GitHub Release **v0.9.28** 的 `octopus-windows-x86_64.zip`
- 文档写明下载 URL、解压后文件名、放置到 `bin_dir` 或 `MODEL_HUB_GATEWAY_BIN`
- 不将巨大 exe 强制提交进 Git（可提供 scripts 下载脚本）

### R2. 启停与健康
- `gateway_start` 后状态 `running`，TCP/端口可达
- `gateway_stop` / 应用退出后进程清理（或文档说明守护化行为）

### R3. 管理 API
- 静默 login admin/admin 成功
- 渠道 list / 创建（可用假 Key 测 API，不要求真实上游）
- 分组 list / 创建
- 日志 list

### R4. Chat（尽力）
- 若有可用上游 Key 环境变量则真转发；否则用 mock/文档说明跳过
- 至少验证 `/v1/models` 或 chat 路径是否可达（401/空列表也算「网关在响应」）

### R5. 修复
- 配置路径、工作目录、环境变量、守护化、健康检查误判等联调问题就地修

## Acceptance Criteria

- [ ] AC1：`gateway/README.md` 钉扎 v0.9.28 + Windows x64 下载步骤
- [ ] AC2：本机可 start 真实二进制至 running
- [ ] AC3：静默鉴权 + channel/group/log API 至少 list 成功
- [ ] AC4：联调问题已修或文档记录 workaround
- [ ] AC5：`pnpm build` / `cargo test` 仍通过

## Out of Scope

- 完整协议矩阵、价格页
- 将 exe 提交进仓库
