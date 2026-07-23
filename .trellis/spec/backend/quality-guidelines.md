# Quality Guidelines

> 后端/壳质量门禁（内嵌代理）。

---

## Standards

1. **Windows MVP** 验收优先。
2. **进程安全**：退出路径 stop 内嵌代理。
3. **默认 `127.0.0.1`**；改非本机须提示风险。
4. **无客户端 API Key 鉴权**：本机 `/v1/*` 不校验客户端 Key；任意 `Authorization` 忽略。
5. **禁止**提交真实 Key、无说明的大型 exe。
6. HTTP 代理作为 Tauri 进程内异步任务运行，禁止增加外部代理进程依赖。

---

## Testing（最低）

| 类型 | 要求 |
|------|------|
| 单元 | 队列、路径、结构化错误信封识别 |
| 集成 | 无 Key 可访问；任意 Bearer 不 401；models 分组列表；任意失败换源成功（wiremock） |

```powershell
cd src-tauri
cargo test
cargo check
```

---

## Review Checklist

- [ ] 数据目录可发现
- [ ] 代理启停与 Base URL；首选端口占用时自动改口并持久化
- [ ] 无密钥进日志
- [ ] 默认只监听本机
- [ ] `/v1` 无 Key 可访问；任意 Bearer 也不 401
