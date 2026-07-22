# 当前架构

Model Hub 是本机优先的 Tauri 2 桌面应用，由 Vue 3 管理台和同一进程内运行的 Rust HTTP 代理组成。

## 组件

```text
外部 OpenAI 兼容客户端
        │ 127.0.0.1 /v1/* + 客户端 Key
        ▼
Tauri 2 进程内 Rust 代理 ── SQLite
        ▲
        │ Tauri commands（IPC）
        │
Vue 3 + Tailwind 管理台
```

- 管理台只通过 Tauri commands 操作供应商、分组、客户端 Key、日志和代理状态。
- 外部客户端只访问本机 HTTP：`GET /v1/models` 与 `POST /v1/chat/completions`。
- 代理默认绑定 `127.0.0.1:8080`，端口可在应用内修改。
- SQLite 位于应用数据目录，保存配置和脱敏请求日志。

## 路由与可靠性

客户端请求中的 `model` 是分组名。每个分组维护有序队列，每个条目绑定一个供应商和上游模型名。代理按顺序尝试队列，并在启用自动故障转移时对网络错误、超时及可重试状态换源。

运行时按供应商维护默认熔断状态：连续失败达到阈值后打开熔断，等待恢复窗口后进入半开探测。健康状态和故障转移摘要可在管理台查看。

非流式响应在完整读取后才提交给客户端。流式响应至少读取首个数据块后才提交；提交前可以换源，提交后只透传当前响应。

## 安全边界

- `/v1/*` 客户端 API Key **可选**（本机默认）；若请求携带了 Key，则必须有效且启用。
- 客户端 Key 明文只在创建时展示一次，数据库只保存哈希和脱敏值。
- 上游 Key 保存在本机 SQLite 中，不写入请求日志。
- 请求日志不保存完整消息正文。

## 代码位置

- `src/`：Vue 管理台。
- `src/api/tauri.ts`：IPC 命令及跨层类型。
- `src-tauri/src/proxy/`：HTTP 服务、转发、故障转移与熔断。
- `src-tauri/src/domain/`：SQLite 领域 CRUD。
- `src-tauri/src/commands.rs`：Tauri IPC 薄封装。
