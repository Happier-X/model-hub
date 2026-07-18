# 发布内嵌侧车的 Windows 安装版（v0.0.1）

## Goal

通过 **GitHub Actions** 公开发布 **Model Hub v0.0.1** Windows 安装包：安装后**无需用户自行下载或放置 `octopus.exe`**；内嵌钉扎 **octopus v0.9.28**；附带 AGPL 合规材料与对应源码获取信息；并规划后续 Rust 原生网关迁移（本任务不实现重写）。

## Background

- 产品功能（管理 UI、API Key、渠道/分组、日志、Chat 自检、托盘）已具备。
- 当前壳仅从 `MODEL_HUB_GATEWAY_BIN` 或 app data `bin/octopus.exe` 找侧车，用户仍需手工准备。
- 本地已有钉扎二进制约 49 MB（gitignore）；Git 不提交大二进制。
- 用户决策：
  - **A+B**：先内嵌发布，同时规划 Rust 原生迁移。
  - **公开分发**（不仅内部试用）。
  - **GitHub Actions** 发布。
  - 首版版本号 **v0.0.1**。
  - 未强制要求代码签名（未签名将可能触发 SmartScreen「未知发布者」）。

## Requirements

### R1. 用户零外部侧车依赖

- 安装/运行后不要求用户放置 `octopus.exe`。
- 解析优先级：`MODEL_HUB_GATEWAY_BIN`（开发覆盖）→ app data 已部署副本 → 安装资源内嵌侧车并自动部署。
- 设置页文案改为「内置网关 v0.9.28」，不再要求手工下载（开发覆盖仍可文档说明）。

### R2. 内嵌与构建

- 构建前下载并校验 octopus v0.9.28 Windows x64。
- Tauri 发布配置将侧车与第三方许可证作为 `bundle.resources` 打入安装包。
- 首次启动将资源侧车原子复制到 app data `bin/`（按哈希/版本判断是否覆盖）。
- 产出 **NSIS** 安装包；可选 MSI。
- 应用版本统一为 **0.0.1**（`package.json` / `Cargo.toml` / `tauri.conf.json`）。

### R3. GitHub Actions 公开释放

- Workflow：在推送 tag `v0.0.1`（及后续 `v*.*.*`）时于 `windows-latest` 构建。
- 步骤：checkout → Node/Rust 工具链 → prepare 侧车（下载+校验）→ `pnpm install` → Tauri build（NSIS + 内嵌资源）→ 上传 artifacts → 创建 GitHub Release（draft 或正式，默认正式 release）。
- Release 附件至少：
  - NSIS 安装包
  - SHA-256 校验文件
  - 合规说明 / NOTICE / AGPL 文本（或安装包内已含 + Release body 摘要）
- Release notes 中文：安装说明、未知发布者提示、JWT vs sk-octopus、上游 AGPL 与源码链接。

### R4. 合规（公开分发）

- 仓库与安装包内附：
  - octopus **AGPL-3.0** 全文
  - NOTICE：上游仓库、tag `v0.9.28`、commit、二进制 URL、**对应源码 archive URL**
  - Model Hub 自身许可证声明（若尚未明确，本任务至少标明与 AGPL 组件关系）
- 不提供法律意见；实现以「可公开附许可证 + 对应源码链接 + 版本钉扎」为工程完成标准。
- 不提交真实密钥。

### R5. 验收

- 干净环境：无 `MODEL_HUB_GATEWAY_BIN`、无预置 app data exe → 安装 v0.0.1 → 网关可运行 → 管理 UI + 创建 API Key + `/v1/models` 非 401。
- CI 能产出 NSIS 与 SHA-256。
- 本地 `pnpm lint/build`、`cargo fmt/test/check`、smoke 通过。
- 另建 **Rust 原生网关** 父任务路线图（仅规划，不实现）。

## Out of Scope

- 完整 Rust 网关重写（本任务仅规划父任务）。
- 代码签名证书采购与签名流水线。
- macOS/Linux 发布。
- 应用内自动更新。

## Acceptance Criteria

- [x] AC1：安装后无需用户自备 `octopus.exe` 即可启动内置网关。（运行时：resource → app data 部署；本地 NSIS 已产出）
- [x] AC2：版本号 **0.0.1**；`.github/workflows/release-windows.yml` 已就位（正式 Release 需推 tag 由 CI 执行）。
- [x] AC3：干净环境鉴权闭环：`e2e-octopus-smoke.py` 通过（含 `/v1/models` 200）。
- [x] AC4：AGPL/NOTICE/源码获取信息已在 `third-party/octopus/` 与 release notes / CI 附件。
- [x] AC5：`pnpm lint/build`、`cargo fmt/test/check`、真实侧车 smoke 通过。
- [x] AC6：已创建后续 `07-18-rust-native-gateway` 父任务规划（不实现）。

## Decisions Log

| # | 决策 | 选择 | 日期 |
|---|------|------|------|
| D1 | 近期可用版 | 内嵌 octopus v0.9.28 | 2026-07-18 |
| D2 | 长期架构 | 同时规划 Rust 原生迁移 | 2026-07-18 |
| D3 | Git | 不提交侧车二进制；CI/脚本下载校验 | 2026-07-18 |
| D4 | 用户依赖 | 安装后零手工侧车 | 2026-07-18 |
| D5 | 发布范围 | 公开分发 | 2026-07-18 |
| D6 | 发布方式 | **GitHub Actions** + GitHub Release | 2026-07-18 |
| D7 | 首版版本 | **v0.0.1** | 2026-07-18 |
| D8 | 签名 | 首版可不签名；文档提示 SmartScreen | 2026-07-18 |

## Notes

- 实现前需用户确认本 PRD + design/implement 后 `task.py start`。
