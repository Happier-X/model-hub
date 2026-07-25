# 桌面悬浮条刷新可观测性

## Goal

桌面悬浮条（`OverlayApp.vue`）展示的"当前模型"来自每 2.5s 一次的轮询（`proxyStatus` + `getLastSuccessRequest`）。当它"看起来不更新"时，现有 UI 无法让人现场区分是哪一类原因，只有一个黄色 `·` 提示轮询失败。本任务给悬浮条补上最小可观测性，让使用者能自助判断"不更新"属于以下哪种：

- IPC/后端持续失败（轮询 catch 分支被触发，保留旧数据）
- 当前确实没有新的成功请求落库（`get_last_success_request` 仍返回旧记录）
- 只是固有的轮询/落库延迟

## Requirements

- 轮询失败时把错误对象打到 overlay 窗口的 console，而不是完全静默吞掉，便于开发时排查后端异常。
- tooltip 中补充"最后成功刷新时间"（`lastSuccessPolledAt`，指最近一次 `poll()` 成功返回的本地时刻），让使用者判断轮询是否还在正常进行。
- 保持既有的"失败保留旧数据、不闪烁"行为不变；本任务只做增量观测信息，不改变展示状态机（`view` 派生逻辑）。
- 不改动后端 Rust 代码与数据库查询；改动范围限定在 `src/OverlayApp.vue`。
- 中文文案，与现有 tooltip 风格一致。

## Acceptance Criteria

- [ ] 轮询失败时，overlay 窗口 console 能看到带上下文的错误输出（如 `[overlay] poll failed` + 错误对象）。
- [ ] tooltip 在有数据时追加一行"最后刷新：MM-DD HH:mm:ss"，取自最近一次成功轮询时刻。
- [ ] 轮询持续失败时，`fetchFailed` 黄点仍出现，且 tooltip 的"最后刷新"时间停在最后一次成功的时刻（不随失败轮询前进），从而能一眼看出已多久没成功刷新。
- [ ] `pnpm typecheck` 通过。

## Notes

- 展示状态机与"最近一条成功请求"的语义不变：悬浮条本就是显示 DB 里最近一条 2xx 且无 error 的请求，不是"正在用的模型"。本任务不改这个语义，只补充观测手段。
- 轻量任务，PRD-only。改动集中在单文件前端组件，无需 design.md / implement.md。
