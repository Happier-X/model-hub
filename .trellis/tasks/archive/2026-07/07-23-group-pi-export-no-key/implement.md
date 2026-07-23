# 实施计划

1. **Rust `pi_export`**：实现按分组名 upsert；固定 `DEFAULT_PLACEHOLDER_KEY`；单测：新建、二次 upsert 同 id、保留其它 models/providers、空名拒绝、改 baseUrl。
2. **Command / lib**：`export_group_to_pi_agent(group_id)`；移除旧 `export_to_pi_agent` 注册与实现。
3. **前端 API + GroupsPage**：`exportGroupToPiAgent`；行内「配置到 Pi」；加载中禁用、成功提示含路径与 `model-hub/<分组名>`。
4. **ApiKeysPage**：删除 Pi 区块与 `exportToPiAgent` 引用。
5. **文档**：`docs/client-integration.md`、相关 release/README 若写死旧入口则改；Overview 改口提示改指向分组页。
6. **门禁**：`cargo test`（pi_export）、`pnpm typecheck/lint`；必要时 `test:unit`。
7. **spec（收尾）**：frontend 组件/API 约定补充「Pi 导出在分组页、无 Key」。

## 验证命令

```powershell
cd src-tauri; cargo test pi_export
pnpm typecheck
pnpm lint
```
