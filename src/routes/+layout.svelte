<script lang="ts">
  import "$lib/app.css";
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { listen } from "@tauri-apps/api/event";
  import * as api from "$lib/api/client";

  let { children } = $props();

  // The first time the window is closed, the Rust shell holds it open and asks
  // here whether to idle to the menu bar or actually quit. The choice is saved,
  // so this only ever shows once (unless reset from Settings).
  let askClose = $state(false);

  // The tray Help menu asks the app to navigate (e.g. to the uninstall section).
  // Handled here in the root layout so it works from whatever route is showing.
  onMount(() => {
    const stops = [
      listen<string>("nav", (e) => void goto(e.payload)),
      listen("close:prompt", () => (askClose = true)),
    ];
    return () => stops.forEach((s) => void s.then((f) => f()));
  });

  async function choose(behavior: "menuBar" | "quit") {
    askClose = false; // dismiss before the window hides (menuBar) or app exits (quit)
    await api.resolveClose(behavior);
  }
</script>

{@render children()}

{#if askClose}
  <!-- One-time close prompt. Cancel (Esc / backdrop) keeps the window open and
       leaves the decision for next time. -->
  <div
    class="close-backdrop"
    role="presentation"
    onclick={() => (askClose = false)}
    onkeydown={(e) => e.key === "Escape" && (askClose = false)}
  >
    <div
      class="close-modal"
      role="dialog"
      aria-modal="true"
      aria-labelledby="close-title"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={() => {}}
    >
      <h2 id="close-title">Before you close…</h2>
      <p>
        Lineage can keep running quietly in the menu bar so your Lineage stays a click
        away - or it can fully quit, like the few Mac apps that actually close when you
        close them.
      </p>
      <p class="dim small">You can change this any time in Settings.</p>
      <div class="close-actions">
        <button onclick={() => choose("quit")}>Quit completely</button>
        <button class="primary" onclick={() => choose("menuBar")}>Keep in menu bar</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .close-backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: flex;
    align-items: center;
    justify-content: center;
    background: color-mix(in srgb, var(--bg) 55%, transparent);
    backdrop-filter: blur(3px);
  }
  .close-modal {
    width: min(380px, calc(100vw - 48px));
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 22px 22px 18px;
    box-shadow: 0 18px 50px rgba(0, 0, 0, 0.45);
    text-align: center;
  }
  .close-modal h2 {
    margin: 0 0 10px;
    font-size: 17px;
  }
  .close-modal p {
    margin: 0 0 8px;
    font-size: 13px;
    line-height: 1.5;
  }
  .close-modal .small {
    font-size: 12px;
  }
  .close-actions {
    margin-top: 18px;
    display: flex;
    gap: 10px;
    justify-content: center;
  }
</style>
