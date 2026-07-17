# Quality Guidelines

> 前端质量门禁。

---

## Standards

1. **无登录路径**：E2E/手动验收不得依赖 admin 密码。
2. **本机工具**：设置页展示监听地址、端口、数据目录（若壳提供）。
3. **可访问性底线**：按钮有文本或 aria-label；表单 label 关联控件。
4. **i18n**：MVP 可仅简体中文硬编码；勿半中半英随机混用同一按钮。
5. **依赖**：与 Vite/React 19 兼容；不强制一次搬空源站所有 Radix 依赖。

---

## Lint / Format

- ESLint + TypeScript；提交前 `pnpm lint` / `npm run lint`（以锁文件工具为准）。
- 格式化 Prettier 或 Biome（脚手架时选定一种，写进 README）。

---

## Forbidden

- 提交 `.env` 含真实 Key。
- 在发行构建中留下 `console.debug` 刷请求体。
- 引入 Next 专用 API 到 Vite 项目。

---

## Review Checklist

- [ ] 无登录可完成渠道/分组配置
- [ ] 网关未启动时有明确状态
- [ ] 列表有 loading/empty/error
- [ ] 不展示完整密钥（可遮罩）
