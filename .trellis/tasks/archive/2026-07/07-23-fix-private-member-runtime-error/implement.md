# 实施计划

1. 修改 `OverviewPage.vue`，将 pendingUpdate 改为 shallowRef，确保 Update 不被 Proxy。
2. 整理待更新资源清理函数，用于取消和重新检查。
3. 增加防回归单测，覆盖类实例身份与私有成员方法调用。
4. 运行 pnpm test:unit、typecheck、lint。
5. 更新 frontend spec，禁止把 Tauri Resource 类实例放入深层 ref/reactive。
