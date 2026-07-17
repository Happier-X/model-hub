import { gatewayHttp } from "./gatewayHttp";

export const OPENAI_CHAT_TYPE = "openai/chat_completions";

export interface ChannelKey {
  id?: number;
  channel_id?: number;
  enabled: boolean;
  channel_key: string;
  remark?: string;
}

export interface BaseUrl {
  url: string;
  delay: number;
}

export interface Channel {
  id: number;
  name: string;
  type: string;
  enabled: boolean;
  base_urls: BaseUrl[] | null;
  keys: ChannelKey[] | null;
  model: string;
  custom_model?: string;
}

export interface CreateChannelInput {
  name: string;
  baseUrl: string;
  apiKey: string;
  model: string;
}

export async function listChannels(): Promise<Channel[]> {
  const data = await gatewayHttp.get<Channel[] | null>("/api/v1/channel/list");
  return (data ?? []).map((item) => ({
    ...item,
    base_urls: item.base_urls ?? [],
    keys: item.keys ?? [],
  }));
}

export async function createOpenAiChatChannel(
  input: CreateChannelInput,
): Promise<unknown> {
  return gatewayHttp.post("/api/v1/channel/create", {
    name: input.name,
    type: OPENAI_CHAT_TYPE,
    enabled: true,
    base_urls: [{ url: input.baseUrl.replace(/\/$/, ""), delay: 0 }],
    keys: [{ enabled: true, channel_key: input.apiKey }],
    model: input.model,
  });
}

export async function deleteChannel(id: number): Promise<unknown> {
  return gatewayHttp.delete(`/api/v1/channel/delete/${id}`);
}

export async function setChannelEnabled(
  id: number,
  enabled: boolean,
): Promise<unknown> {
  return gatewayHttp.post("/api/v1/channel/enable", { id, enabled });
}

export function maskSecret(value: string | undefined): string {
  if (!value) {
    return "—";
  }
  if (value.length <= 8) {
    return "••••";
  }
  return `${value.slice(0, 3)}••••${value.slice(-4)}`;
}
