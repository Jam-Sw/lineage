// API client wrappers: command names, argument shapes, and error mapping, with
// the Tauri invoke boundary mocked.
import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}));
// client.ts re-exports listen from the event module; stub it so importing the
// client does not require a Tauri runtime.
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

import {
  ApiError,
  authStatus,
  connectViaPat,
  resolveClose,
  saveTreeImage,
  setSettings,
} from "./client";
import type { AppSettings } from "./types";

beforeEach(() => {
  invoke.mockReset();
});

describe("api client wrappers", () => {
  it("authStatus calls auth_status with no args", async () => {
    invoke.mockResolvedValue({ connected: false, source: null, login: null });
    await authStatus();
    expect(invoke).toHaveBeenCalledWith("auth_status", undefined);
  });

  it("connectViaPat passes the token", async () => {
    invoke.mockResolvedValue({ connected: true, source: "pat", login: "me" });
    await connectViaPat("ghp_x");
    expect(invoke).toHaveBeenCalledWith("connect_via_pat", { token: "ghp_x" });
  });

  it("setSettings passes the settings object", async () => {
    const settings = { excludeGenerated: true } as unknown as AppSettings;
    invoke.mockResolvedValue(settings);
    await setSettings(settings);
    expect(invoke).toHaveBeenCalledWith("set_settings", { settings });
  });

  it("resolveClose passes the chosen behavior", async () => {
    invoke.mockResolvedValue(undefined);
    await resolveClose("menuBar");
    expect(invoke).toHaveBeenCalledWith("resolve_close", { behavior: "menuBar" });
  });

  it("saveTreeImage passes the image and login", async () => {
    invoke.mockResolvedValue("/path/tree.png");
    await saveTreeImage("AAAA", "octocat");
    expect(invoke).toHaveBeenCalledWith("save_tree_image", {
      dataB64: "AAAA",
      login: "octocat",
    });
  });

  it("maps structured backend errors to ApiError", async () => {
    invoke.mockRejectedValue({ code: "AUTH", message: "token rejected" });
    const err = await authStatus().catch((e) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect(err.code).toBe("AUTH");
    expect(err.message).toBe("token rejected");
  });

  it("maps unstructured failures to an UNKNOWN ApiError", async () => {
    invoke.mockRejectedValue("boom");
    const err = await authStatus().catch((e) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect(err.code).toBe("UNKNOWN");
    expect(err.message).toBe("boom");
  });
});
