# 执行计划：升级 happier-ui 0.0.3 + 手写控件替换

## 有序清单

### 门 1：依赖升级（先跑通）
1. `pnpm add happier-ui@0.0.3`（package.json + lockfile）。
2. CSS 入口已确认未变，`main.ts` 无需改。
3. `pnpm typecheck` + `pnpm lint` 冒烟。

### 门 2：HSelect（5 处，低风险）
4. LogsPage：statusClass + pageSize（pageSize 暂留，门 5 再定合并）→ HSelect。
5. GroupsPage：sortMode + bulkProviderId + 队列行内 provider_id → HSelect（bulkProviderId/行内评估 placeholder vs value=0 保留）。
6. `pnpm typecheck` + `pnpm lint`。

### 门 3：HTextarea + HBadge（低风险）
7. ProvidersPage 粘贴框 → HTextarea（实测 font-mono；不契合则记录，缺功能提 issue）。
8. HomePage 状态 + LogsPage 状态码 → HBadge（variant 映射）。
9. `pnpm typecheck` + `pnpm lint`。

### 门 4：HPagination（中风险）
10. LogsPage 翻页 → HPagination；评估是否合并 pageSize（show-size-changer）。合并则删门 2 的 pageSize HSelect。
11. 「筛选 N 条 · 库内 M 条」独立 span 保留。
12. `pnpm typecheck` + `pnpm lint`。

### 门 5：HTable（最高风险，独立回滚点）
13. LogsPage 8 列 table → HTable（cell slot 承接状态码 HBadge / 错误彩字 / 故障转移多行）。
14. ProvidersPage 4 列 table → HTable（cell slot 承接操作列 HButton）。
15. row 类型断言（`row as RequestLog` / `row as Provider`）。
16. 不契合则回退该页 table 保留手写，缺功能提 issue。
17. `pnpm typecheck` + `pnpm lint`。

### 门 6：spec + 全量验证
18. 更新 `component-guidelines.md` 3.1 组件面（含 HTag 不启用理由、缺功能保留位置的 issue 链接）。
19. 若过程中提了 issue，回填链接到 spec/代码注释。
20. 全量：`pnpm lint`、`pnpm typecheck`、`pnpm test:unit`、`pnpm build`、`cargo build`。

## 验证命令

```powershell
pnpm lint
pnpm typecheck
pnpm test:unit
pnpm build
cargo build --manifest-path src-tauri/Cargo.toml
```

## 审查门 / 回滚点

- 门 1 单独可验证（升级跑通）；门 2-5 各自 typecheck+lint 后进下一门。
- 门 5（HTable）是独立回滚点：不佳则单页 `git checkout` 保留手写 table。
- 每个组件替换可独立回退，互不影响。

## 缺功能提 issue

- 发现缺功能 → 记录现象 → 判断能否降级 → `gh issue create --repo Happier-X/happier-ui` → 回填链接。

## 手动验收清单（需 pnpm tauri dev）
- [ ] 依赖 0.0.3，样式无缺失。
- [ ] LogsPage：状态/每页筛选、翻页、表格彩标与多行渲染正常。
- [ ] ProvidersPage：粘贴框识别、供应商表格与操作按钮正常。
- [ ] GroupsPage：排序方式、批量供应商、队列行内供应商选择正常。
- [ ] HomePage：代理状态徽章配色正常。
