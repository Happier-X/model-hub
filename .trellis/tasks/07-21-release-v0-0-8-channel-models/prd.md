# PRD：v0.0.8 + 渠道拉取模型列表

## 目标

1. 发布 v0.0.8：本地开放鉴权、CORS、Tauri HTTP、scope 修复等一并进入安装包。
2. 创建/编辑渠道时，可根据 Base URL + 上游 API Key **拉取模型列表** 供选择。

## 范围

- 网关：新增本机开放的「探测上游 models」接口（代理 GET `{base}/models`）
- 前端：渠道创建/编辑表单增加「拉取模型」按钮与下拉/可选列表
- 版本 bump 0.0.8 + 发布说明 + tag

## 验收

- [ ] 填 Base URL + Key 后可拉取模型并选中写入 channel.model
- [ ] 发布包含开放鉴权网关，管理 API 无 Token 可访问
- [ ] tag v0.0.8 Actions 成功
