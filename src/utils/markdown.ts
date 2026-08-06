import MarkdownIt from "markdown-it";

/**
 * 更新日志等外部 markdown 文本的渲染器。
 *
 * `html: false` 会把原始 HTML 转义成文本，是本模块的安全前提：
 * 渲染结果虽然经 `v-html` 注入，但内容里的 `<script>` / `onerror` 等一律不会成为节点。
 * 若将来放开 `html`，必须先引入 sanitizer。
 */
const md = new MarkdownIt({
  html: false,
  linkify: true,
  breaks: false,
});

// 链接一律新窗口打开：Tauri 默认 urlOpenPolicy 为 allow，会交给系统浏览器，
// 避免在应用内 WebView 里导航走丢管理界面。
md.renderer.rules.link_open = (tokens, idx, options, _env, self) => {
  const token = tokens[idx];
  const targetIndex = token.attrIndex("target");
  if (targetIndex < 0) {
    token.attrPush(["target", "_blank"]);
    token.attrPush(["rel", "noopener noreferrer"]);
  } else {
    token.attrSet("target", "_blank");
    token.attrSet("rel", "noopener noreferrer");
  }
  return self.renderToken(tokens, idx, options);
};

/**
 * 把 markdown 源码渲染为可直接注入的 HTML 字符串。
 * 空输入返回空串，便于调用方用 `v-if` 判空而不渲染空容器。
 */
export function renderMarkdown(source: string | null | undefined): string {
  const text = source?.trim();
  if (!text) return "";
  return md.render(text);
}
