/**
 * 用 Catmull-Rom 转 Cubic Bezier 生成平滑 SVG 路径（张力 0.5 = 自然平滑）。
 *
 * 数据点过少时退化为直线/空路径，保证任意点数下都有合理输出：
 * - 0 个点：返回空字符串
 * - 1 个点：返回 `Mx,y`
 * - 2 个点：返回 `Mx,y Lx,y`（直线）
 * - ≥3 个点：首末段用线性收尾、中间自左向右用三次贝塞尔平滑插值
 */
type Pt = { x: number; y: number };

export function buildSmoothPath(pts: Pt[]): string {
  const n = pts.length;
  if (n === 0) return "";
  if (n === 1) return `M${pts[0].x.toFixed(1)},${pts[0].y.toFixed(1)}`;
  if (n === 2) return `M${pts[0].x.toFixed(1)},${pts[0].y.toFixed(1)} L${pts[1].x.toFixed(1)},${pts[1].y.toFixed(1)}`;

  const t = 0.5; // 张力
  let d = `M${pts[0].x.toFixed(1)},${pts[0].y.toFixed(1)}`;

  for (let i = 0; i < n - 1; i++) {
    const p0 = pts[i - 1] ?? pts[i]; // 首点处回退为自身，保持切线水平
    const p1 = pts[i];
    const p2 = pts[i + 1];
    const p3 = pts[i + 2] ?? p2; // 末点处回退，避免悬垂
    const half = (t * 2);
    const c1x = p1.x + ((p2.x - p0.x) / 6) * half;
    const c1y = p1.y + ((p2.y - p0.y) / 6) * half;
    const c2x = p2.x - ((p3.x - p1.x) / 6) * half;
    const c2y = p2.y - ((p3.y - p1.y) / 6) * half;
    d += ` C${c1x.toFixed(1)},${c1y.toFixed(1)} ${c2x.toFixed(1)},${c2y.toFixed(1)} ${p2.x.toFixed(1)},${p2.y.toFixed(1)}`;
  }

  return d;
}