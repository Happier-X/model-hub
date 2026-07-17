# 执行计划：MVP 工程脚手架

## Checklist

1. [x] 用非交互方式初始化 Tauri 2 + React-TS Vite 工程（保留 `.trellis`）
2. [x] 安装并配置 Tailwind
3. [x] 实现 `paths.rs` + `get_paths` 命令；capabilities 放行
4. [x] 前端：布局占位 + 设置页展示路径 + `api/tauri.ts`
5. [x] 根 README、`.gitignore`
6. [x] 验证：`pnpm build`、`cargo check/test/fmt`、`tauri build --no-bundle` 与 exe 冒烟
7. [x] 更新 `.trellis/spec` 中目录树与跨层路径契约为真实实现
8. [x] trellis-check 全范围检查并修复字段契约一致性

## Validation

```bash
pnpm install
pnpm build
cd src-tauri && cargo check
# 可选：pnpm tauri dev
```

## Rollback

删除 `src/`、`src-tauri/`、前端配置文件；保留 `.trellis/`。
