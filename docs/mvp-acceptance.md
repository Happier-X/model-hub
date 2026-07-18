# M1 验收清单

对照父任务 `07-17-tauri-port-octopus` 的 MVP 验收项，并含网关 API Key 鉴权闭环。

| ID | 标准 | 状态 | 说明 |
|----|------|------|------|
| AC1 | Windows 启动；管理 UI **无需登录** | 代码完成 | 无登录页；静默 admin + Token 兜底 |
| AC2 | OpenAI 渠道 + 分组 Chat 转发 | 代码完成 / 环境待验 | 需本机放置 `octopus.exe` 与真实上游 Key |
| AC3 | 至少一种负载策略 | 代码完成 | 分组默认 **轮询 (mode=1)** |
| AC4 | 基础请求日志；正常退出停侧车 | 代码完成 | 日志轮询 list；Exit 时 stop 托管进程 |
| AC5 | SQLite；目录可发现 | 代码完成 | `get_paths` + 设置页路径；DB 在 gateway_dir |
| AC6 | 启停/健康检查；端口占用可提示 | 代码完成 | 缺 exe / 端口占用有错误文案 |
| AC7 | 网关 API Key 管理与客户端鉴权 | 代码完成 | UI 创建/列表/删除；`GET /v1/models` 非 401 |
| AC8 | 仪表盘配置检查清单 | 代码完成 | 网关/鉴权/渠道/分组/Key 状态 + 客户端 curl 模板 |

## 手工验收步骤

1. `pnpm install`
2. 放置 `octopus.exe` → `bin_dir` 或 `MODEL_HUB_GATEWAY_BIN`
3. `pnpm tauri dev`
4. 仪表盘 → 查看配置检查清单（网关未启动时 3–5 应为「等待前置」）
5. 设置 → 启动网关 → 状态「运行中」
6. 渠道 → 创建 OpenAI Chat 渠道
7. 分组 → 创建分组并绑定渠道
8. **API 密钥** → 创建密钥 → 复制完整 `sk-octopus-...`
9. 返回仪表盘确认步骤 1–5 为已完成；复制 curl 模板
10. 使用 [client-integration.md](./client-integration.md)，以该 Key 调用 `GET /v1/models`（期望非 401）
11. （可选）有真实上游时发起 Chat；无上游时业务错误可接受
12. 日志页查看是否出现记录
13. 退出应用，确认侧车进程结束（任务管理器）

## 自动化验证

```bash
pnpm lint
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
# 需本机 tools/octopus/octopus.exe（v0.9.28）
python scripts/e2e-octopus-smoke.py
```

## 已知边界

- 真机钉扎 **octopus v0.9.28**（见 `gateway/README.md`、`scripts/fetch-octopus-windows.ps1`、`scripts/e2e-octopus-smoke.py`）。
- 该版本渠道 `type` 为 **数字**（OpenAI Chat = `0`）；字符串 type 会 Invalid JSON。
- `/v1/*` 客户端 **必须** 使用侧车签发的 `sk-octopus-...` Key；管理 API 用 JWT。
- 上游若改密，需设置页 Token。
- 开发清理**只结束测试端口/PID**，勿按进程名杀掉本机全部 octopus。
- 无真实供应商 Key 时不保证 Chat 200；鉴权闭环以 `/v1/models` 非 401 为准。
