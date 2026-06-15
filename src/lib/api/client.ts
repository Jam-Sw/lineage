// The only caller of Tauri `invoke`. Every command is a typed wrapper; errors
// become ApiError with the core's stable codes.

import { invoke } from "@tauri-apps/api/core";
import type {
  AppSettings,
  AppearanceSettings,
  AuthStatus,
  ProfileStats,
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
export const getProfile = () => call<ProfileStats | null>("get_profile");
export const refreshProfile = () => call<void>("refresh_profile");
export const getSettings = () => call<AppSettings>("get_settings");
export const setSettings = (settings: AppSettings) =>
  call<AppSettings>("set_settings", { settings });
export const getAppearance = () => call<AppearanceSettings>("get_appearance");
export const setAppearance = (appearance: AppearanceSettings) =>
  call<AppearanceSettings>("set_appearance", { appearance });

// auth
export const ghAvailable = () => call<boolean>("gh_available");
export const connectViaGh = () => call<AuthStatus>("connect_via_gh");
export const connectViaPat = (token: string) =>
  call<AuthStatus>("connect_via_pat", { token });
export type OauthStart = { userCode: string; verificationUri: string; expiresIn: number };
// Starts the device flow and returns the code to show; the backend polls and
// finishes the connect, or emits an "oauth:error" event.
export const connectViaOauth = () => call<OauthStart>("connect_via_oauth");
export const disconnect = () => call<AuthStatus>("disconnect");

// actions. Returns false when a sync was already running.
export const syncNow = () => call<boolean>("sync_now");
export const openUrl = (url: string) => call<void>("open_url", { url });
export const openDashboard = () => call<void>("open_dashboard");
export const openOnboarding = () => call<void>("open_onboarding");
export const openDataFolder = () => call<void>("open_data_folder");
// Answer the first-close prompt: "menuBar" idles to the tray, "quit" terminates.
export const resolveClose = (behavior: "menuBar" | "quit") =>
  call<void>("resolve_close", { behavior });
export const cacheInfo = () => call<string>("cache_info");
export const clearCache = () => call<void>("clear_cache");
// Cleanly removes all local data, the Keychain token, and the app bundle, then quits.
export const uninstallApp = () => call<void>("uninstall_app");
export const saveTreeImage = (dataB64: string, login: string) =>
  call<string>("save_tree_image", { dataB64, login });

export { listen } from "@tauri-apps/api/event";
