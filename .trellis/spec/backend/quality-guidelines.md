# Quality Guidelines

> 后端/壳质量门禁（内嵌代理）。

---

## Standards

1. **Windows MVP** 验收优先。
2. **进程安全**：退出路径 stop 内嵌代理。
3. **默认 `127.0.0.1`**；改非本机须提示风险。
4. **客户端 API Key 可选**：本机默认可不带 Key；若请求携带了 Key，则必须有效且启用。
5. **禁止**提交真实 Key、无说明的大型 exe。
6. HTTP 代理作为 Tauri 进程内异步任务运行，禁止增加外部代理进程依赖。

---

## Testing（最低）

| 类型 | 要求 |
|------|------|
| 单元 | 熔断、队列、路径、Key 校验 |
| 集成 | 无 Key 可访问；错误 Key 401；models 分组列表；5xx 换源成功（wiremock） |

```powershell
cd src-tauri
cargo test
cargo check
```

---

## Review Checklist

- [ ] 数据目录可发现
- [ ] 代理启停与 Base URL
- [ ] 无密钥进日志
- [ ] 默认只监听本机
- [ ] `/v1` 无 Key 可访问；错误 Key 拒绝
