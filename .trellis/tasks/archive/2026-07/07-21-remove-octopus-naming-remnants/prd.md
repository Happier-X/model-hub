# PRD：彻底清除 octopus 命名残留

## 背景

运行时兼容侧车已删除。用户要求继续去掉仍带 octopus 字样的产品面：

1. 客户端 Key 前缀 `sk-octopus-...`
2. `migrate-octopus` 一次性导入 CLI 与代码
3. `third-party/octopus/` 合规归档目录

## 目标

产品代码、主文档与 UI **不再**依赖 octopus 命名；Key 使用新产品前缀。

## 范围

### 必须

- Key 前缀改为 **`sk-modelhub-`**（生成、校验、脱敏、前端文案、文档示例）
- **不再**接受旧前缀 `sk-octopus-`（破坏性；文档写明需重建 Key）
- 删除 `gateway-rust` 的 `migrate_octopus` 模块、CLI 子命令与相关测试
- 删除 `third-party/octopus/` 整目录
- 更新 README、`gateway*`、客户端/上手/验收文档与 UI 文案
- 同步 backend/frontend spec 中相关表述

### 不做

- 不改写已发布 `docs/release-notes-v0.0.1.md`～`v0.0.4.md` 历史正文（可保留史实）
- 不自动迁移已有 SQLite 中旧前缀 Key（用户需在 UI 重新创建）
- 不发版（除非用户另行要求 v0.0.5）

## 验收标准

- [x] 代码中无运行时 `sk-octopus` 前缀生成/校验（改为 `sk-modelhub-`）
- [x] 无 `migrate_octopus` / `migrate-octopus` 入口
- [x] 无 `third-party/octopus/` 目录
- [x] 主文档/UI 使用 `sk-modelhub-`
- [x] `cargo test`（gateway-rust + src-tauri）与 `pnpm lint` 通过
