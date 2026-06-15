<script lang="ts">
  import { onMount } from "svelte";
  import * as api from "$lib/api/client";
  import type {
    Snapshot,
    SyncStatus,
    AuthStatus,
    SyncTick,
    SyncPhase,
    RepoStat,
    AppearanceSettings,
    ProfileStats,
  } from "$lib/api/types";
  import { commas, signed, relativeTime } from "$lib/format";
  import LiveReveal from "$lib/components/LiveReveal.svelte";
  import LanguageBars from "$lib/components/LanguageBars.svelte";
  import Treemap from "$lib/components/Treemap.svelte";
  import ImpactTree from "$lib/components/ImpactTree.svelte";

  type FeedItem = { repo: string; added: number; removed: number; lang: string | null; cached: boolean };

  let auth = $state<AuthStatus>({ connected: false, source: null, login: null });
  let snapshot = $state<Snapshot | null>(null);
  let profile = $state<ProfileStats | null>(null);
  let syncStatus = $state<SyncStatus | null>(null);
  let tick = $state<SyncTick | null>(null);
  let feed = $state<FeedItem[]>([]);
  let syncingLive = $state(false);
  let syncMessage = $state("Starting sync…");
  let error = $state<string | null>(null);
  let appearance = $state<AppearanceSettings>({
    trayIcon: "plusMinus",
    trayShowNumber: true,
    trayMetric: "net",
    barStyle: "language",
  });

  let sortKey = $state<"net" | "added" | "removed" | "name">("net");
  let sortDir = $state<1 | -1>(-1);

  // Swipe pager (dashboard <-> impact tree).
  let pagerEl = $state<HTMLDivElement | null>(null);
  let page = $state(0);

  const phase = $derived(
    !auth.connected
      ? "connect"
      : !snapshot && syncingLive
        ? "live"
        : snapshot
          ? "dashboard"
          : "firstrun",
  );

  const sortedRepos = $derived.by<RepoStat[]>(() => {
    if (!snapshot) return [];
    const arr = [...snapshot.repos];
    arr.sort((a, b) => {
      if (sortKey === "name") return a.fullName.localeCompare(b.fullName) * sortDir;
      return (a[sortKey] - b[sortKey]) * sortDir;
    });
    return arr;
  });

  onMount(() => {
    void load();
    api.listen<SyncTick>("sync:tick", (e) => {
      tick = e.payload;
      syncingLive = true;
      if (e.payload.repo) {
        feed = [
          {
            repo: e.payload.repo,
            added: e.payload.repoAdded,
            removed: e.payload.repoRemoved,
            lang: e.payload.repoTopLanguage,
            cached: e.payload.fromCache,
          },
          ...feed,
        ].slice(0, 16);
      }
    });
    api.listen<SyncPhase>("sync:phase", (e) => {
      syncMessage = e.payload.message;
      syncingLive = true;
    });
    api.listen<Snapshot>("sync:done", (e) => {
      snapshot = e.payload;
      syncingLive = false;
      tick = null;
      feed = [];
      void refreshSync();
    });
    api.listen<{ code: string; message: string }>("sync:error", (e) => {
      error = e.payload.message;
      syncingLive = false;
      tick = null;
      void refreshSync();
    });
    api.listen<AuthStatus>("auth:changed", (e) => {
      auth = e.payload;
      void load();
    });
    api.listen<AppearanceSettings>("appearance:changed", (e) => {
      appearance = e.payload;
    });
    api.listen<ProfileStats>("profile:done", (e) => {
      profile = e.payload;
    });
  });

  async function load() {
    auth = await api.authStatus();
    appearance = await api.getAppearance();
    snapshot = await api.getSnapshot();
    profile = await api.getProfile();
    // No cached contributions graph yet (and not mid-sync): fetch it on demand.
    if (!profile && auth.connected) void api.refreshProfile();
    await refreshSync();
    if (syncStatus?.syncing) syncingLive = true;
  }

  function goPage(p: number) {
    if (!pagerEl) return;
    page = Math.max(0, Math.min(1, p));
    pagerEl.scrollTo({ left: page * pagerEl.clientWidth, behavior: "smooth" });
  }

  function onPagerScroll() {
    if (!pagerEl) return;
    page = Math.round(pagerEl.scrollLeft / pagerEl.clientWidth);
  }

  function onKey(e: KeyboardEvent) {
    if (phase !== "dashboard") return;
    if (e.key === "ArrowRight") goPage(page + 1);
    else if (e.key === "ArrowLeft") goPage(page - 1);
  }

  async function refreshSync() {
    syncStatus = await api.getSyncStatus();
  }

  async function doSync() {
    error = null;
    feed = [];
    tick = null;
    syncMessage = "Starting sync…";
    syncingLive = true; // immediate feedback, before the IPC round-trip resolves
    try {
      await api.syncNow();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      syncingLive = false;
    }
    await refreshSync();
  }

  function setSort(k: typeof sortKey) {
    if (sortKey === k) sortDir = (sortDir * -1) as 1 | -1;
    else {
      sortKey = k;
      sortDir = -1;
    }
  }
</script>

<svelte:window onkeydown={onKey} />

{#if phase === "dashboard" && snapshot}
  <div class="pager" bind:this={pagerEl} onscroll={onPagerScroll}>
    <div class="page page-scroll">
      <div class="col">
        <header>
      <div class="headline">
        <div class="net mono {snapshot.summary.net >= 0 ? 'add' : 'remove'}">
          {signed(snapshot.summary.net)}
        </div>
        <div class="sub">
          <span class="add">+{commas(snapshot.summary.added)}</span>
          <span class="remove">−{commas(snapshot.summary.removed)}</span>
          <span class="dim"
            >· {commas(snapshot.summary.commits)} commits · {snapshot.summary.repoCount} repos · {snapshot
              .summary.languageCount} languages</span
          >
          {#if snapshot.languages[0]}
            <span class="fav">★ {snapshot.languages[0].language}</span>
          {/if}
          {#if snapshot.filtered}<span class="badge">code only</span>{/if}
        </div>
      </div>
      <div class="actions">
        <a class="gear" href="/settings" title="Settings">⚙</a>
        <button class="sync-btn" onclick={doSync} disabled={syncingLive}>
          {#if syncingLive}<span class="spin" aria-hidden="true"></span>Syncing…{:else}Sync now{/if}
        </button>
        <div class="synced dim">
          {#if syncingLive && tick && tick.total}
            {tick.done}/{tick.total} · <span class="mono">{tick.repo}</span>
          {:else if syncingLive}
            {syncMessage}
          {:else}
            synced {relativeTime(snapshot.lastSyncedAt)}
          {/if}
        </div>
      </div>
    </header>

    {#if syncingLive}
      <div class="restrip" class:indeterminate={!tick || !tick.total}>
        <div
          class="rfill"
          style={tick && tick.total ? `width:${(tick.done / tick.total) * 100}%` : ""}
        ></div>
      </div>
    {/if}
    {#if error}<p class="remove">{error}</p>{/if}

    <section>
      <h2>Language share</h2>
      <Treemap languages={snapshot.languages} />
    </section>

    <section>
      <h2>By language</h2>
      <LanguageBars languages={snapshot.languages} style={appearance.barStyle} />
    </section>

    <section>
      <h2>Repositories</h2>
      <table>
        <thead>
          <tr>
            <th class="lh" onclick={() => setSort("name")}>repo{sortKey === "name" ? (sortDir < 0 ? " ↓" : " ↑") : ""}</th>
            <th onclick={() => setSort("added")}>added{sortKey === "added" ? (sortDir < 0 ? " ↓" : " ↑") : ""}</th>
            <th onclick={() => setSort("removed")}>removed{sortKey === "removed" ? (sortDir < 0 ? " ↓" : " ↑") : ""}</th>
            <th onclick={() => setSort("net")}>net{sortKey === "net" ? (sortDir < 0 ? " ↓" : " ↑") : ""}</th>
            <th class="lh">top</th>
          </tr>
        </thead>
        <tbody>
          {#each sortedRepos as r (r.fullName)}
            <tr>
              <td class="mono">{r.fullName}</td>
              <td class="add mono">+{commas(r.added)}</td>
              <td class="remove mono">−{commas(r.removed)}</td>
              <td class="mono {r.net >= 0 ? 'add' : 'remove'}">{signed(r.net)}</td>
              <td class="dim">{r.topLanguage ?? "-"}</td>
            </tr>
          {/each}
          </tbody>
        </table>
        </section>
      </div>
    </div>

    <div class="page tree-page">
      <ImpactTree {snapshot} {profile} />
    </div>
  </div>

  <div class="dots">
    <button class:active={page === 0} onclick={() => goPage(0)} title="Dashboard" aria-label="Dashboard"></button>
    <button class:active={page === 1} onclick={() => goPage(1)} title="Impact tree" aria-label="Impact tree"></button>
  </div>
{:else}
  <main class:wide={phase === "live"}>
    {#if phase === "connect"}
      <div class="empty">
        <h1>Lineage</h1>
        <p class="dim">Connect your GitHub account to see your Lineage.</p>
        <button class="primary" onclick={() => api.openOnboarding()}>Connect GitHub</button>
      </div>
    {:else if phase === "firstrun"}
      <div class="empty">
        <h1>Ready</h1>
        <p class="dim">
          Lineage will clone your repositories and tally every line you have written. The
          first run takes a few minutes; after that, re-syncs are fast.
        </p>
        {#if error}<p class="remove">{error}</p>{/if}
        <button class="primary" onclick={doSync}>Compute my Lineage</button>
      </div>
    {:else if phase === "live"}
      <LiveReveal
        net={tick?.net ?? 0}
        added={tick?.added ?? 0}
        removed={tick?.removed ?? 0}
        commits={tick?.commits ?? 0}
        done={tick?.done ?? 0}
        total={tick?.total ?? syncStatus?.reposTotal ?? 0}
        languages={tick?.languages ?? []}
        barStyle={appearance.barStyle}
        message={syncMessage}
        {feed}
      />
    {/if}
  </main>
{/if}

<style>
  main {
    padding: 22px 26px 44px;
    max-width: 720px;
    margin: 0 auto;
  }
  main.wide {
    max-width: 920px;
  }

  /* Swipe pager: dashboard <-> impact tree. */
  .pager {
    display: flex;
    height: 100vh;
    overflow-x: auto;
    overflow-y: hidden;
    scroll-snap-type: x mandatory;
    scrollbar-width: none;
  }
  .pager::-webkit-scrollbar {
    display: none;
  }
  .page {
    flex: 0 0 100%;
    width: 100%;
    height: 100vh;
    scroll-snap-align: start;
    scrollbar-width: none;
  }
  .page::-webkit-scrollbar {
    display: none;
  }
  .page-scroll {
    overflow-y: auto;
  }
  .page.tree-page {
    overflow: hidden;
  }
  .col {
    max-width: 920px;
    margin: 0 auto;
    padding: 22px 26px 64px;
  }
  .dots {
    position: fixed;
    bottom: 14px;
    left: 0;
    right: 0;
    display: flex;
    justify-content: center;
    gap: 9px;
    z-index: 5;
    pointer-events: none;
  }
  .dots button {
    pointer-events: auto;
    width: 8px;
    height: 8px;
    padding: 0;
    border-radius: 50%;
    border: none;
    background: var(--border);
    transition: background 0.15s, transform 0.15s;
  }
  .dots button:hover {
    background: var(--text-faint);
  }
  .dots button.active {
    background: var(--accent);
    transform: scale(1.25);
  }
  .empty {
    text-align: center;
    margin-top: 16vh;
    max-width: 460px;
    margin-left: auto;
    margin-right: auto;
  }
  .empty h1 {
    margin: 0 0 6px;
  }
  .empty button {
    margin-top: 18px;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 16px;
    margin-bottom: 14px;
  }
  .net {
    font-size: 54px;
    font-weight: 800;
    line-height: 1;
    letter-spacing: var(--track-display);
  }
  .sub {
    margin-top: 8px;
    display: flex;
    gap: 10px;
    align-items: center;
    flex-wrap: wrap;
  }
  .fav {
    font-size: 12px;
    color: #ffcf56;
  }
  .badge {
    font-size: 11px;
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 2px 8px;
    color: var(--text-dim);
  }
  .actions {
    text-align: right;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 8px;
  }
  .gear {
    text-decoration: none;
    font-size: 18px;
    opacity: 0.7;
  }
  .gear:hover {
    opacity: 1;
  }
  .synced {
    font-size: 12px;
    min-height: 15px;
  }
  .sync-btn {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    min-width: 96px;
    justify-content: center;
  }
  .spin {
    width: 11px;
    height: 11px;
    border: 2px solid color-mix(in srgb, var(--text) 35%, transparent);
    border-top-color: var(--text);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .restrip {
    height: 3px;
    background: var(--bg-elev-2);
    border-radius: 999px;
    overflow: hidden;
    margin-bottom: 14px;
  }
  .rfill {
    height: 100%;
    background: var(--accent);
    transition: width 0.3s ease;
  }
  /* Before the first repo result, total is unknown: sweep an indeterminate bar. */
  .restrip.indeterminate .rfill {
    width: 40%;
    border-radius: 999px;
    animation: sweep 1.1s ease-in-out infinite;
  }
  @keyframes sweep {
    0% {
      margin-left: -42%;
    }
    100% {
      margin-left: 100%;
    }
  }
  section {
    margin-top: 26px;
  }
  h2 {
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: var(--track-label);
    color: var(--text-dim);
    margin: 0 0 12px;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }
  th {
    text-align: right;
    color: var(--text-faint);
    font-weight: 500;
    padding: 6px 10px;
    border-bottom: 1px solid var(--border);
    cursor: pointer;
    user-select: none;
  }
  th.lh {
    text-align: left;
  }
  th:hover {
    color: var(--text);
  }
  td {
    padding: 5px 10px;
    text-align: right;
    border-bottom: 1px solid var(--bg-elev);
  }
  td:first-child {
    text-align: left;
  }
</style>
