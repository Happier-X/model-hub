import assert from "node:assert/strict";
import test from "node:test";
import { renderMarkdown } from "./markdown.ts";

test("空输入返回空串，调用方可据此不渲染容器", () => {
  assert.equal(renderMarkdown(""), "");
  assert.equal(renderMarkdown("   \n  "), "");
  assert.equal(renderMarkdown(null), "");
  assert.equal(renderMarkdown(undefined), "");
});

test("标题与列表被解析为标签，不再残留 markdown 标记符号", () => {
  const html = renderMarkdown("# v0.1.1\n\n## 功能\n\n- 第一条\n- 第二条");
  assert.match(html, /<h1>v0\.1\.1<\/h1>/);
  assert.match(html, /<h2>功能<\/h2>/);
  assert.match(html, /<ul>/);
  assert.match(html, /<li>第一条<\/li>/);
  // 渲染结果里不应出现原始的行首标记
  assert.ok(!html.includes("# v0.1.1"));
  assert.ok(!html.includes("- 第一条"));
});

test("内联代码与代码块渲染为 code / pre", () => {
  assert.match(renderMarkdown("路径 `src/utils` 下"), /<code>src\/utils<\/code>/);
  const block = renderMarkdown("```\npnpm build\n```");
  assert.match(block, /<pre><code>pnpm build\n<\/code><\/pre>/);
});

test("原始 HTML 被转义，不产生可执行节点（XSS 防线）", () => {
  const html = renderMarkdown('正常 <img src=x onerror=alert(1)> <script>alert(2)</script>');
  assert.ok(!html.includes("<img"));
  assert.ok(!html.includes("<script"));
  assert.match(html, /&lt;img src=x onerror=alert\(1\)&gt;/);
  assert.match(html, /&lt;script&gt;/);
});

test("链接带 target=_blank 与 rel=noopener，交给系统浏览器打开", () => {
  const html = renderMarkdown("[发布页](https://github.com/Happier-X/model-hub/releases)");
  assert.match(html, /<a href="https:\/\/github\.com\/Happier-X\/model-hub\/releases"/);
  assert.match(html, /target="_blank"/);
  assert.match(html, /rel="noopener noreferrer"/);
});

test("linkify 让裸 URL 也变成安全链接", () => {
  const html = renderMarkdown("详见 https://example.com/notes 说明");
  assert.match(html, /<a href="https:\/\/example\.com\/notes"/);
  assert.match(html, /target="_blank"/);
  assert.match(html, /rel="noopener noreferrer"/);
});

test("javascript: 伪协议不被识别为链接，原文转义后不产生 <a> 节点", () => {
  const html = renderMarkdown("[点我](javascript:alert(1))");
  // markdown-it 不把括号/伪协议解析为链接，整行当作普通文本（已被 html:false 转义），
  // 因此不会产生 <a href="javascript:..."> 这种可点击脚本节点。
  assert.ok(!html.includes("<a "));
  assert.ok(!html.includes('href="javascript'));
});

test("真实发布日志片段：标题 / 列表 / 内联代码 / 分隔线组合渲染", () => {
  const body = [
    "# v0.1.1 (2026-08-06)",
    "",
    "## 功能",
    "",
    "- 分组新建/编辑改为独立页面：路由 `/groups/new`",
    "",
    "---",
    "",
    "### 构建产物",
    "",
    "- Windows NSIS 安装包",
  ].join("\n");
  const html = renderMarkdown(body);
  assert.match(html, /<h1>v0\.1\.1 \(2026-08-06\)<\/h1>/);
  assert.match(html, /<h3>构建产物<\/h3>/);
  assert.match(html, /<hr>/);
  assert.match(html, /<code>\/groups\/new<\/code>/);
});
