# 组合式函数规范

> 本文件保留原文件名；在 Vue 3 中统一称为组合式函数。

## 命名与位置

- 使用 `use` 前缀，例如 `useProxyStatus`、`useProviders`。
- 可复用的组合式函数放在 `src/composables/`；仅单页使用的简单状态可直接留在页面组件。
- Tauri 命令调用和跨层类型仍集中在 `src/api/tauri.ts`，组合式函数只负责编排。

## 返回契约

按领域返回明确的响应式字段，例如：

```ts
const { data, error, loading, refresh } = useProviders()
```

- `data`、`error` 和 `loading` 使用 `ref` 或 `computed`。
- 写操作应暴露领域动词，如 `create`、`update`、`remove`，不要暴露通用的无类型请求函数。

## 副作用

1. 初始化加载使用 `onMounted` 或显式 `refresh`。
2. 定时刷新必须可停止，并在 `onUnmounted` 清理。
3. 避免多个页面同时高频轮询代理状态；优先由上层布局统一刷新并通过 props 或注入共享。
4. 异步失败必须写入可展示的错误状态，不能静默转换为空数据。

## 禁止模式

- 在组合式函数中跳转到不存在的登录页。
- 绕过 `src/api/tauri.ts` 直接拼接管理 HTTP 请求。
- 创建无法清理的定时器或事件监听。
- 为简单页面状态引入额外的远端缓存库。
