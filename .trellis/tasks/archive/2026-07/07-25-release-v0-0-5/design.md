# 技术设计：发布 v0.0.5

## 1. 边界

- 纯发布材料变更：版本号、changelog、README。无功能代码改动。
- 沿用 v0.0.4 已验证的发布链路（本地门禁 → 提交 → 推 master → 推 tag → `release-windows` CI → 校验资产）。

## 2. 版本号同步点（0.0.4 → 0.0.5）

| 文件 | 字段 |
|------|------|
| `package.json` | `"version": "0.0.5"` |
| `src-tauri/Cargo.toml` | `version = "0.0.5"` |
| `src-tauri/tauri.conf.json` | `"version": "0.0.5"` |
| `src-tauri/tauri.release.conf.json` | `"version": "0.0.5"`（若存在该字段） |
| `src-tauri/Cargo.lock` | `model-hub` 包版本 → 用 `cargo check` 或 `cargo update -p model-hub` 同步，不动其他依赖 |

## 3. changelog/v0.0.5.md

沿用 v0.0.4 结构：标题 `# v0.0.5 (YYYY-MM-DD)`，分「新增 / 变更」「修复」「安装与更新」。内容取自 prd Scope。安装与更新段落照抄 v0.0.4（NSIS、SHA256SUMS、latest.json 链接不变）。

## 4. README.md

- 版本徽标/说明行：`0.0.4` → `0.0.5`。
- 更新日志链接：`changelog/v0.0.4.md` → `changelog/v0.0.5.md`，标题文字同步。
- 示例 tag：`v0.0.4` → `v0.0.5`。

## 5. 发布链路

`release-windows.yml` 由 `push tag v*` 触发：tauri-action 构建 NSIS + 签名 updater，创建非 draft Release，校验 latest.json 版本/签名/资产引用一致性，再上传 SHA256SUMS。无需改工作流。

## 6. 回滚点

- 提交前：直接改文件 / `git checkout`。
- 推 master 前：`git reset` 撤销本地发布提交。
- 推 tag 前：`git tag -d v0.0.5` 撤销本地 tag。
- 推 tag 后：不覆盖同 tag 资产；如错误则删 Release/tag 后改发更高版本。

## 7. 阻塞处理

- `gh` 未登录 / 网络不可用：完成到本地提交或推送边界，报告剩余手动步骤，不虚构 CI 结果。
