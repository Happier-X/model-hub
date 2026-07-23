# 技术设计

## 数据流

GroupsPage「配置到 Pi」→ `export_group_to_pi_agent(group_id)` → 查分组名 + 代理 `base_url` → `pi_export` upsert `providers.model-hub` → 写 `~/.pi/agent/models.json`。

## Provider 形态（方案 A）

```json
{
  "providers": {
    "model-hub": {
      "baseUrl": "http://127.0.0.1:8080/v1",
      "api": "openai-completions",
      "apiKey": "model-hub",
      "models": [
        { "id": "<分组名>", "name": "<分组名>" }
      ]
    }
  }
}
```

- 多次对**不同**分组配置：models 数组累积 upsert（按 `id`）。
- 对**同一**分组再次配置：更新该条，并刷新 provider 级 `baseUrl`/`apiKey`（端口改口后重新配置生效）。
- 首次无 `model-hub`：创建节点。
- 禁止整表用「仅当前一个模型」替换掉已有 models（除非该 id 已是唯一条目）。

## IPC

替换/废弃旧命令 `export_to_pi_agent(api_key?)`：

```text
export_group_to_pi_agent(group_id: i64) -> ExportToPiResult {
  path, provider_id: "model-hub", model_count: /* 写入后 model-hub.models 长度 */,
  base_url, group_name
}
```

- 不再接收 `api_key`；不再返回 `used_placeholder_key`（恒为占位）。
- 前端仅 `exportGroupToPiAgent(groupId: number)`。

## 模块改动

| 位置 | 行为 |
|------|------|
| `pi_export.rs` | `upsert_model_hub_group(path, base_url, group_name)`；内部固定占位 Key；合并逻辑按 model id upsert |
| `commands.rs` | 新 command；删除旧全局全量导出 command（或薄包装转调但前端不再暴露） |
| `GroupsPage.vue` | 列表行按钮 + 成功/失败文案 |
| `ApiKeysPage.vue` | 删除 Pi 配置区块 |
| `tauri.ts` | 类型与 API |
| docs | client-integration、概览改口提示等 |

## 兼容

- 代理仍认占位 Key `model-hub`（`proxy/server.rs` 已有）。
- 用户若曾用旧全局导出写入真实客户端 Key：本次配置会**改回占位** `apiKey`，符合「不要 Key」产品决策。

## 错误

| 条件 | message 方向 |
|------|----------------|
| 分组 id 不存在 | 分组不存在或已删除，刷新后重试 |
| Base URL 空 | 先启动代理或检查端口配置 |
| 读/写 models.json 失败 | 路径 + 权限/JSON 非法提示 |
