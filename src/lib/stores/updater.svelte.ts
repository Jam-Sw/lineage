// Self-update state (Svelte 5 runes). Checks GitHub Releases via the Tauri
// updater plugin; the UI surfaces this in Settings only when an update actually
// exists, so the happy path stays invisible.

import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";

export type UpdateStatus =
  | "idle"
  | "checking"
  | "available"
  | "downloading"
  | "ready"
  | "error";

const RECHECK_INTERVAL_MS = 6 * 60 * 60 * 1000;

class UpdaterStore {
  status = $state<UpdateStatus>("idle");
  version = $state<string | null>(null);
  // 0..1 while downloading, null when total size is unknown.
  progress = $state<number | null>(null);
  error = $state<string | null>(null);

  #update: Update | null = null;
  #timer: ReturnType<typeof setInterval> | null = null;

  /** Check once now, then every RECHECK_INTERVAL_MS while running. */
  start() {
    if (this.#timer) return;
    void this.checkNow();
    this.#timer = setInterval(() => void this.checkNow(), RECHECK_INTERVAL_MS);
  }

  stop() {
    if (this.#timer) {
      clearInterval(this.#timer);
      this.#timer = null;
    }
  }

  async checkNow() {
    // Never interrupt a download or a pending restart.
    if (this.status === "downloading" || this.status === "ready") return;
    this.status = "checking";
    try {
      const update = await check();
      if (update) {
        this.#update = update;
        this.version = update.version;
        this.status = "available";
      } else {
        this.status = "idle";
      }
    } catch {
      // Offline, private repo, or no manifest yet: stay silent. Updates are a
      // suggestion, never an obstruction.
      this.status = "idle";
    }
  }

  async downloadAndInstall() {
    const update = this.#update;
    if (!update || this.status === "downloading") return;
    this.status = "downloading";
    this.progress = null;
    this.error = null;
    let total: number | null = null;
    let received = 0;
    try {
      await update.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            total = event.data.contentLength ?? null;
            break;
          case "Progress":
            received += event.data.chunkLength;
            if (total) this.progress = Math.min(received / total, 1);
            break;
        }
      });
      this.status = "ready";
    } catch (e) {
      this.status = "error";
      this.error = e instanceof Error ? e.message : String(e);
    }
  }

  async restart() {
    if (this.status !== "ready") return;
    await relaunch();
  }
}

export const updater = new UpdaterStore();
