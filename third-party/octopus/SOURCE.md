# octopus 对应源码获取

为满足 AGPL-3.0 对随二进制分发时提供**对应源码**的要求，Model Hub 钉扎版本的源码可通过以下公开 URL 获取（与二进制 tag 一致）：

## 推荐

| 格式 | URL |
|------|-----|
| tar.gz | https://github.com/bestruirui/octopus/archive/refs/tags/v0.9.28.tar.gz |
| zip | https://github.com/bestruirui/octopus/archive/refs/tags/v0.9.28.zip |

## 精确版本

| 项 | 值 |
|----|-----|
| tag | `v0.9.28` |
| commit | `b7b053e7fd81911e2062359e93f9dcbd58114bb0` |
| commit 浏览 | https://github.com/bestruirui/octopus/tree/b7b053e7fd81911e2062359e93f9dcbd58114bb0 |
| Releases | https://github.com/bestruirui/octopus/releases/tag/v0.9.28 |

## 本地校验二进制

```powershell
# historical: scripts/prepare-bundled-octopus.ps1 removed; use upstream release zip if needed
```

历史脚本已移除。当前产品不下载/启动 octopus；数据导入请用 model-hub-gateway migrate-octopus。

## 说明

- 上述 archive 由 GitHub 按 tag 生成，对应 tag 指向的树即为本产品内嵌二进制的源码基线。
- 若上游移动或删除 release，请以本仓库 `NOTICE.md` 中记录的 commit 与镜像源为准另行归档。
