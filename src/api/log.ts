import { gatewayHttp } from "./gatewayHttp";

export interface RelayLog {
  id: number;
  time: number;
  request_model_name: string;
  channel_name: string;
  actual_model_name: string;
  input_tokens: number;
  output_tokens: number;
  use_time: number;
  cost: number;
  error: string;
}

export async function listLogs(page = 1, pageSize = 30): Promise<RelayLog[]> {
  const query = new URLSearchParams({
    page: String(page),
    page_size: String(pageSize),
  });
  const data = await gatewayHttp.get<RelayLog[] | null>(
    `/api/v1/log/list?${query.toString()}`,
  );
  return data ?? [];
}

export async function clearLogs(): Promise<unknown> {
  return gatewayHttp.delete("/api/v1/log/clear");
}

export function formatLogTime(unixSecondsOrMs: number): string {
  const ms = unixSecondsOrMs > 1e12 ? unixSecondsOrMs : unixSecondsOrMs * 1000;
  try {
    return new Date(ms).toLocaleString();
  } catch {
    return String(unixSecondsOrMs);
  }
}
