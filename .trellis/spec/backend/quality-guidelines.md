# Quality Guidelines

> 后端/壳代码质量门禁。

---

## Standards

1. **Windows MVP**：CI/本地验收以 Windows 为准；不引入仅在 macOS 验证的脚本作为唯一路径。
2. **进程安全**：正常退出路径必须尝试优雅停止侧车；超时才允许强制结束，并打 warn。
3. **默认安全绑定**：默认 `127.0.0.1`；改为非本机地址必须有 UI/文档风险提示。
4. **无管理登录**：禁止重新引入 admin 登录作为默认路径（产品 D3）。
5. **客户端网关 Key 必填**：默认 `gateway-rust` 与可选 octopus 回退对 `/v1/*` 强制 `sk-octopus-...`（历史前缀兼容）；管理 JWT 与客户端 Key 分离（管理 JWT 不得当作 `/v1/*` 凭证）。不 patch 侧车做「本机免鉴权」；文档/UI 禁止写「任意占位 api_key」。客户端完整 Key 不得明文持久化（内存/SQLite 均只存哈希）；上游渠道 Key 可明文存本机库，但日志禁止打印完整值。
6. **依赖**：钉版本；默认侧车与可选回退说明写入 `gateway/README.md` 或 lock 说明。
7. **内嵌侧车**：Git **禁止**提交 `tools/**/*.exe`；发布包**仅**内嵌 `model-hub-gateway.exe`（`prepare:gateway-rust`）；**不再**下载/打包 octopus 与上传 AGPL 附件。可选本地 `prepare:octopus` 仅供开发回退，不进发布资源。
8. **进程清理**：测试/脚本只按测试端口或托管 PID 结束进程，**禁止** `Stop-Process -Name octopus`（或按名杀 model-hub-gateway）误杀本机实例。

---

## Testing (MVP 最低)

| 类型 | 要求 |
|------|------|
| 单元 | 路径解析、状态机（idle/starting/running/stopping）可测 |
| 集成（手动可接受） | 启动 → health → 停止；端口占用失败可见 |
| 端到端 | OpenAI Chat 经分组转发（真上游或 mock） |

---

## Forbidden

- 提交真实上游 API Key、个人 access token。
- 在仓库提交巨大无用二进制而不说明来源/许可证。
- 若再次引入/分发 AGPL 相关源码/二进制时不写致谢与许可证说明。
- 文档谎称发布包仍内嵌 octopus，或默认实现仍为 octopus。

---

## Review Checklist

- [ ] 数据目录可发现
- [ ] 启停与健康检查
- [ ] 无密钥进日志
- [ ] 默认只监听本机
- [ ] 用户可见错误有下一步
