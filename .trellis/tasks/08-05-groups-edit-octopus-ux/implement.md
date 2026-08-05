# 执行计划：分组编辑对齐 octopus 交互

## 前置

- [x] PRD 决策 D1–D6 已关闭
- [x] design.md 已写
- [ ] 用户审阅本计划后 `task.py start`（**未批准前不写产品代码**）

## 清单

### 0. 准备

1. [ ] 读 frontend：`component-guidelines`、`model-queue-sort`、`state-management`；backend：`upstream-access`
2. [ ] 确认 `AppDialog` wide 样式；若双栏过窄，增加 `xl` 或仅分组页覆盖 max-width

### 1. 卡片内即时编辑

3. [ ] 抽出或页内实现 `persistGroupItems(group, nextItems)`：组装全量 `updateGroup` payload
4. [ ] 卡片队列：非绑定显示拖动手柄 + 删除；绑定只读
5. [ ] 拖拽松手 → 本地顺序 + `persist`；`cardSaving` 锁
6. [ ] 删成员 → 无确认直接 `persist`（D5=M1）
7. [ ] 删分组：卡片内确认 UI，移除 `window.confirm`
8. [ ] 失败：`error` + `refresh()`；成功可局部替换或 `refresh`

### 2. 双栏编辑对话框

9. [ ] 引入 provider 模型缓存（ensure on expand / refresh）
10. [ ] 表单上方：名称 / 思考强度 / 绑定（保持 vue-form + editingGroupId）
11. [ ] 左侧手风琴 + 搜索已加载 + 点选加入 + 全部加入；绑定态禁用
12. [ ] 右侧队列拖拽/删除/清空 + 能力排序/刷新榜单；绑定态只读 + 立即同步
13. [ ] 移除旧逐行供应商选择 / 独立批量添加条
14. [ ] 提交路径与 `getGroupSaveMode` 不变；失败保留态

### 3. 质量

15. [ ] `pnpm typecheck` / lint / 相关 unit test
16. [ ] 按 PRD AC1–AC13 手测核对
17. [ ] 有价值约定写入 `.trellis/spec/frontend/`（finish 阶段）

## 验证命令

```bash
pnpm typecheck
pnpm exec eslint src/pages/GroupsPage.vue src/components/groups --max-warnings 0
pnpm exec vitest run src/utils/groupSaveMode.test.ts
# 若新增 util 测试一并 run
```

## 回滚点

- 仅前端文件；`git checkout -- src/pages/GroupsPage.vue src/components/groups`（若有）即可。
- 无 DB / IPC 变更。

## 拆分说明

单任务可交付；若实现中文件过大，可在执行期拆子任务，但 **不阻塞 start**：卡片即时编辑与双栏对话框共享同一页状态，强耦合，保持一个 in_progress 任务更合适。
