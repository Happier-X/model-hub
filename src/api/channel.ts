import { gatewayHttp } from "./gatewayHttp";

/**
 * octopus v0.9.28 Windows 二进制的 channel.type 为数字枚举（非当前 dev 源码的字符串）。
 * 实测：`type: 0` 可创建成功；字符串（如 openai/chat_completions）会返回 Invalid JSON format。
 */
export const CHANNEL_TYPE_OPENAI_CHAT = 0;

export const CHANNEL_TYPE_LABELS: Record<number, string> = {
  0: "OpenAI Chat",
  1: "类型 1",
  2: "类型 2",
  3: "类型 3",
  4: "类型 4",
  5: "类型 5",
  6: "类型 6",
  7: "类型 7",
};

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
  type: number | string;
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

export interface UpdateChannelInput {
  id: number;
  name: string;
  baseUrl: string;
  model: string;
  /** 非空时轮换首条 Key；空字符串/未填则不改 Key */
  apiKey?: string;
  /** 首条 Key 的 id；有新 Key 时用于 keys_to_update */
  primaryKeyId?: number;
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
    type: CHANNEL_TYPE_OPENAI_CHAT,
    enabled: true,
    base_urls: [{ url: input.baseUrl.replace(/\/$/, ""), delay: 0 }],
    keys: [{ enabled: true, channel_key: input.apiKey, remark: "" }],
    model: input.model,
    custom_model: "",
    proxy: false,
    auto_sync: false,
    auto_group: 0,
    custom_header: [],
  });
}

/**
 * v0.9.28 真机支持部分更新：`{id, name, base_urls, model}`；
 * Key 轮换用 `keys_to_update: [{id, channel_key}]`（有 key id 时）或 `keys_to_add`。
 */
export async function updateOpenAiChatChannel(
  input: UpdateChannelInput,
): Promise<unknown> {
  const payload: Record<string, unknown> = {
    id: input.id,
    name: input.name,
    type: CHANNEL_TYPE_OPENAI_CHAT,
    base_urls: [{ url: input.baseUrl.replace(/\/$/, ""), delay: 0 }],
    model: input.model,
  };

  const nextKey = input.apiKey?.trim();
  if (nextKey) {
    if (input.primaryKeyId != null && input.primaryKeyId > 0) {
      payload.keys_to_update = [
        { id: input.primaryKeyId, channel_key: nextKey },
      ];
    } else {
      payload.keys_to_add = [
        { enabled: true, channel_key: nextKey, remark: "" },
      ];
    }
  }

  return gatewayHttp.post("/api/v1/channel/update", payload);
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

export function channelTypeLabel(type: number | string): string {
  if (typeof type === "number") {
    return CHANNEL_TYPE_LABELS[type] ?? `类型 ${type}`;
  }
  return type;
}

export function primaryChannelKey(channel: Channel): ChannelKey | undefined {
  return channel.keys?.[0];
}
