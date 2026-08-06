import { ref } from "vue";
import {
  extractInvokeError,
  fetchProviderModels,
  getProviderModels,
} from "../api/tauri";

export type ProviderModelsStatus = "idle" | "loading" | "error" | "ready";

/**
 * 按供应商缓存模型列表。
 * - `ensure`：优先读本地持久化模型（`get_provider_models`，离线可用）；本地为空时回退实时拉取兜底。
 * - `refresh`：强制实时拉取上游到内存缓存（不写本地持久化；持久化由供应商页「立即同步」或后台自动同步完成）。
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

  /**
   * 获取模型列表：force=false 先读本地持久化（非空即用），为空再实时拉取；
   * force=true 跳过本地直连上游实时拉取（供「重试/刷新」按钮）。
   */
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

    const request = (
      force
        ? fetchProviderModels({ provider_id: providerId })
        : getProviderModels(providerId).then((local) => {
            // 本地持久化命中（离线可用）直接使用；为空则回退实时拉取兜底。
            if (local.length > 0) {
              return local;
            }
            return fetchProviderModels({ provider_id: providerId });
          })
    )
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

  /** 强制实时拉取上游模型到内存缓存（不写本地持久化）。 */
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
