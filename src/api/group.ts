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

export async function deleteGroup(id: number): Promise<unknown> {
  return gatewayHttp.delete(`/api/v1/group/delete/${id}`);
}
