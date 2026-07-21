# M1 验收清单

对照父任务 `07-17-tauri-port-octopus` 的 MVP 验收项，并含网关 API Key 鉴权闭环。

| ID | 标准 | 状态 | 说明 |
|----|------|------|------|
| AC1 | Windows 启动；管理 UI **无需登录** | 代码完成 | 无登录页；静默 admin + Token 兜底 |
| AC2 | OpenAI 渠道 + 分组 Chat 转发 | 代码完成 / 环境待验 | 安装版内嵌 model-hub-gateway；开发需 `pnpm prepare:gateway-rust` 或 `MODEL_HUB_GATEWAY_BIN`；Chat 需真实上游 Key |
| AC3 | 至少一种负载策略 | 代码完成 | 分组默认 **轮询 (mode=1)** |
| AC4 | 基础请求日志；正常退出停侧车 | 代码完成 | 日志轮询 list；Exit 时 stop 托管进程 |
| AC5 | SQLite；目录可发现 | 代码完成 | `get_paths` + 设置页路径；DB 在 gateway_dir |
| AC6 | 启停/健康检查；端口占用可提示 | 代码完成 | 缺 exe / 端口占用有错误文案 |
| AC7 | 网关 API Key 管理与客户端鉴权 | 代码完成 | UI 创建/列表/删除；`GET /v1/models` 非 401 |
| AC8 | 仪表盘配置检查清单 | 代码完成 | 网关/鉴权/渠道/分组/Key 状态 + 客户端 curl 模板 |
| AC9 | 渠道编辑与 Key 显示 | 代码完成 | 改 name/URL/model/轮换 Key；列表脱敏可显示；删除确认 |
| AC10 | 分组绑定可读与编辑 | 代码完成 | 列表展示渠道+model_name；改名/换绑；删除确认 |
| AC11 | Chat 上手文档与客户端自检 | 代码完成 | `docs/chat-onboarding.md`；仪表盘用 sk-octopus 测 /v1 |
| AC12 | 日志分页/过滤/详情 | 代码完成 | 分页、当前页过滤、详情展开、清空确认、自动刷新可关 |
| AC13 | 托盘与关窗隐藏 | 代码完成 | 关窗隐藏到托盘；托盘退出才停托管网关 |

## 手工验收步骤

1. `pnpm install`
2. 安装版可跳过；开发执行 `pnpm prepare:gateway-rust` 或设置 `MODEL_HUB_GATEWAY_BIN`
3. `pnpm tauri dev`
4. 仪表盘 → 查看配置检查清单（网关未启动时 3–5 应为「等待前置」）
5. 设置 → 启动网关 → 状态「运行中」
6. 渠道 → 创建 OpenAI Chat 渠道
7. 分组 → 创建分组并绑定渠道
8. **API 密钥** → 创建密钥 → 复制完整 `sk-octopus-...`
9. 返回仪表盘确认步骤 1–5 为已完成；复制 curl 模板
10. 使用 [client-integration.md](./client-integration.md) 或 [chat-onboarding.md](./chat-onboarding.md)，以该 Key 调用 `GET /v1/models`（期望非 401）
11. 仪表盘「客户端路径自检」粘贴 Key；可选填分组名测 chat
12. （可选）有真实上游时发起 Chat；无上游时业务错误可接受
13. 日志页查看是否出现记录
14. 关闭主窗口：确认应用隐藏到托盘且侧车仍运行；托盘「显示」恢复窗口；托盘「退出」后确认侧车进程结束（任务管理器）

## 自动化验证

```bash
pnpm lint
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path gateway-rust/Cargo.toml
# 可选：兼容侧车历史冒烟（需自备 tools/octopus/octopus.exe）
# python scripts/e2e-octopus-smoke.py
```

## 已知边界

- **默认网关为 Rust**（`model-hub-gateway`）；安装包内嵌该二进制（见 `gateway/README.md`、`scripts/prepare-bundled-gateway-rust.ps1`）。
- 公开安装包**不再**内嵌 octopus；Git 不提交 `tools/gateway-rust/` / `tools/octopus/` 大二进制。
- 渠道 `type` 为 **数字**（OpenAI Chat = `0`）。
- `/v1/*` 客户端 **必须** 使用网关签发的 `sk-octopus-...` Key（历史前缀）；管理 API 用 JWT。
- 从旧 octopus 库切换请用 `migrate-octopus` 或新建数据目录；勿混用同一 `data.db`。
- 开发清理**只结束测试端口/PID**，勿按进程名乱杀。
- 无真实供应商 Key 时不保证 Chat 200；鉴权闭环以 `/v1/models` 非 401 为准。
