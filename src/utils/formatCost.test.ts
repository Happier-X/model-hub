import assert from "node:assert/strict";
import test from "node:test";
import { formatCost } from "./formatCost.ts";

test("0 显示 $0", () => {
  assert.equal(formatCost(0), "$0");
  assert.equal(formatCost(-0.0001), "$0");
});

test("整数与整价显示", () => {
  assert.equal(formatCost(1.25), "$1.25");
  assert.equal(formatCost(10), "$10");
  assert.equal(formatCost(3.375), "$3.375");
});

test("小额去尾 0", () => {
  assert.equal(formatCost(0.0012), "$0.0012");
  assert.equal(formatCost(0.0005), "$0.0005");
  assert.equal(formatCost(0.001), "$0.001");
  assert.equal(formatCost(0.00004), "$0"); // 4 位小数下舍入为 0
});

test("负数钳制为 $0", () => {
  assert.equal(formatCost(-5), "$0");
});
