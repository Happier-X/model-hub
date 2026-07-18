# 设计：网关监听端口设置

## 方案

新增壳配置文件 `{config_dir}/shell.json`，由 Rust 负责读取和原子写入。前端通过 Tauri invoke 保存端口，不使用 localStorage 作为真源。

```json
{
  "gateway_port": 8080
}
```

## 启动数据流

```text
Tauri setup
  → resolve_paths
  → load_shell_config(config_dir/shell.json)
  → 缺失：DEFAULT_PORT=8080
  → 合法：使用 gateway_port
  → 损坏/越界：回退 8080，并保留可恢复能力
  → GatewayHandle::new(host, port, gateway_dir)
```

保存数据流：

```text
设置页输入 → 前端整数/范围校验
  → gateway_set_port(port)
  → runtime.set_port(port)
     - Running/Starting/Stopping：拒绝，提示先停止
     - Idle/Error：更新 GatewayStatus 的 port/base_url，清理旧端口错误
  → shell.json 临时文件 + rename 原子写入
  → 返回最新 GatewayStatus
  → 下次 gateway_start 生成新 config/env/health 地址
```

## Rust 模块

### `gateway/settings.rs`

- `ShellConfig { gateway_port: u16 }`
- `load_shell_config(config_dir) -> Result<ShellConfig, AppError>`
- `save_shell_config(config_dir, &ShellConfig) -> Result<(), AppError>`
- 缺失文件返回默认配置。
- JSON 损坏或端口非法：为保证应用仍可打开，回退默认值；保存新值可覆盖恢复。若需暴露警告，可通过日志/状态错误，但本次不新增日志基础设施。

### `GatewayRuntime::set_port`

- 只允许 `Idle` / `Error`。
- 更新 `status.port` 与 `status.base_url`。
- 端口范围由 `u16` + 命令参数转换校验；前端亦提前校验。

### Tauri command

```text
gateway_set_port(app, gateway, port: u32) -> Result<GatewayStatus, InvokeError>
```

使用 `u32` 接收，显式校验 `1..=65535` 后转 `u16`，避免 JSON 数字转换错误变成模糊提示。成功顺序：先校验运行状态，再原子写配置，再更新内存；或先验证可更新、写入成功后更新内存，避免持久化失败导致状态分裂。

## 前端

在设置页 `GatewayPanel` 下新增 `GatewayPortPanel`：

- `type="number"`、`min=1`、`max=65535`、`step=1`
- 初值与 `gateway.port` 同步；用户正在编辑时避免 2 秒轮询覆盖输入
- 运行/启动/停止中禁用输入和保存按钮
- 保存成功显示“已保存，下次启动使用 …”
- 校验失败、invoke 失败使用 `role="alert"`

`src/api/tauri.ts` 新增 `gatewaySetPort(port)`。

## 兼容性

- 首次运行或无配置：继续使用 `127.0.0.1:8080`。
- 已有 `gateway/data/config.json` 会在每次启动时由壳按当前端口重写，不作为壳设置真源。
- 不改变 host、鉴权、内嵌侧车或数据目录。
- 不探测并自动选择端口，不结束任何占用进程。

## 测试

- Rust：缺失配置默认 8080；保存/加载；损坏回退；端口更新同步 base URL；运行中拒绝修改。
- 前端：lint/build 覆盖类型和 JSX；当前项目无前端测试框架，不新增重型依赖。
- 集成：选择空闲测试端口，保存后启动，检查状态/base_url 与生成的 `data/config.json`。

## 回滚

移除 `gateway_set_port`、端口面板与 shell 配置加载，恢复 `DEFAULT_PORT` 初始化；不影响侧车业务数据库。
