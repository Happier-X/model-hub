# 上游供应商走代理服务器（HTTP/SOCKS5）

## Goal

让本地网关向上游供应商发请求时，可走用户配置的代理服务器（HTTP/HTTPS/SOCKS5），解决无法直连海外上游的问题。

## Background

当前 `forward.rs` 的 `UpstreamClients` 构造 `reqwest::Client` 时无任何代理配置，海外上游（OpenAI/Anthropic）在无系统代理环境下无法直连。`reqwest` 的 `Cargo.toml` 也未启用 `proxy` feature。

## Requirements

- 全局代理：所有上游供应商请求走同一个代理地址（不按供应商独立配置）。
- 支持 HTTP/HTTPS 代理（`http://host:port`）和 SOCKS5（`socks5://host:port`）。
- 代理地址为空时不走代理（直连），保持现有行为。
- 代理配置持久化到 `ShellConfig`（`shell.json`）。
- 修改代理后，若代理正在运行，自动 stop → start 重建客户端。
- SettingsPage「代理配置」卡片内新增「上游代理」输入框。
- 代理配置含：启用开关（Checkbox）+ 代理地址（Input）+ 可选认证（用户名/密码）。

## Acceptance Criteria

- [ ] `ShellConfig` 新增 `upstream_proxy_enabled`、`upstream_proxy_url`、`upstream_proxy_user`、`upstream_proxy_pass` 字段，有 serde default。
- [ ] `reqwest` Cargo.toml 添加 `proxy` + `socks` feature（SOCKS5 支持）。
- [ ] `UpstreamClients::new()` 接受代理配置参数，按协议注入 `reqwest::Proxy`。
- [ ] `RuntimeInner` 持有代理配置，`start()` 时按当前配置重建 `UpstreamClients`。
- [ ] 新增 Tauri command `set_upstream_proxy`，保存配置 + 重建 clients + 重启代理（如正在运行）。
- [ ] 前端 `tauri.ts` 新增 `setUpstreamProxy` 方法。
- [ ] `SettingsPage.vue` 代理配置卡片新增上游代理表单（开关 + 地址 + 认证）。
- [ ] `ShellPrefs` 新增代理相关字段暴露给前端。
- [ ] 测试：`UpstreamClients` 可注入代理配置，验证直连/代理两种路径。
- [ ] 测试：验证无代理时保持现有直连行为（regression）。
- [ ] `vue-tsc` 和 `eslint` 通过。

## Out of Scope

- 按供应商独立代理配置。
- 代理自动检测（如读取系统环境变量 `HTTP_PROXY`）。
- 代理连接测试（探测代理是否可用）。
- 代理配置导出/导入。

## Key Decisions

- **全局代理（A）**：所有上游走同一代理，配置一次即可。
- **持久化位置**：复用 `ShellConfig`（`shell.json`），不新建配置文件。
- **重启策略**：修改代理配置后 stop → start 重建 clients，与 `set_port` 模式一致。
- **SOCKS5**：启用 `reqwest` 的 `socks` feature，与 HTTP proxy 共用一个 URL 字段，按协议前缀自动识别。

## Technical Notes

- reqwest `Proxy::http(url)` / `Proxy::https(url)` / `Proxy::all(url)` 按协议自动匹配。
- SOCKS5 需要 `socks` feature（`reqwest` 的可选依赖 `tokio-socks`）。
- 代理认证用 `Proxy::basic_auth(user, pass)`。
- `UpstreamClients` 从固定构造改为按配置构造，需传递 `Option<ProxyConfig>`。
