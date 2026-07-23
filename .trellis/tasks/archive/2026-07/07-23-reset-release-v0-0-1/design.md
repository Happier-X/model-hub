# 技术设计

## 版本与文档布局

| 路径 | 动作 |
|------|------|
| `package.json` `version` | `0.1.1` → `0.0.1` |
| `src-tauri/Cargo.toml` `version` | `0.1.1` → `0.0.1` |
| `src-tauri/tauri.conf.json` `version` | `0.1.1` → `0.0.1` |
| `src-tauri/tauri.release.conf.json` `version` | `0.1.1` → `0.0.1` |
| `changelog/v0.0.1.md` | 新建，muses 风格中文首发说明 |
| `docs/release-notes-v*.md` | 全部删除 |
| `README.md` / `docs/in-app-updater.md` | 发版步骤改为 `changelog/` |

## Changelog 内容合同

- 文件名：`changelog/v{version}.md`（含 `v` 前缀，对齐 muses）。
- `v0.0.1` 按当前代码能力分模块写「新增 / 能力 / 注意」，不写「相对 v0.1.0」类叙事。
- CI 将文件全文作为 GitHub Release body 主体；可在 body 末尾追加安装包与校验说明。

## Release CI 调整

文件：`.github/workflows/release-windows.yml`

1. 从 tag 解析 `VERSION`（去掉 `v` 前缀）。
2. 若存在 `changelog/v${VERSION}.md`，以其内容作为 `releaseBody`；否则使用简短占位。
3. 继续使用 `tauri-apps/tauri-action` 构建并上传 updater 资产（NSIS、`.sig`、`latest.json`）。
4. 收集步骤：
   - 复制 NSIS `.exe`；
   - 可选：将 `changelog/v${VERSION}.md` 作为附件上传（与 muses「正文 + 产物」一致即可，附件非必须）；
   - 生成 `SHA256SUMS.txt`；
   - **不再**复制 `docs/release-notes-v*.md`。
5. 校验步骤保持：`latest.json` 版本与 tag 一致、签名存在。

## 历史痕迹清理顺序

1. 列出本地 tags / `git ls-remote --tags` / `gh release list`。
2. 对每个历史 tag：`gh release delete <tag> --yes`（有 Release 时）。
3. `git push origin --delete <tag>` 删除远端 tag。
4. `git tag -d <tag>` 删除本地 tag。
5. 仓库内删除旧 release-notes 文件并提交。

注意：删除远端 tag/Release 不可轻易回滚，属已确认的完整重置。

## 推送与触发

1. 功能与文档提交落在 `master`。
2. `git push origin master`（当前 ahead）。
3. `git tag v0.0.1` 打在含版本与 changelog 的提交上。
4. `git push origin v0.0.1` 触发 `release-windows`。

## 风险

- 旧安装包用户的应用内更新：`latest.json` 版本变为 `0.0.1` 后，已装 `0.1.1` 的客户端可能认为无更新（版本比较语义）。完整重置已接受该后果；用户需手动装新包。
- `Cargo.lock` 若锁定 package 版本字段，需随 `Cargo.toml` 同步检查。
