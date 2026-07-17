# Bootstrap Task: Fill Project Development Guidelines

**You (the AI) are running this task. The developer does not read this file.**

## Status

- [x] Fill backend guidelines
- [x] Fill frontend guidelines
- [x] Add code examples（目标约定级示例与路径；仓库尚无业务代码，示例为规划布局与规则，脚手架后修订）

## What was done

按产品任务 `07-17-tauri-port-octopus` 已确认决策（D1–D7）写入 **目标栈** spec：

- Backend：Tauri/Rust 壳 + 可替换网关侧车、SQLite only、错误/日志/质量
- Frontend：React + Vite + TS + Tailwind、无登录、Query/Zustand 边界

所有文件在 `.trellis/spec/backend/` 与 `.trellis/spec/frontend/`。  
标注为 Target：代码落地后应用真实路径替换「目标约定」。

## Completion

开发者确认后：

```bash
python ./.trellis/scripts/task.py finish
python ./.trellis/scripts/task.py archive 00-bootstrap-guidelines
```

（若当前会话 active 为 octopus 任务，先 `task.py start 00-bootstrap-guidelines` 再 finish/archive，或按 task.py 实际支持的方式归档。）
