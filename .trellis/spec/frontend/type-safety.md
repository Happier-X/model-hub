# 类型安全

> Vue 3、TypeScript 与 Tauri IPC 的跨层类型约定。

## 规则

1. TypeScript 保持 `strict`，Vue 文件由 `vue-tsc --noEmit` 校验。
2. Tauri 命令名、参数和返回类型集中定义在 `src/api/tauri.ts`。
3. 页面调用封装函数，不直接散落 `invoke<any>` 或命令字符串。
4. **返回体 / payload 结构字段**与 Rust serde 一致，当前为 `snake_case`（如 `base_url`、`auto_failover`）。
5. **Tauri 2 命令参数名**在前端 `invoke` 时使用 **camelCase** 键（Rust `group_id` → JS `{ groupId }`，`force_refresh` → `{ forceRefresh }`）。参数名与结构体字段不是同一套规则。
6. 时间字段使用后端返回的统一字符串格式，并仅在展示层格式化。
7. 环境变量只允许通过 `import.meta.env.VITE_*` 暴露给前端，且不得包含真实密钥。

## 类型示例

```ts
export interface ProxyStatus {
  state: "idle" | "starting" | "running" | "stopping" | "error"
  host: string
  port: number
  last_error: string | null
  base_url: string
  data_dir: string
}
```

对新增命令：

1. 先确认 Rust command 的**参数名**（前端 camelCase）与**结构体字段**（serde snake_case）。
2. 在 `src/api/tauri.ts` 添加输入和返回类型。
3. 用泛型调用 `invoke<ResultType>(...)`，扁平参数用 camelCase 键。
4. 在页面处理中保留错误分支，不用类型断言伪造成功值。

## 禁止模式

- 长期使用 `any`、双重断言或 `@ts-ignore` 绕过跨层契约。
- 关闭严格模式解决短期错误。
- 返回空对象冒充失败的 IPC 响应。
- 在组件中重复定义与 `src/api/tauri.ts` 不一致的领域类型。
- 对 Tauri 2 扁平命令参数使用 snake_case 键（会导致 missing required key）。
