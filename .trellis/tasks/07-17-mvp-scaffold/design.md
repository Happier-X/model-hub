# 设计：MVP 工程脚手架

## 范围

只搭 **可运行空壳**，为 `mvp-gateway-sidecar` 预留模块位置，不实现进程管理逻辑。

## 技术选型

| 层 | 选择 |
|----|------|
| 桌面 | Tauri 2 |
| 前端 | React 19 + Vite + TypeScript |
| 样式 | Tailwind CSS 4（或脚手架稳定的 3.x，以可构建为准） |
| 包管理 | pnpm |
| 路径 | `tauri-plugin-path` 或 Tauri 2 内置 path API |

## 仓库布局（落地）

```
/
├── package.json
├── pnpm-lock.yaml
├── vite.config.ts
├── tsconfig.json
├── index.html
├── src/
│   ├── main.tsx
│   ├── App.tsx
│   ├── index.css
│   ├── api/tauri.ts
│   ├── components/layout/
│   └── pages/ 或 routes/
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/
│   └── src/
│       ├── lib.rs
│       ├── main.rs
│       ├── paths.rs
│       └── error.rs
├── README.md
└── .gitignore
```

## paths 契约

- `app_data_dir`：Tauri 应用数据根
- `config_dir` = `{app_data_dir}/config`
- `gateway_dir` = `{app_data_dir}/gateway`
- `bin_dir` = `{app_data_dir}/bin`
- 首次 `get_paths` 时 `create_dir_all` 上述目录
- 返回 JSON：`{ app_data_dir, config_dir, gateway_dir, bin_dir }`

## 前端占位

- 单页布局 + 简单 state 切换导航（暂不强制 react-router，可引入轻量 router）
- 顶栏/侧栏：网关状态「未连接 / idle」
- ���置页：展示 paths

## 风险

- 本机需 WebView2（Windows 通常已有）
- `create-tauri-app` 交互式可能不适合自动化 → 可用非交互模板或手写最小工程
