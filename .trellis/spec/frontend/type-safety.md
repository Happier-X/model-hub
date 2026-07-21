# 类型安全

> Vue 3、TypeScript 与 Tauri IPC 的跨层类型约定。

## 规则

1. TypeScript 保持 `strict`，Vue 文件由 `vue-tsc --noEmit` 校验。
2. Tauri 命令名、参数和返回类型集中定义在 `src/api/tauri.ts`。
3. 页面调用封装函数，不直接散落 `invoke<any>` 或命令字符串。
4. Rust IPC 结构使用 `snake_case` 序列化；TypeScript 类型字段与实际 JSON 保持一致。
5. 时间字段使用后端返回的统一字符串格式，并仅在展示层格式化。
6. 环境变量只允许通过 `import.meta.env.VITE_*` 暴露给前端，且不得包含真实密钥。

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

1. 先确认 Rust command 的参数名与序列化字段。
2. 在 `src/api/tauri.ts` 添加输入和返回类型。
3. 用泛型调用 `invoke<ResultType>(...)`。
4. 在页面处理中保留错误分支，不用类型断言伪造成功值。

## 禁止模式

- 长期使用 `any`、双重断言或 `@ts-ignore` 绕过跨层契约。
- 关闭严格模式解决短期错误。
- 返回空对象冒充失败的 IPC 响应。
- 在组件中重复定义与 `src/api/tauri.ts` 不一致的领域类型。
