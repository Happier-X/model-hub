# Tauri 移植 octopus LLM 网关

## Goal

用 **Tauri 桌面应用**（Windows）提供与 [bestruirui/octopus](https://github.com/bestruirui/octopus) 能力对等的 **个人向 LLM API 聚合网关 + 管理面板**：本机一键运行，对外提供统一兼容接口，供 OpenAI SDK / Claude Code / Codex 等客户端对接。

## Background

### 源项目

- 定位：个人向 LLM API 聚合与负载均衡
- 技术：Go 后端 + Next.js/React Web（构建产物嵌入 Go 二进制）
- 许可证：**AGPL-3.0**（完整移植/分发合规由用户自行确认）
- 源前端参考：`web/package.json` 为 Next 16 + React 19 + Tailwind 4 + Zustand + TanStack Query

### 源功能大表（长期对等目标）

| 模块 | 能力 |
|------|------|
| 渠道 | 多渠道、单渠道多 Key |
| 端点 | 多端点、按延迟智能选择 |
| 负载 | 轮询 / 随机 / 故障转移 / 加权 |
| 协议 | OpenAI Chat / Responses / Images、Anthropic、Gemini 等互转 |
| 分组 | 对外统一 model 名映射多渠道 |
| 价格 | models.dev 同步 + 本地覆盖价 |
| 模型 | 渠道可用模型同步 |
| 统计 | 请求、Token、费用；内存聚合后批量落库 |
| 管理台 | Dashboard、渠道、分组、价格、日志、设置 |
| 存储 | SQLite / MySQL / PostgreSQL |
| 配置 | `config.json` + 环境变量覆盖 |

### 本仓库

- 空仓起步（`.trellis/`、`.pi/`、`AGENTS.md`）；目标栈 spec 已按本 PRD 写入 `.trellis/spec/`。
- 本任务为**父任务**：需求与验收总册；实现在子任务中完成。

## Requirements

### R0. 实现路径

- **混合渐进（C）**：阶段 1 为 Tauri 壳 + 可替换网关侧车（优先 octopus 兼容进程）；阶段 2+ 按模块 Rust 化网关，降低对 Go 侧车依赖。
- 壳与侧车契约：启停、健康检查、数据目录注入；管理 UI 业务数据以 **HTTP** 访问侧车，避免 CRUD 锁死在 invoke。

### R1. 产品形态

- Windows Tauri 可运行应用；内嵌管理 SPA；本机 HTTP 网关。
- 前端：**React + Vite + TypeScript + Tailwind**（非 Next SSR）。
- 默认监听 **`127.0.0.1`**；端口可配置（建议默认 8080，见 design）。

### R2. 功能范围

- **长期**：源功能大表对等（或子任务中明确废弃项）。
- **MVP（M1）包含**：侧车启停与健康检查；管理台无登录；SQLite；渠道（上游 Base URL + Key）；分组统一 model 名 + **至少一种**负载策略；**OpenAI Chat** 转发（流式若侧车支持则透传）；基础请求日志；正常退出时优雅停侧车。
- **MVP 不做**：Responses/Images/Anthropic/Gemini 全矩阵；MySQL/PG；models.dev 自动价；全负载模式打磨；管理登录；强制网关 API Key。

### R3. 数据与配置

- MVP 仅 SQLite；路径在应用数据目录（design 定具体子路径）。
- 配置可文件化；关键项可由环境变量/壳注入覆盖。

### R4. 桌面体验

- 启动/退出正确管理侧车生命周期；主窗口 + 网关状态可见；端口/数据目录可发现。
- 托盘、自动更新等可后置。

### R5. 合规

- 文档致谢源项目并标明许可证；用户承担 AGPL 合规责任。
- 仓库不提交真实 API Key。

### R6. 认证与访问

- **无管理台登录**；打开 UI 即可配置。
- **LLM API 本机免鉴权**（SDK 若必填 `api_key` 可用占位值）；上游 Key 仅存在渠道配置。
- 若改为非本机监听，须有明确风险提示。

## Acceptance Criteria

### 长期

- [ ] AC-LONG：源功能大表在子任务中逐项对等或明确废弃，并完成集成验收。

### MVP（M1）

- [x] AC1：Windows 上 Tauri 可启动；管理 UI **无需登录**即可配置（静默鉴权）。
- [x] AC2：渠道/分组/客户端文档已就绪；**真机 Chat 转发**需本机 `octopus.exe` + 上游 Key（见 `docs/mvp-acceptance.md`）。
- [x] AC3：分组默认轮询（Round Robin）。
- [x] AC4：日志列表页；应用 Exit 停止托管侧车。
- [x] AC5：SQLite 落 gateway_dir；`get_paths` + 设置页可发现。
- [x] AC6：启停/健康检查与缺二进制、端口占用错误文案。

## Out of Scope

- 云端多租户 SaaS、团队 RBAC、计费系统。
- 管理台账号体系；MVP 强制网关 Key。
- 非 Tauri 移动端/浏览器扩展。
- macOS/Linux 正式验收（MVP）。
- 法律意见书。

## Decisions Log

| # | 决策 | 选择 | 日期 |
|---|------|------|------|
| D1 | 实现路径 | C 混合渐进 | 2026-07-17 |
| D2 | MVP | M1 最小可用 | 2026-07-17 |
| D3 | 管理台认证 | 无登录 | 2026-07-17 |
| D4 | 平台 | Windows only | 2026-07-17 |
| D5 | 前端 | React + Vite + TS + Tailwind | 2026-07-17 |
| D6 | 对外 API 鉴权 | 本机免鉴权 + 127.0.0.1 | 2026-07-17 |
| D7 | 与 bootstrap | 先目标栈 spec，再脚手架 | 2026-07-17 |

## Child Task Map

| 顺序 | 建议 slug | 交付 |
|------|-----------|------|
| 1 | `mvp-scaffold` | Tauri + Vite 脚手架、数据目录、可开窗 |
| 2 | `mvp-gateway-sidecar` | 侧车钉扎、启停、health、优雅退出、D3/D6 适配 |
| 3 | `mvp-admin-ui` | 渠道/分组/日志/设置（无登录） |
| 4 | `mvp-e2e-docs` | Chat E2E、客户端文档、AC1–AC6、致谢 |

详见 `design.md`、`implement.md`。

## References

- https://github.com/bestruirui/octopus
- `.trellis/spec/backend/`、`.trellis/spec/frontend/`
- 本任务 `design.md`、`implement.md`
