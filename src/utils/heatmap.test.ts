import assert from "node:assert/strict";
import test from "node:test";
import { colorTier } from "./heatmap.ts";

test("0 与负数归为无数据档", () => {
  assert.equal(colorTier(0, 100), 0);
  assert.equal(colorTier(-3, 100), 0);
});

test("max <= 1 时正数直接取最高档", () => {
  assert.equal(colorTier(1, 0), 3);
  assert.equal(colorTier(1, 1), 3);
});

test("按比例分三档", () => {
  assert.equal(colorTier(1, 100), 1);
  assert.equal(colorTier(50, 100), 2);
  assert.equal(colorTier(65, 100), 2);
  assert.equal(colorTier(66, 100), 3);
  assert.equal(colorTier(100, 100), 3);
});
