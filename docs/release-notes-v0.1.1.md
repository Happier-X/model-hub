# Model Hub v0.1.1 发布说明

在 **v0.1.0**（Vue 3 管理台 + Tauri 进程内 Rust 代理）基线上的增量发布：运维体验、分组队列排序与供应商录入效率。

## 相对 v0.1.0 的变更

### 分组与模型能力

- 分组故障转移队列支持按**模型名本地启发式**排序（系列/标签/分数展示，未知模型稳定偏后，可拖拽微调）
- 支持按 **OpenRouter 公共榜单**排序：外部通用能力 / 外部编码能力；未匹配或无分回退本地启发式
- 榜单：固定公开 API、无 Key、白名单字段解析；缓存 `config_dir/model-leaderboard-openrouter.json`（TTL 24h）；网络失败有缓存时 stale 回退
- 排序只改当前表单，需用户保存；UI 展示来源、分数、更新时间与缓存状态

### 供应商与运维

- 供应商粘贴可快速识别 Base URL 与 API Key
- 代理首选端口被占用时，自动扫描可用端口并写入配置（不结束占用进程）
- 请求日志默认保留 30 天并支持清理过期
- 日志页默认每 3 秒自动刷新

### 兼容与修复（若自更早构建升级）

- 旧库 `request_logs` 等缺列兼容与落库修复（延续 0.1.0 幂等 `ALTER` 策略）

## 升级注意

1. 完全退出旧版（含托盘）后安装本版本，或使用应用内「检查更新」。
2. 配置与 SQLite 数据目录沿用，一般**无需**重配供应商/分组。
3. 首次使用「外部能力排序」时需能访问 OpenRouter 公共 Models API；离线时若已有榜单缓存仍可 stale 使用，否则请改用本地启发式排序。
4. 客户端用法不变：Base URL 为本机代理地址，`model` 填**分组名**；本机 API Key 仍可选。

## 安装与更新

- 安装包：GitHub Release 中的 NSIS `.exe`
- 校验：同 Release 的 `SHA256SUMS.txt`
- 应用内更新清单：`https://github.com/Happier-X/model-hub/releases/latest/download/latest.json`
- 概览页「检查更新」；用户确认后下载安装并重启。详情见 [应用更新与发布说明](./in-app-updater.md)。

## 架构与验收

- 架构：[当前架构](./current-architecture.md)
- 自动化 / 手工 MVP：[mvp-acceptance.md](./mvp-acceptance.md)
- 本机联调：[local-acceptance.md](./local-acceptance.md)
- 客户端对接：[client-integration.md](./client-integration.md)
- 基线说明：[v0.1.0](./release-notes-v0.1.0.md)

## 开发者验证

```powershell
pnpm install
pnpm lint
pnpm typecheck
pnpm test:unit
pnpm build
cd src-tauri
cargo test
cargo check
```

本地打 Windows 安装包：

```powershell
pnpm release:windows
```

CI 推送 `v*` tag 时执行 `.github/workflows/release-windows.yml`，需 Secrets：

```text
TAURI_SIGNING_PRIVATE_KEY
TAURI_SIGNING_PRIVATE_KEY_PASSWORD
```

## 已知限制

- OpenRouter 榜单依赖外网与公共接口形态；解析仅白名单字段，不做网页抓取。
- 外部排序为高置信精确匹配（归一化后全等），名称差异大的模型会回退本地启发式。
- 其余限制同 v0.1.0（可选启动检查更新、默认仅 `127.0.0.1`、无 0.0.x 侧车库完整导入等）。
