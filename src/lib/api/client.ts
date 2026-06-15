// The only caller of Tauri `invoke`. Every command is a typed wrapper; errors
// become ApiError with the core's stable codes.

import { invoke } from "@tauri-apps/api/core";
import type {
  AppSettings,
  AuthStatus,
  Snapshot,
  SyncStatus,
} from "./types";

export class ApiError extends Error {
  code: string;
  constructor(code: string, message: string) {
    super(message);
    this.name = "ApiError";
    this.code = code;
  }
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    if (e && typeof e === "object" && "code" in e && "message" in e) {
      throw new ApiError(String(e.code), String(e.message));
    }
    throw new ApiError("UNKNOWN", String(e));
  }
}

// data
export const authStatus = () => call<AuthStatus>("auth_status");
export const getSnapshot = () => call<Snapshot | null>("get_snapshot");
export const getSyncStatus = () => call<SyncStatus>("get_sync_status");
export const getSettings = () => call<AppSettings>("get_settings");
export const setSettings = (settings: AppSettings) =>
  call<AppSettings>("set_settings", { settings });

// auth
export const ghAvailable = () => call<boolean>("gh_available");
export const connectViaGh = () => call<AuthStatus>("connect_via_gh");
export const connectViaPat = (token: string) =>
  call<AuthStatus>("connect_via_pat", { token });
export const disconnect = () => call<AuthStatus>("disconnect");

// actions
export const syncNow = () => call<void>("sync_now");
export const openUrl = (url: string) => call<void>("open_url", { url });
export const openDashboard = () => call<void>("open_dashboard");
export const openOnboarding = () => call<void>("open_onboarding");
export const openDataFolder = () => call<void>("open_data_folder");
export const cacheInfo = () => call<string>("cache_info");
export const clearCache = () => call<void>("clear_cache");

export { listen } from "@tauri-apps/api/event";
