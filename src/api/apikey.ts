import { gatewayHttp } from "./gatewayHttp";
import { maskSecret } from "./channel";

export interface ApiKey {
  id: number;
  name: string;
  api_key: string;
  enabled: boolean;
  expire_at?: string | number | null;
  max_cost?: number | null;
  supported_models?: string[] | null;
}

export interface CreateApiKeyInput {
  name: string;
  enabled?: boolean;
}

export interface UpdateApiKeyInput {
  id: number;
  name?: string;
  enabled?: boolean;
  expire_at?: string | number | null;
  max_cost?: number | null;
  supported_models?: string[] | null;
}

export async function listApiKeys(): Promise<ApiKey[]> {
  const data = await gatewayHttp.get<ApiKey[] | null>("/api/v1/apikey/list");
  return data ?? [];
}

export async function createApiKey(input: CreateApiKeyInput): Promise<ApiKey> {
  return gatewayHttp.post<ApiKey>("/api/v1/apikey/create", {
    name: input.name,
    enabled: input.enabled ?? true,
  });
}

export async function updateApiKey(input: UpdateApiKeyInput): Promise<unknown> {
  return gatewayHttp.post("/api/v1/apikey/update", input);
}

export async function deleteApiKey(id: number): Promise<unknown> {
  return gatewayHttp.delete(`/api/v1/apikey/delete/${id}`);
}

export { maskSecret };
