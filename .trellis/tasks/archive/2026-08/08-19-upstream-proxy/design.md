# 上游供应商走代理服务器 — 技术设计

## 架构概览

```
SettingsPage.vue  →  tauri invoke  →  commands::set_upstream_proxy
                                          ↓
                                    ShellConfig（shell.json）持久化
                                          ↓
                                    RuntimeInner 重建 UpstreamClients
                                          ↓
                                    stop → start（若正在运行）
                                          ↓
                                    forward.rs 新客户端走代理
```

## 数据流

1. **配置持久化**：`ShellConfig` 新增 4 个字段，serde default 保证向后兼容。
2. **客户端重建**：`set_upstream_proxy` command 读取新配置 → 重建 `UpstreamClients` → 若正在运行则 stop → start。
3. **转发层**：`forward.rs` 的 `attempt_non_stream` / `attempt_stream_prime` 使用传入的 `clients`，无需改动转发逻辑本身。

## 关键变更

### 1. `src-tauri/Cargo.toml`

```toml
reqwest = { version = "=0.13.4", default-features = false, features = ["json", "rustls", "stream", "proxy", "socks"] }
```

`socks` feature 启用 `tokio-socks`，支持 `socks5://` 协议。

### 2. `src-tauri/src/settings.rs` — ShellConfig

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellConfig {
    // ...existing fields...
    /// 上游代理是否启用。
    #[serde(default)]
    pub upstream_proxy_enabled: bool,
    /// 上游代理地址，如 "http://127.0.0.1:7890" 或 "socks5://127.0.0.1:1080"。
    #[serde(default)]
    pub upstream_proxy_url: String,
    /// 代理认证用户名（可选）。
    #[serde(default)]
    pub upstream_proxy_user: String,
    /// 代理认证密码（可选）。
    #[serde(default)]
    pub upstream_proxy_pass: String,
}
```

serde `#[serde(default)]` 保证旧 `shell.json` 无这些字段时自动用空字符串/false，零迁移。

### 3. `src-tauri/src/proxy/forward.rs` — UpstreamClients

```rust
#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub url: String,        // "http://127.0.0.1:7890" or "socks5://127.0.0.1:1080"
    pub username: String,
    pub password: String,
}

impl UpstreamClients {
    pub fn new(proxy: Option<&ProxyConfig>) -> Self {
        let mut non_stream_builder = Client::builder()
            .timeout(NON_STREAM_TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT);
        let mut stream_builder = Client::builder()
            .connect_timeout(CONNECT_TIMEOUT);

        if let Some(cfg) = proxy {
            if !cfg.url.is_empty() {
                if let Ok(p) = reqwest::Proxy::all(&cfg.url) {
                    let p = if !cfg.username.is_empty() {
                        p.basic_auth(&cfg.username, &cfg.password)
                    } else {
                        p
                    };
                    non_stream_builder = non_stream_builder.proxy(p.clone());
                    stream_builder = stream_builder.proxy(p);
                }
            }
        }

        let non_stream = non_stream_builder.build().expect("http client");
        let stream = stream_builder.build().expect("stream http client");
        Self { non_stream, stream }
    }
}
```

- `Proxy::all(url)` 自动匹配 HTTP/HTTPS/SOCKS5（按 scheme 识别）。
- 两个 client 共享同一代理配置（克隆 `Proxy` 实例）。

### 4. `src-tauri/src/proxy/runtime.rs` — RuntimeInner

```rust
struct RuntimeInner {
    // ...existing fields...
    /// 上游代理配置快照（与 ShellConfig 同步）。
    proxy_config: Option<ProxyConfig>,
}
```

`new_with_config_dir` 时从 `ShellConfig` 读取代理配置。`start()` 时用 `self.proxy_config` 构造 `UpstreamClients`。

### 5. `src-tauri/src/commands.rs` — 新增 command

```rust
#[derive(Debug, Serialize)]
pub struct ShellPrefs {
    // ...existing fields...
    pub upstream_proxy_enabled: bool,
    pub upstream_proxy_url: String,
    pub upstream_proxy_user: String,
    // 注意：不暴露密码到前端
}

#[tauri::command]
pub fn set_upstream_proxy(
    app: AppHandle,
    proxy: State<'_, ProxyHandle>,
    enabled: bool,
    url: String,
    username: String,
    password: String,
) -> Result<ShellPrefs, InvokeError> {
    let paths = paths::resolve_paths(&app).map_err(InvokeError::from)?;
    let config_dir = Path::new(&paths.config_dir);
    let mut cfg = settings::load_shell_config(config_dir).map_err(InvokeError::from)?;
    cfg.upstream_proxy_enabled = enabled;
    cfg.upstream_proxy_url = url;
    cfg.upstream_proxy_user = username;
    cfg.upstream_proxy_pass = password;
    settings::save_shell_config(config_dir, &cfg).map_err(InvokeError::from)?;
    // 重建客户端并重启代理
    proxy.set_upstream_proxy(config_dir, &cfg).map_err(Into::into)?;
    Ok(shell_prefs(&cfg))
}
```

### 6. `src-tauri/src/proxy/runtime.rs` — ProxyHandle::set_upstream_proxy

```rust
pub fn set_upstream_proxy(
    &self,
    config_dir: &Path,
    cfg: &settings::ShellConfig,
) -> Result<ProxyStatus, AppError> {
    let proxy_config = if cfg.upstream_proxy_enabled && !cfg.upstream_proxy_url.is_empty() {
        Some(ProxyConfig {
            url: cfg.upstream_proxy_url.clone(),
            username: cfg.upstream_proxy_user.clone(),
            password: cfg.upstream_proxy_pass.clone(),
        })
    } else {
        None
    };
    let was_running = self.with_inner(|inner| {
        inner.proxy_config = proxy_config;
        inner.config_dir = Some(config_dir.to_path_buf());
        Ok(matches!(inner.state, ProxyState::Running))
    })?;
    if was_running {
        let _ = self.stop();
        return self.start();
    }
    self.status_snapshot()
}
```

### 7. 前端 `src/api/tauri.ts`

```typescript
export const setUpstreamProxy = (payload: {
  enabled: boolean;
  url: string;
  username: string;
  password: string;
}) => invoke<ShellPrefs>("set_upstream_proxy", payload);
```

### 8. `src/pages/SettingsPage.vue`

代理配置卡片新增：
- 启用代理开关（Checkbox）
- 代理地址输入框（Input，placeholder `http://127.0.0.1:7890` 或 `socks5://127.0.0.1:1080`）
- 用户名输入框（可选）
- 密码输入框（type=password，可选）
- 保存按钮

保存时调用 `setUpstreamProxy`，成功后刷新 `ShellPrefs`。

## 兼容性

- `ShellConfig` 所有新字段 `#[serde(default)]`，旧 `shell.json` 零迁移自动兼容。
- 无代理时（`upstream_proxy_enabled=false` 或 URL 为空），`UpstreamClients::new(None)` 走直连，行为与改动前完全一致。

## 风险与缓解

| 风险 | 缓解 |
|------|------|
| SOCKS5 feature 增加编译时间 | `socks` 仅引入 `tokio-socks`，增量可控 |
| 代理密码明文存储在 shell.json | 当前 MVP；后续可考虑加密或 keychain |
| 修改代理后重启中断正在进行的请求 | 与 `set_port` 行为一致，用户预期 |
