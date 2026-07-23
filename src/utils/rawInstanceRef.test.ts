import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { isProxy, ref, shallowRef } from "vue";

/**
 * 模拟 Tauri Resource / Update：方法依赖 JS 私有字段。
 * 若实例被 Vue 深层代理，访问 #id 会抛 private member 错误。
 */
class ResourceLike {
  #id: number;
  constructor(id: number) {
    this.#id = id;
  }
  getId(): number {
    return this.#id;
  }
  async downloadAndInstall(): Promise<number> {
    return this.#id;
  }
}

describe("raw class instance must not be deeply reactive", () => {
  it("ref 会 Proxy 类实例，导致私有成员方法失败", async () => {
    const instance = new ResourceLike(7);
    const boxed = ref(instance);
    assert.equal(isProxy(boxed.value), true);
    await assert.rejects(
      async () => boxed.value.downloadAndInstall(),
      (err: unknown) =>
        err instanceof TypeError &&
        String(err.message).includes("private member"),
    );
  });

  it("shallowRef 保持原始实例身份，私有成员方法可用", async () => {
    const instance = new ResourceLike(11);
    const boxed = shallowRef(instance);
    assert.equal(boxed.value, instance);
    assert.equal(isProxy(boxed.value), false);
    assert.equal(await boxed.value.downloadAndInstall(), 11);
  });
});
