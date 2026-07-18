# NOTICE — octopus 上游组件

Model Hub 在 Windows 安装包中**内嵌**了 [bestruirui/octopus](https://github.com/bestruirui/octopus) 的预编译 Windows x64 二进制，作为阶段 1 的本地 LLM 网关侧车。

## 上游信息

| 项 | 值 |
|----|-----|
| 项目 | [bestruirui/octopus](https://github.com/bestruirui/octopus) |
| 许可证 | **AGPL-3.0**（全文见同目录 `LICENSE-AGPL-3.0.txt`） |
| 钉扎版本 / tag | **v0.9.28** |
| 对应 commit | `b7b053e7fd81911e2062359e93f9dcbd58114bb0` |
| Windows 二进制资源 | `octopus-windows-x86_64.zip` |
| 二进制下载 URL | https://github.com/bestruirui/octopus/releases/download/v0.9.28/octopus-windows-x86_64.zip |
| 二进制 SHA-256（exe） | `38c4238c5c8be0d3e718eb6192c9d06b2e1dcb4222179f625627c67b1e98c0d8` |
| Zip SHA-256 | `17b071b66218f15b574efe08c73b4ec56d6adfd9c08aab3b216728b29ac0f92f` |

## 与 Model Hub 的关系

- Model Hub 桌面壳（Tauri / Rust + 管理 UI）负责进程启停、数据目录与管理界面。
- 渠道/分组/转发/SQLite 等业务由内嵌的 octopus 进程提供。
- 分发内嵌二进制时，须遵守 AGPL-3.0 对**对应源码**的提供义务；对应源码获取方式见 `SOURCE.md`。
- 本 NOTICE **不是法律意见**；公开分发前请自行评估合规要求。

## 致谢

感谢 octopus 项目作者与贡献者提供的网关实现。
