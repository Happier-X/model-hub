# 实施计划

1. 全库审计：启动路径、定时器、`fetch_provider_models` 调用点、是否有非用户触发的上游 HTTP。
2. 新建 `backend/upstream-access.md`（完整 code-spec 七段）。
3. 更新 `error-handling.md`、`component-guidelines.md`、backend/frontend `index.md`；必要时改 docs 中易误解文案。
4. 若发现自动探测：删除并补回归说明。
5. 验证：`rg` 审计结论；若改了 TS 则 typecheck/lint。

## 验证

```powershell
rg -n "fetch_provider_models|fetchProviderModels|interval|setInterval|on_start|warmup|测活" src src-tauri -g'*.{rs,ts,vue}'
```
