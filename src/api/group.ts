import { gatewayHttp } from "./gatewayHttp";

/** 与上游 GroupMode.RoundRobin 对齐 */
export const GROUP_MODE_ROUND_ROBIN = 1;

export interface GroupItem {
  id?: number;
  group_id?: number;
  channel_id: number;
  model_name: string;
  priority: number;
  weight: number;
}

export interface Group {
  id?: number;
  name: string;
  mode: number;
  match_regex: string;
  items?: GroupItem[];
}

export interface CreateGroupInput {
  name: string;
  channelId: number;
  modelName: string;
}

export interface UpdateGroupInput {
  id: number;
  name: string;
  channelId: number;
  modelName: string;
  /** 当前首条 item；用于判断是否需要 delete+add 换绑 */
  primaryItem?: GroupItem | null;
}

export async function listGroups(): Promise<Group[]> {
  const data = await gatewayHttp.get<Group[] | null>("/api/v1/group/list");
  return data ?? [];
}

export async function createRoundRobinGroup(
  input: CreateGroupInput,
): Promise<unknown> {
  const payload: Group = {
    name: input.name,
    mode: GROUP_MODE_ROUND_ROBIN,
    match_regex: "",
    items: [
      {
        channel_id: input.channelId,
        model_name: input.modelName,
        priority: 1,
        weight: 1,
      },
    ],
  };
  return gatewayHttp.post("/api/v1/group/create", payload);
}

/**
 * v0.9.28：改名用 `{id,name}`；换绑渠道/改 model_name 用
 * `items_to_delete` + `items_to_add`（可与 name 同请求）。
 */
export async function updateRoundRobinGroup(
  input: UpdateGroupInput,
): Promise<unknown> {
  const payload: Record<string, unknown> = {
    id: input.id,
    name: input.name,
  };

  const primary = input.primaryItem;
  const needRebind =
    !primary ||
    primary.channel_id !== input.channelId ||
    primary.model_name !== input.modelName;

  if (needRebind) {
    if (primary?.id != null) {
      payload.items_to_delete = [primary.id];
    }
    payload.items_to_add = [
      {
        channel_id: input.channelId,
        model_name: input.modelName,
        priority: 1,
        weight: 1,
      },
    ];
  }

  return gatewayHttp.post("/api/v1/group/update", payload);
}

export async function deleteGroup(id: number): Promise<unknown> {
  return gatewayHttp.delete(`/api/v1/group/delete/${id}`);
}

export function groupModeLabel(mode: number): string {
  switch (mode) {
    case 1:
      return "轮询";
    case 2:
      return "随机";
    case 3:
      return "故障转移";
    case 4:
      return "加权";
    default:
      return `模式 ${mode}`;
  }
}
