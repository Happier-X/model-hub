# Hook Guidelines

> 自定义 hooks 约定。

---

## Naming

- 以 `use` 前缀：`useGatewayStatus`、`useChannels`。
- 返回值稳定：`{ data, error, isLoading, refetch }` 或领域语义字段。

---

## Patterns

1. **服务端状态**：TanStack Query 的 `useQuery` / `useMutation` 封装在 hooks 或 features 内。
2. **壳状态**：`useGatewayStatus` 轮询或事件订阅 Tauri/health，不与渠道列表 query 混在一个巨大 hook。
3. **副作用**：订阅与 timer 必须 cleanup。

---

## Anti-Patterns

- 在 hook 里做路由跳转到 `/login`。
- 无依赖数组控制的疯狂 `setInterval` 打爆管理 API。
