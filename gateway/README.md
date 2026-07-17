# 网关侧车（octopus 兼容）

Model Hub 阶段 1 通过 **外部网关进程** 提供 LLM 聚合能力。桌面壳负责启停与健康检查，业务数据落在应用 `gateway_dir`。

## 许可证

上游项目 [bestruirui/octopus](https://github.com/bestruirui/octopus) 使用 **AGPL-3.0**。下载、修改或随本应用分发其二进制/源码时，请自行遵守 AGPL 义务（包括源码提供与许可证传递），并保留上游致谢。

## 版本钉扎（建议）

| 项 | 值 |
|----|-----|
| 上游仓库 | https://github.com/bestruirui/octopus |
| 建议渠道 | [GitHub Releases](https://github.com/bestruirui/octopus/releases) 的 Windows x64 产物 |
| 运行方式 | `octopus.exe start` |
| 配置 | 工作目录下 `config.json`，可用 `OCTOPUS_*` 环境变量覆盖 |

实现时请在本文件记录你实际使用的 **精确版本号**（例如 `v0.9.28`）与下载校验方式。未钉扎版本可能导致启动参数/鉴权行为不一致。

## 放置路径

桌面应用会在首次运行时创建数据目录，并通过 `get_paths` 暴露：

- `bin_dir`：放置 `octopus.exe`
- `gateway_dir`：侧车工作目录（配置、SQLite、日志）

Windows 默认可执行文件名：

```text
{bin_dir}/octopus.exe
```

也可设置环境变量覆盖：

```text
MODEL_HUB_GATEWAY_BIN=C:\path\to\octopus.exe
```

## 本产品强制约定

| 项 | 约定 |
|----|------|
| 监听地址 | 默认 **`127.0.0.1`**（通过配置/环境变量注入，覆盖上游默认 `0.0.0.0`） |
| 端口 | 默认 **8080** |
| 数据库 | SQLite，路径位于 `gateway_dir` |
| 管理台登录 | Model Hub 自身 UI **无登录**；若上游管理 API 仍要求 admin，由后续管理 UI 任务适配 |
| 客户端网关 Key | MVP **本机免鉴权**；SDK 必填时可填占位 `api_key` |

## 启停

- 应用内：设置页「启动网关 / 停止网关」
- 退出应用时：壳会尝试结束托管的子进程（Windows 上以进程终止为主；请避免在任务管理器中直接强杀作为日常操作）

## 故障排查

1. **未找到 octopus.exe**：按上文放到 `bin_dir` 或设置 `MODEL_HUB_GATEWAY_BIN`。
2. **端口占用**：结束占用 8080 的进程，或后续版本在设置中改端口。
3. **进程启动后无监听**：确认二进制架构为 Windows x64，且版本支持 `start` 子命令。
4. **健康检查超时**：查看 `gateway_dir` 下日志/数据库是否生成。
