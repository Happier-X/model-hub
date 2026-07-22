# 启动时可选自动检查更新

## Goal

在已有「手动检查更新」基础上，支持用户**可选**在应用启动后自动检查更新；默认关闭，发现新版本时仅提示并需确认后下载安装（不静默安装）。

## Requirements

- R1：偏好项 `check_update_on_startup`，**默认 false**。
- R2：持久化到 `shell.json`（与端口同文件），缺字段按 false；改端口保存不得擦除该字段。
- R3：概览页开关 + 文案说明；可与手动检查共用同一套结果 UI。
- R4：启动后（概览 `onMounted`）若开启则自动 `check()`；无更新可不打扰或极简提示；有更新进入「待确认」卡片。
- R5：自动检查失败不弹致命错误，记录简短错误即可。
- R6：更新 `docs/in-app-updater.md`。

## Acceptance Criteria

- [x] 默认不自动检查。
- [x] 开启后重启/再次进入概览会自动检查。
- [x] 有更新须确认才下载安装。
- [x] 保存端口后开关偏好仍在。
- [x] `pnpm typecheck` / `lint` / `cargo test --lib settings` 相关通过。

## Out of Scope

- 后台定时轮询。
- 强制自动下载安装。
