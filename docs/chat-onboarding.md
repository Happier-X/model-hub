# Chat 上手与故障排查

从零把 **OpenAI 兼容客户端** 接到本机 Model Hub（默认 **Rust 原生网关**）并跑通 Chat。

## 你需要准备

| 项 | 说明 |
|----|------|
| Windows + Model Hub | 安装版或 `pnpm tauri dev` |
| 内置网关 | 安装版已内嵌 **model-hub-gateway**；开发见 [gateway/README.md](../gateway/README.md) |
| **真实上游 API Key** | 渠道里的供应商 Key（假 Key 只能测鉴权，Chat 通常业务失败） |
| 网关客户端 Key | 应用 **API 密钥** 页创建的 `sk-octopus-...`（历史前缀，兼容命名） |

## 五分钟配置

1. **设置** → 启动网关 → 状态「运行中」。  
2. **渠道** → 新建 OpenAI Chat（`type=0`）：真实 Base URL + **真实上游 Key** + 上游模型名。  
3. **分组** → 新建分组：  
   - **分组名** = 客户端请求里的 `model`  
   - 绑定渠道，`model_name` = 上游真实模型  
4. **API 密钥** → 创建并**完整复制一次** `sk-octopus-...`（与设置页管理 JWT **不是**同一套）。  
5. **仪表盘** → 确认检查清单 1–5 完成；或使用「客户端路径自检」。

## 验证鉴权：`GET /v1/models`

```bash
curl -sS "http://127.0.0.1:8080/v1/models" \
  -H "Authorization: Bearer sk-octopus-YOUR_KEY"
```

- **期望**：HTTP **非 401**（常见 200；`data` 为空也可能）。  
- **401**：Key 错、用了管理 JWT、或未创建网关 Key。

## 验证 Chat：`POST /v1/chat/completions`

将 `your-group-name` 换成**分组名**（不是上游模型名）：

```bash
curl -sS "http://127.0.0.1:8080/v1/chat/completions" \
  -H "Authorization: Bearer sk-octopus-YOUR_KEY" \
  -H "Content-Type: application/json" \
  -d "{\"model\":\"your-group-name\",\"messages\":[{\"role\":\"user\",\"content\":\"hi\"}]}"
```

- **200 + choices**：真上游转发成功。  
- **非 401 的 4xx/5xx**（如 502）：**鉴权已过**，查上游 Key、Base URL、分组绑定、`model_name`。  
- **无真实上游时不保证 200**；产品鉴权闭环以 models 非 401 为准。

## 错误对照

| 现象 | 常见原因 | 处理 |
|------|----------|------|
| 连接失败 / status 0 | 网关未启动、端口不对 | 设置启动；Base 默认 `http://127.0.0.1:8080` |
| **401** Authentication failed | 占位 Key、管理 JWT、Key 禁用/过期 | API 密钥页创建并复制完整 `sk-octopus-...` |
| models **200** 但列表空 | 尚无分组或路由未就绪 | 建分组并绑定渠道；再测 chat |
| chat **非 401** 业务错误 | 上游 Key 无效、URL 错、`model_name` 不匹配、无绑定 | 渠道编辑轮换上游 Key；分组页核对绑定 |
| chat 提示 model 不存在 | 客户端 `model` 填成了上游名 | 改成**分组名** |
| 管理 API 401 | 静默 admin 失败或改密 | 设置页粘贴管理 Token（仅管理端） |

## 仪表盘「客户端路径自检」

1. 打开 **仪表盘** → **客户端路径自检**。  
2. 粘贴网关 API Key（**不要**粘管理 JWT）。  
3. 可选填分组名 → **运行自检**。  
4. 解读：  
   - models **401** → Key 问题  
   - models 成功 + chat **非 401** → 鉴权 OK；若 chat 非 200，按上表查上游/分组  
5. Key **仅内存**，不写 localStorage。

## 相关文档

- [客户端对接（接口说明）](./client-integration.md)  
- [M1 验收清单](./mvp-acceptance.md)  
- [网关侧车](../gateway/README.md)  
