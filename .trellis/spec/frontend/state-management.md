# 状态管理

> Vue 3 管理台当前采用组件状态与少量组合式函数，不引入全局状态库。

## 状态归属

| 状态类型 | 位置 | 示例 |
|----------|------|------|
| 页面业务数据 | 页面内 `ref` / `reactive` | 供应商、分组、日志列表 |
| 派生状态 | `computed` | 过滤结果、按钮可用状态、Base URL 文案 |
| 可复用异步状态 | `src/composables/` | 代理状态轮询、通用加载流程 |
| 跨层调用 | `src/api/tauri.ts` | CRUD、代理启停、路径与健康快照 |
| 表单密钥 | 当前组件局部状态 | 上游供应商 Key |

## 规则

1. SQLite 是业务数据真源；写操作完成后重新加载或精确更新本地响应式数据。
2. 不把整份后端配置复制为长期全局状态。
3. 上游 Key 明文不得写入 `localStorage`、会话存储或其它前端持久化。
4. 共享状态确有多个远离组件需要时，优先使用组合式函数或 provide/inject；只有复杂度明确增长后才评估 Pinia。
5. 应用不维护用户会话状态。
6. **Tauri Resource 类实例**（如 `@tauri-apps/plugin-updater` 的 `Update`）必须用 `shallowRef` / `markRaw` 保存，**禁止**放入深层 `ref` / `reactive`。深层代理会破坏 JS 私有成员，调用 `downloadAndInstall` 等实例方法时抛出 `Cannot read private member...`。

## 禁止模式

- 页面和全局状态分别维护同一份业务数据并产生双源真相。
- 网络或 IPC 错误时用空数据覆盖上一次有效内容且不提示。
- 为侧栏开关等简单状态引入大型状态框架。
- 把依赖私有字段的原生类实例（尤其是 Tauri Resource）放进深层响应式容器。
