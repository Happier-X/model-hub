# Type Safety

> TypeScript 约定。

---

## Rules

1. `strict` 开启（脚手架默认严格模式）。
2. **API 响应**定义 interface/type；禁止在业务层长期 `as any`。
3. Tauri invoke 命令名与返回类型集中声明（如 `src/api/tauri.ts`）。
4. 环境变量：仅 `import.meta.env.VITE_*` 暴露给前端。

---

## API Types

- 渠道、分组、日志条目等与侧车 JSON 字段对齐；字段变更时同时改类型与 UI。
- 时间字段统一 ISO 字符串或统一 unix，并在 UI 层格式化。

---

## Anti-Patterns

- 关闭 `strict` 换速度。
- 网络错误用空对象冒充成功数据。
