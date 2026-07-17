# 设计：MVP 网关侧车集成

## 架构

```
UI ──invoke──► gateway module (Rust)
                  │ spawn / kill / health
                  ▼
              octopus.exe (Windows)
                  │ cwd/env → gateway_dir
                  ▼
              127.0.0.1:PORT + SQLite under gateway_dir
```

## 模块

新增 `src-tauri/src/gateway/`：

| 文件 | 职责 |
|------|------|
| `mod.rs` | 导出命令与状态 |
| `state.rs` | 进程状态机 + Mutex/Arc |
| `process.rs` | spawn、stop、graceful timeout |
| `health.rs` | HTTP/TCP 探活 |
| `config.rs` | 写 config.json / 组装环境变量 |
| `binary.rs` | 解析 `bin_dir` 下 exe 名 |

注册命令：`gateway_start`、`gateway_stop`、`gateway_status`。

应用 `setup`：初始化 paths；可选自动 start。  
`RunEvent::ExitRequested` / window close：调用 stop。

## 配置契约

`gateway_dir/config.json`（示例）：

```json
{
  "server": { "host": "127.0.0.1", "port": 8080 },
  "database": { "type": "sqlite", "path": "data.db" },
  "log": { "level": "info" }
}
```

环境变量（覆盖，与上游一致）：

- `OCTOPUS_SERVER_HOST=127.0.0.1`
- `OCTOPUS_SERVER_PORT=<port>`
- `OCTOPUS_DATABASE_TYPE=sqlite`
- `OCTOPUS_DATABASE_PATH=<absolute or relative under gateway_dir>`
- `OCTOPUS_LOG_LEVEL=info`

工作目录：优先 `gateway_dir`，使相对路径落在数据目录。

## 二进制

- 默认文件名：`octopus.exe`（Windows）
- 搜索顺序：`bin_dir/octopus.exe` → 可选开发旁路环境变量 `MODEL_HUB_GATEWAY_BIN`
- **不**把巨大二进制强制提交进 git；`gateway/README.md` 写下载步骤
- 版本建议钉扎上游 release（如 v0.9.x，实现时写明实际选用版本）

## 健康检查

- 轮询 `http://127.0.0.1:{port}/` 或任意返回非连接拒绝的响应（含 401/302 也算进程就绪）
- start：spawn 后在超时窗口内轮询，成功 → running，失败 → stop 残留进程 + error

## 前端

- `src/api/tauri.ts` 增加类型与 `gatewayStart/Stop/Status`
- StatusBar 轮询 status（2s）
- 设置页：路径 + 端口展示 + 启停按钮

## 鉴权

- 本任务不实现绕过 admin 的完整方案（留给 admin-ui 若必须）
- 文档：客户端可用占位 key；监听仅本机

## 测试

- 状态机纯逻辑单测
- config 序列化单测
- 无真实 exe 时 start 返回明确错误（可测）
