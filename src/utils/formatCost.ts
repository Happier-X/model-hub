/**
 * 美元金额格式化：0 → `$0`；>0 → `$` + 至多 4 位小数去尾 0（如 `$1.25`、`$0.0012`）。
 */
export function formatCost(n: number): string {
  const value = Math.max(0, n);
  if (value === 0) return "$0";
  const fixed = value.toFixed(4);
  const trimmed = fixed.replace(/0+$/, "").replace(/\.$/, "");
  return `$${trimmed}`;
}
