# State Management

> 状态放哪里。

---

## Split

| 状态类型 | 方案 | 例子 |
|----------|------|------|
| 服务端数据 | TanStack Query | 渠道列表、分组、日志 |
| UI / 偏好 | Zustand 或组件 state | 侧栏折叠、主题、表格筛选 |
| 网关进程 | Query + Tauri | running、port、最后错误 |
| 密钥 | **不**长期放全局明文 store；表单本地 state + 提交到侧车 | 上游 API Key |

---

## Rules

1. 能 Query 缓存的不要再复制到 Zustand。
2. Zustand 保持薄，避免变成第二后端。
3. MVP **不需要**用户 session store。

---

## Anti-Patterns

- Redux 全家桶无必要引入。
- 把整份 config.json 镜像进前端且双向不同步。
