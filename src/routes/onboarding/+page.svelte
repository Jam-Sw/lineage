<script lang="ts">
  import { onMount } from "svelte";
  import * as api from "$lib/api/client";
  import { isMac, keychainName } from "$lib/platform";

  let ghAvailable = $state(false);
  let pat = $state("");
  let busy = $state(false);
  let error = $state<string | null>(null);
  // The git tree connecting the title to the login wakes up the moment a
  // connection is attempted: gray lineage turns to living color.
  let alive = $state(false);
  // While the device flow is in progress, the code the user enters in the browser.
  let oauth = $state<{ userCode: string; verificationUri: string } | null>(null);

  onMount(() => {
    let unlisten: (() => void) | undefined;
    void (async () => {
      try {
        ghAvailable = await api.ghAvailable();
      } catch {
        ghAvailable = false;
      }
      unlisten = await api.listen<string>("oauth:error", (e) => {
        error = String(e.payload);
        busy = false;
        oauth = null;
        alive = false;
      });
    })();
    return () => unlisten?.();
  });

  async function connectOauth() {
    alive = true;
    busy = true;
    error = null;
    try {
      const s = await api.connectViaOauth();
      oauth = { userCode: s.userCode, verificationUri: s.verificationUri };
      await api.openUrl(s.verificationUri);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      busy = false;
      alive = false;
    }
  }

  async function connectGh() {
    alive = true;
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
    alive = true;
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
  <div class="left">
    <h1>Lineage</h1>
    <p class="dim">
      Connect GitHub to compute your lifetime Lineage - every line you have added and
      removed, by language, across all your repositories.
    </p>
    <p class="foot dim">
      Your token stays in the {keychainName}. Lineage talks only to GitHub: it downloads your
      repositories to this {isMac ? "Mac" : "computer"} and counts your lines here. Nothing is
      uploaded to a server of ours.
    </p>
  </div>

  <!-- A git branch graph bridging the name and the action. Subtle gray until a
       connection is started, then color flows from the title side outward. -->
  <div class="tree" aria-hidden="true">
    <svg viewBox="0 0 120 320" class:alive preserveAspectRatio="xMidYMid meet">
      <!-- gray base: always visible -->
      <path class="base" d="M0,160 L120,160" />
      <path class="base" d="M16,160 C36,160 40,124 60,124 C80,124 84,160 104,160" />
      <path class="base" d="M16,160 C36,160 40,196 60,196 C80,196 84,160 104,160" />
      <!-- colored flow: draws in left-to-right when alive -->
      <path class="flow lane" pathLength="1" style="--d:0s" d="M0,160 L120,160" />
      <path class="flow up" pathLength="1" style="--d:.1s" d="M16,160 C36,160 40,124 60,124 C80,124 84,160 104,160" />
      <path class="flow down" pathLength="1" style="--d:.1s" d="M16,160 C36,160 40,196 60,196 C80,196 84,160 104,160" />
      <!-- commit nodes light up as the color passes -->
      <circle class="node blue" style="--d:.15s" cx="16" cy="160" r="5" />
      <circle class="node orange" style="--d:.5s" cx="60" cy="124" r="5" />
      <circle class="node yellow" style="--d:.5s" cx="60" cy="160" r="5" />
      <circle class="node green" style="--d:.5s" cx="60" cy="196" r="5" />
      <circle class="node blue" style="--d:.85s" cx="104" cy="160" r="5" />
    </svg>
  </div>

  <div class="right">
    <div class="methods">
      <button class="primary big" onclick={connectOauth} disabled={busy}>
        Sign in with GitHub
      </button>

      {#if ghAvailable}
        <button class="big" onclick={connectGh} disabled={busy}>Use my GitHub CLI login</button>
      {/if}

      <div class="pat">
        <!-- <label class="dim" for="pat">…or use a personal access token</label> -->
        <input id="pat" type="password" bind:value={pat} placeholder="(PAT) ghp_…" disabled={busy} />
        <button onclick={connectPat} disabled={busy || !pat.trim()}>Connect with token</button>
      </div>
    </div>

    {#if oauth}
      <div class="device">
        <p>Enter this code at <b>github.com/login/device</b>:</p>
        <p class="code-big">{oauth.userCode}</p>
        <button onclick={() => oauth && api.openUrl(oauth.verificationUri)}>Open GitHub again</button>
      </div>
    {:else if busy}
      <p class="dim status">Connecting…</p>
    {/if}
    {#if error}<p class="remove status">{error}</p>{/if}
  </div>
</main>

<style>
  main {
    display: flex;
    flex-direction: row;
    gap: 24px;
    align-items: center;
    padding: 28px 36px;
    height: 100%;
    box-sizing: border-box;
  }
  .left {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .tree {
    flex: 0 0 120px;
    align-self: stretch;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .tree svg {
    width: 100%;
    height: 100%;
  }
  .right {
    flex: 0 0 250px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  h1 {
    margin: 0;
  }
  .methods {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .big {
    width: 100%;
    padding: 11px;
    font-size: 15px;
    position: relative;
  }
  .pat {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-top: 6px;
    border-top: 1px solid var(--border);
  }
  .foot {
    font-size: 12px;
    margin: 0;
  }
  .status {
    margin: 0;
    font-size: 13px;
  }
  .device {
    margin-top: 4px;
    padding-top: 10px;
    border-top: 1px solid var(--border);
  }
  .device p {
    margin: 0 0 6px;
    font-size: 13px;
  }
  .code-big {
    font-family: var(--mono);
    font-size: 22px;
    letter-spacing: 3px;
    font-weight: 600;
  }

  /* --- git tree --- */
  .tree svg {
    --tree-gray: #3c4350;
    opacity: 0.9;
    transition: opacity 0.6s ease;
  }
  .tree svg.alive {
    opacity: 1;
  }
  .base {
    fill: none;
    stroke: var(--tree-gray);
    stroke-width: 2.5;
    stroke-linecap: round;
  }
  .flow {
    fill: none;
    stroke-width: 2.5;
    stroke-linecap: round;
    stroke-dasharray: 1;
    stroke-dashoffset: 1;
    transition: stroke-dashoffset 0.85s ease var(--d, 0s);
  }
  .flow.lane {
    stroke: var(--accent);
  }
  .flow.up {
    stroke: #ff3e00;
  }
  .flow.down {
    stroke: var(--add);
  }
  svg.alive .flow {
    stroke-dashoffset: 0;
  }
  .node {
    fill: var(--tree-gray);
    transform-box: fill-box;
    transform-origin: center;
    transition: fill 0.5s ease var(--d, 0s);
  }
  .node.blue {
    --nc: var(--accent);
  }
  .node.orange {
    --nc: #ff3e00;
  }
  .node.yellow {
    --nc: #f1e05a;
  }
  .node.green {
    --nc: var(--add);
  }
  svg.alive .node {
    fill: var(--nc);
    animation: pop 0.55s ease var(--d, 0s);
  }
  @keyframes pop {
    0% {
      transform: scale(1);
    }
    40% {
      transform: scale(1.45);
    }
    100% {
      transform: scale(1);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .flow {
      transition: none;
      stroke-dashoffset: 0;
    }
    svg.alive .node {
      animation: none;
    }
  }
</style>
