# 执行计划：网关 API Key 与 Chat 可验收

## 清单

1. [x] 真机探测 create 最小 body（测试端口）；确认 list/delete 与 `/v1/models` 用 Key
2. [x] 新增 `src/api/apikey.ts`（list/create/update/delete + mask）
3. [x] 新增 `ApiKeysPage`；`Sidebar`/`App` 挂载导航
4. [x] 修正设置页 ClientHint 与相关文案
5. [x] 更新 docs + gateway/README + README
6. [x] 扩展 `scripts/e2e-octopus-smoke.py`：create key → models 非 401
7. [x] `pnpm lint` / `pnpm build` / `cargo test` / smoke
8. [x] 按需更新 `.trellis/spec`（frontend 目录/API 约定）

## 验证命令

```bash
python scripts/e2e-octopus-smoke.py
pnpm lint
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
```

## 审查门

- 不出现「任意 api_key 占位即可」的错误文档
- 冒烟不按进程名 `Stop-Process octopus`
- Key 明文仅在创建成功时强调展示

## 回滚点

- 每步仅触及前端或文档；smoke 失败时回退 apikey 载荷字段后重试
