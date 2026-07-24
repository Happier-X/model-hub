# 首页功能拆分到设置 - 设计

## 边界

本任务只调整前端信息架构和页面职责，不新增后端命令，不修改配置字段、数据库或代理行为。

## 路由与导航

- 新增 `src/pages/SettingsPage.vue`。
- `src/router/index.ts` 新增 `{ path: "/settings", name: "settings", component: SettingsPage, meta: { title: "设置" } }`。
- `src/components/AppShell.vue` 侧边栏新增「设置」入口。

## 页面职责

### 首页 `HomePage.vue`

保留高频状态与日常操作：

- 今日请求统计与最近成功请求：`getRequestStats`、`getLastSuccessRequest`。
- 代理状态、Base URL、监听地址：`proxyStatus`。
- 复制 Base URL、启动、停止、刷新：`proxyStart`、`proxyStop`、`proxyStatus`。
- 本机接入步骤和 curl 示例。

移除：

- 端口输入、保存端口和 `proxySetPort`。
- 数据目录展示和 `getPaths`。
- 应用更新检查、下载安装、自动检查偏好和相关更新状态。

### 设置页 `SettingsPage.vue`

承载低频配置：

- 代理配置：读取当前代理状态初始化端口，保存端口调用 `proxySetPort`；展示数据目录调用 `getPaths`。
- 应用更新：迁移现有更新检查、下载、安装、取消和当前版本状态逻辑。
- 自动检查偏好：读取 `getShellPrefs`，保存 `setCheckUpdateOnStartup`，文案为「应用启动时自动检查更新（仍需确认后才安装）」。

## 自动检查更新

- 触发时机：应用启动后自动检查一次。
- 落点：`AppShell.vue` 的 `onMounted` 或内部 helper，读取 `getShellPrefs()`，若 `check_update_on_startup` 为 true，则静默调用 `checkForUpdate()`。
- 行为：
  - 有新版本时，在 `AppShell` 壳顶部显示一条可关闭的轻量提示（本地 `ref` 状态，不新增全局通知系统），文案例如「发现新版本 x.x.x」，提供「前往设置」链接跳转 `/settings` 和关闭按钮。
  - 无更新或失败不阻塞应用初始化，也不弹提示。
  - 不在路由切换时重复触发。
  - 启动检查只调 `checkForUpdate()` 探测版本，取得结果后立即释放 `Update` 资源，只保留版本号用于提示；不持有跨页面资源。

## 状态与复用

- 本次不新增大型全局状态管理。
- 首页和设置页分别调用现有 invoke：
  - 首页关注运行状态。
  - 设置页关注配置和更新。
- 如果实现时发现更新逻辑重复超过可接受范围，可提取 `src/composables/useUpdater.ts`，但仅在能减少真实重复和保持资源释放语义清晰时提取。

## 文档与规范

需要同步：

- `.trellis/spec/frontend/index.md` 信息架构加入设置。
- `.trellis/spec/frontend/directory-structure.md` 加入 `SettingsPage.vue` 并更新 HomePage 职责。
- `.trellis/spec/frontend/quality-guidelines.md` 中首页职责、设置职责与启动检查语义。
- README/docs 中涉及首页端口、数据目录、更新入口的说明。

## 兼容性

- 路径 `/` 不变。
- 现有配置字段 `check_update_on_startup` 不变。
- 端口保存、更新下载安装、重启流程保持现有 invoke 和错误处理语义。
