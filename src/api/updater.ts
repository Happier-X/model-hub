import { getVersion } from "@tauri-apps/api/app";
import { relaunch } from "@tauri-apps/plugin-process";
import {
  check,
  type DownloadEvent,
  type Update,
} from "@tauri-apps/plugin-updater";

export interface UpdateInfo {
  currentVersion: string;
  version: string;
  date?: string;
  body?: string;
  update: Update;
}

export interface DownloadProgress {
  downloadedBytes: number;
  totalBytes?: number;
  finished: boolean;
}

function actionableUpdaterError(error: unknown): Error {
  const detail = error instanceof Error ? error.message : String(error);
  return new Error(
    `应用更新失败：${detail}。请检查网络后重试；若持续失败，请从 GitHub Releases 手动下载安装包。`,
  );
}

export async function getCurrentVersion(): Promise<string> {
  try {
    return await getVersion();
  } catch {
    return "未知";
  }
}

export async function checkForUpdate(): Promise<UpdateInfo | null> {
  try {
    const update = await check({ timeout: 30_000 });
    if (!update) return null;
    return {
      currentVersion: update.currentVersion,
      version: update.version,
      date: update.date,
      body: update.body,
      update,
    };
  } catch (error) {
    throw actionableUpdaterError(error);
  }
}

export async function downloadAndInstallUpdate(
  update: Update,
  onProgress: (progress: DownloadProgress) => void,
): Promise<void> {
  let downloadedBytes = 0;
  let totalBytes: number | undefined;
  const report = (event: DownloadEvent) => {
    if (event.event === "Started") {
      totalBytes = event.data.contentLength;
      downloadedBytes = 0;
    } else if (event.event === "Progress") {
      downloadedBytes += event.data.chunkLength;
    }
    onProgress({
      downloadedBytes,
      totalBytes,
      finished: event.event === "Finished",
    });
  };

  try {
    await update.downloadAndInstall(report, { timeout: 10 * 60_000 });
  } catch (error) {
    throw actionableUpdaterError(error);
  }
}

export async function relaunchAfterUpdate(): Promise<void> {
  await relaunch();
}
