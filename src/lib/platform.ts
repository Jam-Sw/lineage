// Platform-specific UI wording. Detection feeds labels and copy only; behavior
// is platform-gated on the Rust side, so a wrong guess can never break anything.
import { browser } from "$app/environment";

/** True on macOS. Defaults to true when there is no navigator (prerender);
 *  the value is re-evaluated in the real webview at hydration. */
export const isMac = browser ? navigator.userAgent.includes("Mac") : true;

/** Where the app idles: the macOS menu bar or the system tray elsewhere. */
export const trayName = isMac ? "menu bar" : "system tray";

/** Where the GitHub token is stored, in words the platform's user knows. */
export const keychainName = isMac ? "macOS Keychain" : "system credential store";
