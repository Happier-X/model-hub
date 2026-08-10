import assert from "node:assert/strict";
import test from "node:test";
import { formatNumber, formatTokenCount } from "./formatTokenCount.ts";

test("formatTokenCount 小于 1000 原样", () => {
  assert.equal(formatTokenCount(0), "0");
  assert.equal(formatTokenCount(999), "999");
});

test("formatTokenCount 千位缩写", () => {
  assert.equal(formatTokenCount(1000), "1k");
  assert.equal(formatTokenCount(1234), "1.2k");
  assert.equal(formatTokenCount(12_345), "12.3k");
  assert.equal(formatTokenCount(1_234_567), "1234.6k");
});

test("formatTokenCount 负数钳制为 0", () => {
  assert.equal(formatTokenCount(-5), "0");
});

test("formatNumber 千分位", () => {
  assert.equal(formatNumber(0), "0");
  assert.equal(formatNumber(999), "999");
  assert.equal(formatNumber(1_000), "1,000");
  assert.equal(formatNumber(12_345), "12,345");
  assert.equal(formatNumber(-3), "0");
});
