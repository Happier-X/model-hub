/** 每日热力图数据项：本地自然日 YYYY-MM-DD → 请求数 */
export interface HeatmapValue {
  date: string;
  count: number;
}

export type HeatmapTier = 0 | 1 | 2 | 3;

/**
 * 按最大值分四档：0 无数据，1/2/3 由低到高。
 * max <= 1 时所有正数直接取最高档，保证稀疏数据也有层次。
 */
export function colorTier(count: number, max: number): HeatmapTier {
  if (count <= 0) return 0;
  if (max <= 1) return 3;
  const ratio = count / max;
  if (ratio >= 0.66) return 3;
  if (ratio >= 0.33) return 2;
  return 1;
}
