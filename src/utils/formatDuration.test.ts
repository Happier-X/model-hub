import assert from "node:assert/strict";
import test from "node:test";
import { formatDuration } from "./formatDuration.ts";

test("小于 1 秒显示毫秒", () => {
  assert.equal(formatDuration(0), "0 ms");
  assert.equal(formatDuration(500), "500 ms");
  assert.equal(formatDuration(999), "999 ms");
});

test("1~60 秒显示 x.x s", () => {
  assert.equal(formatDuration(1000), "1.0 s");
  assert.equal(formatDuration(1500), "1.5 s");
  assert.equal(formatDuration(59_500), "59.5 s");
});

test("超过 60 秒显示分秒", () => {
  assert.equal(formatDuration(60_000), "1 分 0 秒");
  assert.equal(formatDuration(65_000), "1 分 5 秒");
  assert.equal(formatDuration(3_660_000), "61 分 0 秒");
});

test("负数钳制为 0", () => {
  assert.equal(formatDuration(-5), "0 ms");
});
