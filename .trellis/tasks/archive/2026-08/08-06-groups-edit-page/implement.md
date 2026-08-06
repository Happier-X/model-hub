# 实现计划：分组新建/编辑独立页

## 清单

1. **路由**
   - [ ] `src/router/index.ts` 增加 `groups-new`、`groups-edit`（`id` 限数字）
   - [ ] 保持 `/groups` 列表路由

2. **侧栏高亮**
   - [ ] `AppShell.vue`：`activeNavKey` 对 `/groups` 前缀匹配
   - [ ] 点击侧栏「分组」仍 `push('/groups')`

3. **GroupFormPage**
   - [ ] 新建 `src/pages/GroupFormPage.vue`
   - [ ] 从 `GroupsPage.vue` 迁出 dialog 表单：useForm、选模、队列、榜单、绑定同步、保存
   - [ ] 按 `route.name` / `params.id` 区分 create/edit
   - [ ] 编辑：`listGroups` 定位；缺失展示错误 + 返回列表
   - [ ] 保存成功 / 取消 → `router.push({ name: "groups" })`
   - [ ] 无 dirty 守卫；保存中防重复提交

4. **GroupsPage 瘦身**
   - [ ] 删除 `AppDialog` 与 dialog 专用状态/函数/模板
   - [ ] `openCreate` / `startEdit` 改为路由跳转
   - [ ] 保留卡片即时保存、删除、导出 Pi

5. **验收自测**
   - [ ] 新建 / 编辑 / 取消 / 保存回列表
   - [ ] 绑定只读 + 同步
   - [ ] 侧栏高亮
   - [ ] 非法 id / 不存在 id
   - [ ] 左栏不预拉

6. **质量**
   - [ ] `pnpm` lint / typecheck / 相关 unit test
   - [ ] 更新 frontend spec：`component-guidelines` 双栏从 dialog 改为独立页；`directory-structure` 登记新页面

## 验证命令

```bash
pnpm exec vue-tsc --noEmit
pnpm test   # 或项目既有 test 脚本；至少跑 groupSaveMode
pnpm lint   # 若有
```

（以 `package.json` scripts 为准，实现时读取后选用。）

## 风险文件

- `src/pages/GroupsPage.vue`（大段删除，易回归列表操作）
- `src/components/AppShell.vue`（高亮影响所有 nav）
- `src/router/index.ts`

## 回滚点

- 每完成「路由+空页」「表单迁入」「列表瘦身」可视为可回滚节点
- 无 DB/配置迁移

## start 前

- [x] prd / design / implement 齐备
- [ ] implement.jsonl / check.jsonl 已填真实 spec
- [ ] 用户批准本规划摘要
