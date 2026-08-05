import { ref } from "vue";
import { extractInvokeError, fetchProviderModels } from "../api/tauri";

export type ProviderModelsStatus = "idle" | "loading" | "error" | "ready";

/**
 * 按供应商缓存上游模型列表。
 * 仅 ensure / refresh（用户展开手风琴或点刷新）时请求，禁止打开对话框预拉。
 */
export function useProviderModelCache() {
  const cache = ref<Record<number, string[]>>({});
  const status = ref<Record<number, ProviderModelsStatus>>({});
  const errors = ref<Record<number, string>>({});
  const inflight = new Map<number, Promise<string[]>>();

  function getStatus(providerId: number): ProviderModelsStatus {
    return status.value[providerId] ?? "idle";
  }

  function getModels(providerId: number): string[] {
    return cache.value[providerId] ?? [];
  }

  function getError(providerId: number): string {
    return errors.value[providerId] ?? "";
  }

  async function ensure(providerId: number, force = false): Promise<string[]> {
    if (!providerId) return [];

    if (!force && getStatus(providerId) === "ready") {
      return getModels(providerId);
    }

    const existing = inflight.get(providerId);
    if (!force && existing) {
      return existing;
    }

    status.value = { ...status.value, [providerId]: "loading" };
    errors.value = { ...errors.value, [providerId]: "" };

    const request = fetchProviderModels({ provider_id: providerId })
      .then((ids) => {
        cache.value = { ...cache.value, [providerId]: ids };
        status.value = { ...status.value, [providerId]: "ready" };
        return ids;
      })
      .catch((e) => {
        const msg = extractInvokeError(e);
        status.value = { ...status.value, [providerId]: "error" };
        errors.value = { ...errors.value, [providerId]: msg };
        throw e;
      })
      .finally(() => {
        inflight.delete(providerId);
      });

    inflight.set(providerId, request);
    return request;
  }

  function refresh(providerId: number): Promise<string[]> {
    return ensure(providerId, true);
  }

  return {
    cache,
    status,
    errors,
    getStatus,
    getModels,
    getError,
    ensure,
    refresh,
  };
}
