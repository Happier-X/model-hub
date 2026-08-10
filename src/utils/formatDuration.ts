/**
 * 耗时自适应格式化：<1s 毫秒；<60s x.x s；≥60s x 分 y 秒。
 */
export function formatDuration(ms: number): string {
  const value = Math.max(0, ms);
  if (value < 1000) return `${Math.round(value)} ms`;
  const seconds = value / 1000;
  if (seconds < 60) return `${seconds.toFixed(1)} s`;
  const minutes = Math.floor(seconds / 60);
  const restSeconds = Math.round(seconds % 60);
  return `${minutes} 分 ${restSeconds} 秒`;
}
