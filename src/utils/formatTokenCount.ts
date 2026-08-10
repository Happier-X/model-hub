/**
 * token 计数缩写：<1000 原样；≥1000 显示 x.xk（1 位小数，去尾 0）。
 */
export function formatTokenCount(n: number): string {
  const value = Math.max(0, Math.round(n));
  if (value < 1000) return String(value);
  const k = value / 1000;
  const rounded = Math.round(k * 10) / 10;
  return `${rounded}k`;
}

/** 千分位格式化。 */
export function formatNumber(n: number): string {
  return Math.max(0, Math.round(n)).toLocaleString("zh-CN");
}
