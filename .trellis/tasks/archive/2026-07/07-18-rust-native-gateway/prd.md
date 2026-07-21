# Rust 原生网关（替换内嵌 octopus 侧车）

> **状态**：父任务规划（路线图）。**本任务不实现**；待阶段 1 内嵌侧车发布与验收稳定后再拆子任务启动。

## Goal

用 **Rust 原生网关** 替换 Model Hub 当前内嵌的 **octopus（Go）侧车**，在保持管理 UI / 客户端 OpenAI 兼容契约的前提下，去掉外部 AGPL 二进制依赖，统一技术栈与进程模型。

## Background

- 阶段 1（任务 `07-18-release-bundled-sidecar`）通过 GitHub Actions 发布 Windows 安装包，内嵌钉扎 octopus **v0.9.28**，满足「用户零手工侧车」与公开分发合规材料。
- 长期决策（D2）：内嵌发布的同时规划 Rust 原生迁移。
- 壳与侧车边界已在 `.trellis/spec/backend/` 约定：业务以 HTTP + SQLite 为准；壳只负责启停/路径/托盘。

## 非目标（本规划任务）

- 不实现任何网关业务代码。
- 不删除现有 octopus 内嵌路径（迁移完成前双轨或特性开关另议）。
- 不采购代码签名、不做 macOS/Linux 首发。

## 规划范围（父任务大纲）

### P1. 契约冻结与兼容矩阵

- 管理 API：`/api/v1/*`（JWT）与现网 UI 字段对齐清单。
- 客户端：`/v1/models`、`/v1/chat/completions`（含流式）与 `sk-octopus-...` **或** 新产品 Key 前缀策略。
- 配置：`data/config.json`、默认 `127.0.0.1:8080`、SQLite 路径。
- 渠道 `type`、分组负载策略、日志 list 分页等行为矩阵（对照 v0.9.28 实测）。

### P2. 核心能力切片（建议子任务）

| 切片 | 说明 |
|------|------|
| HTTP 服务骨架 | 本机绑定、优雅退出、健康检查 URL |
| 鉴权 | 管理 JWT + 客户端 API Key 模型 |
| 渠道 / 分组 CRUD | 与现 UI 字段兼容 |
| Chat 转发 | 非流式 + SSE/流式 |
| 路由与负载 | 轮询等现有 mode |
| 请求日志 | 落库与 list API |
| 数据迁移 | 从 octopus SQLite schema → 新 schema（或兼容读） |
| 壳集成 | 去掉/可选化 `sidecar/octopus.exe`；更新 prepare/CI/合规 |
| 去侧车发布 | 新版本 NSIS 无 AGPL 二进制时的 NOTICE 更新 |

### P3. 迁移策略（待决）

- 特性开关：`MODEL_HUB_GATEWAY_IMPL=rust|octopus` 或安装包变体。
- 并行期：保留内嵌 octopus 作为回退。
- 验收：干净环境安装 → 管理 UI + 创建 Key + `/v1/models` 非 401 + 真实/mock Chat。

### P4. 风险

| 风险 | 备注 |
|------|------|
| 契约漂移 | 必须以现 UI + e2e smoke 为金标准 |
| 流式细节 | 不同客户端对 SSE 帧格式敏感 |
| 数据迁移 | 用户已有 gateway_dir 数据不可丢 |
| 合规叙事 | 去掉 AGPL 二进制后更新 third-party 与 Release notes |

## Acceptance Criteria（规划完成即可勾选）

- [x] AC1：本 PRD 列出契约、切片、迁移与风险大纲。
- [ ] AC2：启动实现前再拆可执行子任务并 `task.py start`（**不要**在本规划任务上直接大面积编码）。

## Decisions Log

| # | 决策 | 选择 | 日期 |
|---|------|------|------|
| D1 | 与阶段 1 关系 | 先内嵌 octopus 公开发布，再规划 Rust 替换 | 2026-07-18 |
| D2 | 本任务深度 | 仅父任务规划，不实现 | 2026-07-18 |

## Notes

- 依赖阶段 1 发布任务稳定后再排期。
- 实现启动时读取 `.trellis/spec/backend/*` 与 `gateway/README.md` 契约。
