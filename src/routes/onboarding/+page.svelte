<script lang="ts">
  import { onMount } from "svelte";
  import * as api from "$lib/api/client";

  let ghAvailable = $state(false);
  let pat = $state("");
  let busy = $state(false);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      ghAvailable = await api.ghAvailable();
    } catch {
      ghAvailable = false;
    }
  });

  async function connectGh() {
    busy = true;
    error = null;
    try {
      await api.connectViaGh();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  async function connectPat() {
    if (!pat.trim()) return;
    busy = true;
    error = null;
    try {
      await api.connectViaPat(pat.trim());
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }
</script>

<main>
  <h1>Master Diff</h1>
  <p class="dim">
    Connect GitHub to compute your lifetime master diff - every line you have added and
    removed, by language, across all your repositories.
  </p>

  <div class="methods">
    <button class="primary big" disabled title="Coming soon - requires the OAuth app">
      Sign in with GitHub
      <span class="soon">soon</span>
    </button>

    {#if ghAvailable}
      <button class="big" onclick={connectGh} disabled={busy}>Use my GitHub CLI login</button>
    {/if}

    <div class="pat">
      <label class="dim" for="pat">…or paste a personal access token (needs <code>repo</code> scope)</label>
      <input id="pat" type="password" bind:value={pat} placeholder="ghp_…" disabled={busy} />
      <button onclick={connectPat} disabled={busy || !pat.trim()}>Connect with token</button>
    </div>
  </div>

  {#if busy}<p class="dim">Connecting…</p>{/if}
  {#if error}<p class="remove">{error}</p>{/if}

  <p class="foot dim">
    Your token is stored in the macOS Keychain. All GitHub access and cloning happens locally.
  </p>
</main>

<style>
  main {
    padding: 28px 30px;
    max-width: 460px;
    margin: 0 auto;
  }
  h1 {
    margin: 0 0 6px;
  }
  .methods {
    margin-top: 22px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .big {
    width: 100%;
    padding: 12px;
    font-size: 15px;
    position: relative;
  }
  .soon {
    position: absolute;
    right: 12px;
    top: 50%;
    transform: translateY(-50%);
    font-size: 10px;
    text-transform: uppercase;
    opacity: 0.7;
  }
  .pat {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-top: 6px;
    border-top: 1px solid var(--border);
  }
  .pat label {
    font-size: 12px;
  }
  code {
    font-family: var(--mono);
    background: var(--bg-elev-2);
    padding: 1px 5px;
    border-radius: 4px;
  }
  .foot {
    margin-top: 26px;
    font-size: 12px;
  }
</style>
