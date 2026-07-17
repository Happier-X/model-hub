# Component Guidelines

> 组件如何编写。

---

## Patterns

1. **函数组件 + TypeScript props**；优先具名导出或页面默认导出二选一，项目内统一。
2. **展示与数据分离**：列表页用 hook/query 取数，组件负责渲染与交互。
3. **网关状态条**全局可见（running / starting / error / port）。
4. **表单**：渠道需 Base URL + 上游 API Key；Key 输入用 password 型，支持显示切换。
5. **无登录**：首屏即主布局，禁止闪登录页。

---

## UI Copy

- 用户文案使用**简体中文**。
- 错误提示包含可行动建议。
- 绑定地址非本机时显示风险提示。

---

## Anti-Patterns

- 为「以后多用户」预埋复杂权限组件。
- 无 loading / empty / error 三态的列表硬渲染。
