# NOTICE — octopus 上游组件

> **重要：当前 Windows 发布包不再内嵌 octopus 二进制，也不再随包分发本目录合规材料。**  
> 本目录仅作历史参考与开发/高级用户**可选回退**（自备二进制 + `MODEL_HUB_GATEWAY_IMPL=octopus`）时的许可证说明。  
> 默认网关实现为仓库内 `gateway-rust`（`model-hub-gateway`）。

Model Hub 历史上在 Windows 安装包中曾内嵌 [bestruirui/octopus](https://github.com/bestruirui/octopus) 的预编译 Windows x64 二进制，作为阶段 1 的本地 LLM 网关侧车。

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
- **默认**渠道/分组/转发/SQLite 等业务由内嵌的 **gateway-rust** 提供。
- 若开发/高级用户**自行**下载并运行 octopus，须自行遵守 AGPL-3.0；对应源码获取方式见 `SOURCE.md`。
- 本 NOTICE **不是法律意见**；公开分发前请自行评估合规要求。

## 致谢

感谢 octopus 项目作者与贡献者提供的网关实现。
