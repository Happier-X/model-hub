# 修复配置到 Pi 参数名

## Goal

修复分组页「配置到 Pi」调用失败：前端以 `group_id` 传参，Tauri 2 期望命令参数键为 `groupId`。

## Requirements

- R1：`exportGroupToPiAgent` 的 invoke 参数键改为 `groupId`。
- R2：排查同类扁平命令参数（如 `force_refresh`）是否同样需要 camelCase，一并修正。
- R3：同步 frontend type-safety 规范，明确 Tauri 2 命令参数键使用 camelCase。
- R4：不改动后端业务逻辑与 Pi 导出语义。

## Acceptance Criteria

- [ ] `export_group_to_pi_agent` 不再报 missing required key groupId。
- [ ] 相关扁平 invoke 参数键与 Tauri 2 约定一致。
- [ ] `pnpm typecheck`、`pnpm lint` 通过。
