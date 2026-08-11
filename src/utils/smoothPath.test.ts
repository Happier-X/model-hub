import assert from "node:assert/strict";
import test from "node:test";
import { buildSmoothPath } from "./smoothPath.ts";

test("空数组返回空字符串", () => {
  assert.equal(buildSmoothPath([]), "");
});

test("单点返回 M 指令", () => {
  assert.equal(buildSmoothPath([{ x: 10, y: 20 }]), "M10.0,20.0");
});

test("两点退化为直线 L 指令", () => {
  assert.equal(buildSmoothPath([{ x: 0, y: 0 }, { x: 40, y: 60 }]), "M0.0,0.0 L40.0,60.0");
});

test("≥3 个点生成含 C（三次贝塞尔）的平滑曲线，不再有直线段", () => {
  const d = buildSmoothPath([
    { x: 0, y: 0 },
    { x: 40, y: 10 },
    { x: 80, y: 5 },
    { x: 120, y: 15 },
  ]);
  assert.ok(d.startsWith("M0.0,0.0"), `应从首点 M 开始，实际：${d}`);
  assert.ok(!d.includes(" L"), `平滑曲线不应再出现直线 L 段，实际：${d}`);
  assert.equal(d.match(/C/g)?.length, 3, `4 点应产生 3 段三次贝塞尔，实际：${d}`);
});

test("共线点也产出合法平滑路径", () => {
  const d = buildSmoothPath([
    { x: 0, y: 0 },
    { x: 10, y: 0 },
    { x: 20, y: 0 },
    { x: 30, y: 0 },
  ]);
  assert.ok(d.startsWith("M0.0,0.0"));
  assert.ok(d.includes("C"));
});