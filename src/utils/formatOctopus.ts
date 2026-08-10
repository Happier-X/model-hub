/**
 * octopus 同款数值格式化：返回 { value, unit } 分离，unit 由调用方单独渲染。
 * 规则与 octopus web/src/lib/utils.ts 的 formatNumber 一致（全部 toFixed(2)）。
 */

interface OctopusFormatted {
  value: string;
  unit: string;
}

function formatNumber(
  num: number,
  compare: number[],
  units: string[],
): OctopusFormatted {
  if (num >= compare[0]) return { value: (num / compare[0]).toFixed(2), unit: units[1] };
  if (num >= compare[1]) return { value: (num / compare[1]).toFixed(2), unit: units[2] };
  if (num >= compare[2]) return { value: (num / compare[2]).toFixed(2), unit: units[3] };
  if (num >= compare[3]) return { value: (num / compare[3]).toFixed(2), unit: units[4] };
  return { value: num.toFixed(2), unit: units[5] };
}

/** 计数/Token：B / M / K 后缀（≥1e9 B，≥1e6 M，≥1e3 K，否则无单位）。 */
export function formatCount(num: number): OctopusFormatted {
  const value = Math.max(0, num);
  return formatNumber(value, [1_000_000_000, 1_000_000, 1_000, 1], ["", "B", "M", "K", "", ""]);
}

/** 美元金额：B$ / M$ / K$ / $ 后缀。 */
export function formatMoney(num: number): OctopusFormatted {
  const value = Math.max(0, num);
  return formatNumber(value, [1_000_000_000, 1_000_000, 1_000, 1], ["$", "B$", "M$", "K$", "$", "$"]);
}

/** 耗时（毫秒）：d / h / m / s / ms 单位。 */
export function formatTime(ms: number): OctopusFormatted {
  const value = Math.max(0, ms);
  return formatNumber(value, [86_400_000, 3_600_000, 60_000, 1_000], ["", "d", "h", "m", "s", "ms"]);
}
