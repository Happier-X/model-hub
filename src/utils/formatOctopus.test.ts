import assert from "node:assert/strict";
import test from "node:test";
import { formatCount, formatMoney, formatTime } from "./formatOctopus.ts";

test("formatCount 边界（无/K/M/B）", () => {
  assert.deepEqual(formatCount(0), { value: "0.00", unit: "" });
  assert.deepEqual(formatCount(500), { value: "500.00", unit: "" });
  assert.deepEqual(formatCount(999), { value: "999.00", unit: "" });
  assert.deepEqual(formatCount(1234), { value: "1.23", unit: "K" });
  assert.deepEqual(formatCount(12_345), { value: "12.35", unit: "K" });
  assert.deepEqual(formatCount(1_500_000), { value: "1.50", unit: "M" });
  assert.deepEqual(formatCount(2_000_000_000), { value: "2.00", unit: "B" });
  assert.deepEqual(formatCount(-5), { value: "0.00", unit: "" });
});

test("formatMoney 边界（$/K$/M$/B$）", () => {
  assert.deepEqual(formatMoney(0), { value: "0.00", unit: "$" });
  assert.deepEqual(formatMoney(0.0012), { value: "0.00", unit: "$" });
  assert.deepEqual(formatMoney(1.25), { value: "1.25", unit: "$" });
  assert.deepEqual(formatMoney(3.375), { value: "3.38", unit: "$" });
  assert.deepEqual(formatMoney(1234), { value: "1.23", unit: "K$" });
  assert.deepEqual(formatMoney(1_500_000), { value: "1.50", unit: "M$" });
  assert.deepEqual(formatMoney(-1), { value: "0.00", unit: "$" });
});

test("formatTime 边界（ms/s/m/h/d）", () => {
  assert.deepEqual(formatTime(0), { value: "0.00", unit: "ms" });
  assert.deepEqual(formatTime(500), { value: "500.00", unit: "ms" });
  assert.deepEqual(formatTime(999), { value: "999.00", unit: "ms" });
  assert.deepEqual(formatTime(1500), { value: "1.50", unit: "s" });
  assert.deepEqual(formatTime(65_000), { value: "1.08", unit: "m" });
  assert.deepEqual(formatTime(3_600_000), { value: "1.00", unit: "h" });
  assert.deepEqual(formatTime(90_000_000), { value: "1.04", unit: "d" });
});
