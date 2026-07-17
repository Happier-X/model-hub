# Directory Structure

> 后端与桌面壳代码如何组织。

---

## Overview

单体仓库，当前桌面壳布局：

```
/
├── src-tauri/                 # Tauri / Rust 壳
│   ├── src/
│   │   ├── main.rs            # 桌面入口
│   │   ├── lib.rs             # Builder、命令注册
│   │   ├── paths.rs           # 应用数据目录契约与 get_paths
│   │   └── error.rs           # 可序列化 invoke 错误
│   ├── capabilities/
│   │   └── default.json
│   ├── icons/
│   │   └── icon.ico
│   ├── Cargo.toml
│   ├── Cargo.lock
│   ├── build.rs
│   └── tauri.conf.json
├── src/                       # 前端 SPA（见 frontend/directory-structure.md）
└── .trellis/                  # Trellis 任务与 spec
```

后续侧车任务在 `src-tauri/src/gateway/` 增加进程管理模块；需要打包二进制时再建立 `src-tauri/binaries/`。

---

## Module Organization

| 区域 | 放什么 | 不放什么 |
|------|--------|----------|
| `src-tauri/src/gateway/` | 子进程命令行、工作目录、环境变量、优雅退出、端口探测 | 业务路由、协议转换实现 |
| `src-tauri/src/paths.rs` | 解析 Windows 应用数据目录、配置/DB 路径 | 硬编码用户家目录零散字符串 |
| 侧车进程内部 | 渠道/分组/转发/SQLite | Tauri 窗口逻辑 |
| 管理 UI | 只通过 **HTTP** 调侧车管理 API + 少量 Tauri invoke（启停状态） | 把渠道 CRUD 做成只能走 invoke 的死绑 |

---

## Rules

1. **壳与网关边界**：业务状态以侧车 + SQLite 为准；壳不复制一份业务库。
2. **可替换侧车**：启动参数、数据目录、健康检查 URL 写成稳定契约，便于以后换 Rust 网关。
3. **Windows 路径**：使用官方 app data 约定（如 `%APPDATA%/<app>/` 或 Tauri `path` 插件），禁止写死 `C:\Users\某用户\...`。
4. **密钥**：上游 API Key 只存侧车数据/配置；日志禁止打印完整 Key。

---

## `get_paths` 跨层契约

### 1. Scope / Trigger

当新增或修改应用数据目录、Tauri invoke 返回字段、前端路径展示时，必须同步维护本契约。

### 2. Signatures

- Rust：`get_paths(app: AppHandle) -> Result<AppPaths, InvokeError>`（`src-tauri/src/paths.rs`）
- TypeScript：`getPaths(): Promise<AppPaths>`（`src/api/tauri.ts`）

### 3. Contracts

返回 JSON 字段统一使用 snake_case：

```json
{
  "app_data_dir": "<Tauri app_data_dir>",
  "config_dir": "<app_data_dir>/config",
  "gateway_dir": "<app_data_dir>/gateway",
  "bin_dir": "<app_data_dir>/bin"
}
```

命令调用前或调用中必须确保四个目录存在。前端不得自行拼接操作系统目录。

### 4. Validation & Error Matrix

| 条件 | 结果 |
|------|------|
| app data 可解析且可创建 | 返回四个绝对路径 |
| 目录创建失败 | 返回可序列化 invoke 错误，含失败路径，不含敏感信息 |
| 非 Tauri 浏览器预览 | 仅返回明确的「预览模式」占位值，不伪造真实系统路径 |

### 5. Good / Base / Bad Cases

- Good：设置页通过 `getPaths()` 展示真实路径。
- Base：浏览器 Vite 预览展示不可写的占位路径。
- Bad：前端写死 `%APPDATA%/model-hub` 或自行拼 `C:\\Users`。

### 6. Tests Required

- Rust 单元测试断言根路径派生 `config/gateway/bin`。
- `cargo test` 验证契约；前端构建确保字段类型一致。

### 7. Wrong vs Correct

```ts
// Wrong: 重复路径规则
const gateway = `${process.env.APPDATA}/model-hub/gateway`;

// Correct: 单一契约来源
const { gateway_dir: gateway } = await getPaths();
```

---

## Anti-Patterns

- 在前端硬编码绝对路径到侧车 exe。
- 管理 API 与转发 API 混在同一无区分路由且无法文档化。
- 为「完整移植」在壳里重写一套与侧车重复的配置存储。
