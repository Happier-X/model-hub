# 技术设计

## 修复方案

将 `pendingUpdate` 从 `ref<Update | null>` 改为 `shallowRef<Update | null>`。`shallowRef` 只追踪 `.value` 替换，不递归代理 `Update` 实例，因此调用实例方法时 `this` 保持原对象。

不使用对象展开或 JSON 复制，因为 `Update` 的 Resource id 与原型方法不可序列化。

## 生命周期

- 检查到更新：直接赋值原始 `Update` 实例。
- 下载失败：保留实例以允许重试。
- 用户取消：调用 `update.close()` 释放 updater resource 后清空引用；关闭失败不阻塞 UI 清空。
- 安装成功：`downloadAndInstall` 的 Rust 侧已处理资源，直接清空引用并重启。
- 新一轮检查前若有旧实例，先安全释放。

## 验证

提取/新增小型响应式身份测试，证明 `shallowRef(instance).value === instance`；同时执行类型检查与 lint。
