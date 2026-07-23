# 技术设计

## 边界

| 删除 | 保留 |
|------|------|
| 客户端 API Key 页 / 路由 / 导航 | 供应商 `providers.api_key` 与上游转发 Bearer |
| `domain/apikey.rs` 全部 | `pi_export` 固定占位写入（仅配置文件） |
| commands `*_api_key` | 分组 / 供应商 / 日志 / Pi 导出 |
| `proxy/server.rs` 客户端鉴权 | `/health`、`/v1/*` 业务 |
| `api_keys` 表 schema / 迁移 / 测试 | 其它表迁移 |
| 文档中的客户端 Key 流程 | 本机无鉴权说明 |

## 代理

删除 `extract_client_key` / `require_key` 及调用；`/v1/*` 直接业务处理。

## 数据库

- 新 schema 与 `CREATE TABLE` 列表去掉 `api_keys`。
- 删除 `ensure_api_keys_columns` 及 api_keys 迁移单测。
- `db/mod` 等测试中对 `list_api_keys` 的断言删除。
- **不**写 DROP TABLE 兼容路径；代码路径零依赖该表。

## 测试

- 删除 invalid/valid/placeholder 客户端 Key 用例。
- 可选：带任意 Bearer 仍 200。
- 故障转移与无鉴权用例保留。
