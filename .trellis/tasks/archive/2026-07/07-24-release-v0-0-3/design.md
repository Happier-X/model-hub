# 技术设计

## 版本同步

同步修改为 `0.0.3`：

- `package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock` 中 `model-hub` 包版本
- `src-tauri/tauri.conf.json`
- `src-tauri/tauri.release.conf.json`

不改依赖版本字段（仅 crate/应用自身 `version`）。

## 更新日志

新增 `changelog/v0.0.3.md`，覆盖 `v0.0.2` 之后用户可见点：

### 修复

- 取消熔断与 `auto_failover`：上游错误即按配置顺序故障转移。
- 避免 Vue 深层代理 Tauri `Update` 导致私有成员运行时错误（应用内检查更新）。

### 变更 / 体验

- 渐进接入 `happier-ui`，替换可映射控件。
- 业务对话框表单改用 TanStack Form。

不把 chore/task/spec/journal 类内部提交写进用户 changelog。

## 发布流程

1. 质量检查。
2. 提交版本与 changelog（必要时同步 README 中「当前版本」表述）。
3. 推送 `master`。
4. 打 tag `v0.0.3` 并推送。
5. 等待 `release-windows`，确认 Release 资产与 `latest.json`。

## 风险与回滚

- 已推送 tag 的资产不得原地覆盖；若构建失败或内容错误，修完后发更高版本。
- 签名 Secrets 缺失会导致工作流失败；不在本地提交私钥。
