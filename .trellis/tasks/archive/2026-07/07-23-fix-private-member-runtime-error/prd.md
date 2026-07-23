# 修复私有成员运行时错误

## Goal

修复应用更新流程中的 `Cannot read private member from an object whose class did not declare it`，确保发现更新后可正常下载安装。

## Root Cause Evidence

- `@tauri-apps/plugin-updater` 的 `Update` 继承 Tauri `Resource`，内部使用 JavaScript 私有成员保存资源 id。
- `OverviewPage.vue` 当前用深层响应式 `ref<Update | null>` 保存 `Update` 类实例。
- Vue 会将类实例转换为响应式 Proxy；随后调用 `update.downloadAndInstall()` 时，方法内的私有成员读取以 Proxy 作为 `this`，不属于原始类实例，因此抛出该错误。

## Requirements

- R1：Updater 的 `Update` 实例不得被 Vue 深层代理。
- R2：保持当前更新检查、进度展示、失败重试、安装后重启行为。
- R3：取消或替换待更新对象时，按需关闭旧 Tauri Resource，避免资源泄漏（安装成功后由 Rust 侧释放的情况除外）。
- R4：增加可执行的防回归验证，至少确认存储后仍保持原始实例身份。

## Acceptance Criteria

- [ ] 点击「下载安装」不再触发 private member 错误。
- [ ] `pendingUpdate` 使用 `shallowRef` 或 `markRaw`，`downloadAndInstall` 的接收者保持原始 Update 实例。
- [ ] 更新 UI、进度回调、取消与错误重试正常。
- [ ] `pnpm test:unit`、`pnpm typecheck`、`pnpm lint` 通过。
