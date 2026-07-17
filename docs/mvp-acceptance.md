# M1 验收清单

对照父任务 `07-17-tauri-port-octopus` 的 MVP 验收项。

| ID | 标准 | 状态 | 说明 |
|----|------|------|------|
| AC1 | Windows 启动；管理 UI **无需登录** | 代码完成 | 无登录页；静默 admin + Token 兜底 |
| AC2 | OpenAI 渠道 + 分组 Chat 转发 | 代码完成 / 环境待验 | 需本机放置 `octopus.exe` 与真实上游 Key |
| AC3 | 至少一种负载策略 | 代码完成 | 分组默认 **轮询 (mode=1)** |
| AC4 | 基础请求日志；正常退出停侧车 | 代码完成 | 日志轮询 list；Exit 时 stop 托管进程 |
| AC5 | SQLite；目录可发现 | 代码完成 | `get_paths` + 设置页路径；DB 在 gateway_dir |
| AC6 | 启停/健康检查；端口占用可提示 | 代码完成 | 缺 exe / 端口占用有错误文案 |

## 手工验收步骤

1. `pnpm install`
2. 放置 `octopus.exe` → `bin_dir` 或 `MODEL_HUB_GATEWAY_BIN`
3. `pnpm tauri dev`
4. 设置 → 启动网关 → 状态「运行中」
5. 渠道 → 创建 OpenAI Chat 渠道
6. 分组 → 创建分组并绑定渠道
7. 使用 [client-integration.md](./client-integration.md) 发起 Chat
8. 日志页查看是否出现记录
9. 退出应用，确认侧车进程结束（任务管理器）

## 自动化验证（无真侧车）

```bash
pnpm lint
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```

## 已知边界

- 真机钉扎 **octopus v0.9.28**（见 `gateway/README.md`、`scripts/fetch-octopus-windows.ps1`、`scripts/e2e-octopus-smoke.py`）。
- 该版本渠道 `type` 为 **数字**（OpenAI Chat = `0`）；字符串 type 会 Invalid JSON。
- `/v1/*` 客户端可能仍需侧车签发的 `sk-octopus-...` Key；管理 API 用 JWT。
- 上游若改密，需设置页 Token。
- 开发清理**只结束测试端口/PID**，勿按进程名杀掉本机全部 octopus。
