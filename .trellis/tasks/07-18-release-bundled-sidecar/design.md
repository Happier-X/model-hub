# 设计：v0.0.1 内嵌侧车 + GitHub Actions 公开释放

## 范围

| 交付 | 说明 |
|------|------|
| 运行时 | 从安装资源部署内置 `octopus.exe` 到 app data |
| 构建 | 发布 config + prepare 脚本（SHA 校验） |
| CI | `.github/workflows/release-windows.yml` |
| 版本 | 全仓 **0.0.1** |
| 合规 | `third-party/octopus/` + Release notes |
| 后续 | 另建 Rust 原生网关父任务（仅规划） |

## 版本钉扎

| 项 | 值 |
|----|-----|
| App | 0.0.1 / tag `v0.0.1` |
| Octopus | v0.9.28 / tag commit `b7b053e7fd81911e2062359e93f9dcbd58114bb0` |
| Asset | `octopus-windows-x86_64.zip` |
| Source archive | `https://github.com/bestruirui/octopus/archive/refs/tags/v0.9.28.tar.gz`（及 zip） |

实现时脚本内写死 zip/exe 的 SHA-256（首次从本机下载结果计算后固化）。

## 运行时

```text
prepare_bundled_binary(app, bin_dir, resource_dir)
  if MODEL_HUB_GATEWAY_BIN 有效文件 → 使用
  target = bin_dir/octopus.exe
  source = resource_dir/sidecar/octopus.exe
  if source 存在 → 按 hash 原子部署到 target 并返回 target（安装态真源）
  if target 存在 → 返回 target（开发无内嵌时的回退）
  else → BinaryMissing 可行动错误
```

调用点：`gateway_start` / `start_managed` / `try_autostart` 之前。

## 发布配置

`src-tauri/tauri.release.conf.json`（合并到默认 conf）：

```json
{
  "version": "0.0.1",
  "bundle": {
    "targets": ["nsis"],
    "resources": {
      "../tools/octopus/octopus.exe": "sidecar/octopus.exe",
      "../third-party/octopus/": "third-party/octopus/"
    }
  }
}
```

`package.json` scripts：

```json
"release:windows": "powershell -ExecutionPolicy Bypass -File scripts/prepare-bundled-octopus.ps1 && tauri build --bundles nsis -c src-tauri/tauri.release.conf.json"
```

（路径以 CLI 实测为准。）

## GitHub Actions

触发：`push: tags: ['v*']`  
Runner：`windows-latest`

```yaml
jobs:
  release-windows:
    steps:
      - checkout
      - setup-node (pnpm)
      - setup-rust (stable)
      - prepare-bundled-octopus.ps1
      - pnpm install --frozen-lockfile
      - pnpm tauri build --bundles nsis -c ...
      - hash nsis exe → SHA256SUMS.txt
      - softprops/action-gh-release
          files: nsis/*.exe, SHA256SUMS.txt, third-party notes
          body: 从 docs/release-notes-0.0.1.md 或 heredoc
          generate_release_notes: true (可选)
```

权限：`contents: write`（创建 Release）。  
不使用 secrets 放密钥；侧车从公开 GitHub release 下载。

### 发布操作流程（人工）

1. 合并主线，版本改 0.0.1  
2. 推送 tag `v0.0.1`  
3. 等待 Actions 成功  
4. 检查 Release 页面附件与 notes  

本任务可在本地先跑通 `release:windows` 再推 tag；若本机无 push 权限，则以 workflow 文件 + 本地验证为完成门槛，由用户推 tag。

## 合规包

`third-party/octopus/`：

- `LICENSE-AGPL-3.0.txt`
- `NOTICE.md`
- `SOURCE.md`（对应源码 URL + commit）

Release body 重复关键链接。

## Rust 原生父任务（后续）

创建 `rust-native-gateway` 父任务 PRD 大纲（不实现）：契约、Chat、流式、路由、日志、迁移、去侧车。

## 风险

| 风险 | 处理 |
|------|------|
| SmartScreen | notes 说明未签名 |
| AGPL | 许可证 + 对应源码链接；非法律意见 |
| CI 超时/体积 | NSIS only；缓存 cargo/pnpm |
| resource_dir 路径 | 安装后实测；单测部署逻辑 |
