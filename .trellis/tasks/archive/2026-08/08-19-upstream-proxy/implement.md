# 上游供应商走代理服务器 — 实施清单

## 实施顺序

### 第 1 步：Cargo.toml 添加 proxy feature

- **文件**：`src-tauri/Cargo.toml`
- **变更**：reqwest features 加 `"proxy"`, `"socks"`
- **验证**：`cargo check` 通过

### 第 2 步：ShellConfig 加代理字段

- **文件**：`src-tauri/src/settings.rs`
- **变更**：`ShellConfig` 加 4 个 `#[serde(default)]` 字段
- **验证**：`cargo check` 通过，旧 shell.json 零迁移兼容

### 第 3 步：UpstreamClients 支持代理配置

- **文件**：`src-tauri/src/proxy/forward.rs`
- **变更**：
  - 新增 `ProxyConfig` 结构体（`url`, `username`, `password`）
  - `UpstreamClients::new()` 改为 `new(proxy: Option<&ProxyConfig>)`
  - 按代理配置注入 `reqwest::Proxy`
- **验证**：`cargo check` 通过

### 第 4 步：RuntimeInner 持有代理配置

- **文件**：`src-tauri/src/proxy/runtime.rs`
- **变更**：
  - `RuntimeInner` 加 `proxy_config: Option<ProxyConfig>` 字段
  - `new_with_config_dir` 从 `ShellConfig` 读取代理配置
  - `start()` 时用 `self.proxy_config` 构造 `UpstreamClients`
  - 新增 `set_upstream_proxy` 方法（重建 clients + 重启）
- **验证**：`cargo check` 通过

### 第 5 步：新增 Tauri command

- **文件**：`src-tauri/src/commands.rs`
- **变更**：
  - `ShellPrefs` 加 3 个代理字段（不暴露密码）
  - 新增 `set_upstream_proxy` command
  - `shell_prefs()` 函数加代理字段映射
- **文件**：`src-tauri/src/lib.rs`
- **变更**：`generate_handler!` 注册 `set_upstream_proxy`
- **验证**：`cargo check` 通过

### 第 6 步：后端集成测试

- **文件**：`src-tauri/src/proxy/forward.rs`（现有 `#[cfg(test)]`）
- **变更**：新增测试用例
  - `UpstreamClients::new(None)` 走直连（回归）
  - `UpstreamClients::new(Some(proxy_config))` 注入代理
- **验证**：`cargo test` 通过

### 第 7 步：前端 tauri.ts 类型同步

- **文件**：`src/api/tauri.ts`
- **变更**：
  - `ShellPrefs` interface 加代理字段
  - 新增 `setUpstreamProxy` 方法
- **验证**：`vue-tsc --noEmit` 通过

### 第 8 步：SettingsPage UI

- **文件**：`src/pages/SettingsPage.vue`
- **变更**：
  - 导入 `setUpstreamProxy`
  - 代理配置卡片新增：启用开关 + 地址 + 用户名 + 密码 + 保存按钮
  - `refresh()` 加载代理配置
  - `saveProxy()` 调用 `setUpstreamProxy`
- **验证**：`vue-tsc --noEmit` + `eslint` 通过

### 第 9 步：spec 更新

- **文件**：`.trellis/spec/backend/index.md`（如需）
- **变更**：记录代理配置相关约定

## 验证命令

```bash
# Rust 编译检查
cd src-tauri && cargo check

# Rust 测试
cd src-tauri && cargo test

# 前端类型检查
vue-tsc --noEmit

# 前端 lint
eslint src/pages/SettingsPage.vue src/api/tauri.ts
```

## 回滚点

- 第 1 步：revert Cargo.toml
- 第 2-5 步：revert settings.rs / forward.rs / runtime.rs / commands.rs / lib.rs
- 第 7-8 步：revert tauri.ts / SettingsPage.vue
- 每步独立，可单独回滚

## 关键文件

| 文件 | 变更类型 |
|------|----------|
| `src-tauri/Cargo.toml` | 添加 feature |
| `src-tauri/src/settings.rs` | 新增字段 |
| `src-tauri/src/proxy/forward.rs` | 重构构造函数 |
| `src-tauri/src/proxy/runtime.rs` | 注入配置 + 重建逻辑 |
| `src-tauri/src/commands.rs` | 新增 command + prefs |
| `src-tauri/src/lib.rs` | 注册 command |
| `src/api/tauri.ts` | 类型 + 方法 |
| `src/pages/SettingsPage.vue` | UI 表单 |
