<script lang="ts">
  import { onMount } from "svelte";
  import * as api from "$lib/api/client";
  import type { Snapshot, SyncStatus, AuthStatus, SyncTick, RepoStat } from "$lib/api/types";
  import { commas, signed, relativeTime } from "$lib/format";
  import LiveReveal from "$lib/components/LiveReveal.svelte";
  import LanguageBars from "$lib/components/LanguageBars.svelte";
  import Treemap from "$lib/components/Treemap.svelte";

  type FeedItem = { repo: string; added: number; removed: number; lang: string | null; cached: boolean };

  let auth = $state<AuthStatus>({ connected: false, source: null, login: null });
  let snapshot = $state<Snapshot | null>(null);
  let syncStatus = $state<SyncStatus | null>(null);
  let tick = $state<SyncTick | null>(null);
  let feed = $state<FeedItem[]>([]);
  let syncingLive = $state(false);
  let error = $state<string | null>(null);

  let sortKey = $state<"net" | "added" | "removed" | "name">("net");
  let sortDir = $state<1 | -1>(-1);

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
  });

  async function load() {
    auth = await api.authStatus();
    snapshot = await api.getSnapshot();
    await refreshSync();
    if (syncStatus?.syncing) syncingLive = true;
  }

  async function refreshSync() {
    syncStatus = await api.getSyncStatus();
  }

  async function doSync() {
    error = null;
    syncingLive = true;
    feed = [];
    await api.syncNow();
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

<main class:wide={phase === "live" || phase === "dashboard"}>
  {#if phase === "connect"}
    <div class="empty">
      <h1>Master Diff</h1>
      <p class="dim">Connect your GitHub account to see your lifetime master diff.</p>
      <button class="primary" onclick={() => api.openOnboarding()}>Connect GitHub</button>
    </div>
  {:else if phase === "firstrun"}
    <div class="empty">
      <h1>Ready</h1>
      <p class="dim">
        Master Diff will clone your repositories and tally every line you have written. The
        first run takes a few minutes; after that, re-syncs are fast.
      </p>
      {#if error}<p class="remove">{error}</p>{/if}
      <button class="primary" onclick={doSync}>Compute my master diff</button>
    </div>
  {:else if phase === "live"}
    <LiveReveal
      net={tick?.net ?? 0}
      added={tick?.added ?? 0}
      removed={tick?.removed ?? 0}
      done={tick?.done ?? 0}
      total={tick?.total ?? syncStatus?.reposTotal ?? 0}
      languages={tick?.languages ?? []}
      {feed}
    />
  {:else if snapshot}
    <header>
      <div class="headline">
        <div class="net mono {snapshot.summary.net >= 0 ? 'add' : 'remove'}">
          {signed(snapshot.summary.net)}
        </div>
        <div class="sub">
          <span class="add">+{commas(snapshot.summary.added)}</span>
          <span class="remove">−{commas(snapshot.summary.removed)}</span>
          <span class="dim">· {snapshot.summary.repoCount} repos · {snapshot.summary.languageCount} languages</span>
          {#if snapshot.languages[0]}
            <span class="fav">★ {snapshot.languages[0].language}</span>
          {/if}
          {#if snapshot.filtered}<span class="badge">code only</span>{/if}
        </div>
      </div>
      <div class="actions">
        <a class="gear" href="/settings" title="Settings">⚙</a>
        <button onclick={doSync} disabled={syncingLive}>{syncingLive ? "Syncing…" : "Sync now"}</button>
        <div class="synced dim">
          {#if syncingLive && tick}
            {tick.done}/{tick.total} · <span class="mono">{tick.repo}</span>
          {:else}
            synced {relativeTime(snapshot.lastSyncedAt)}
          {/if}
        </div>
      </div>
    </header>

    {#if syncingLive && tick}
      <div class="restrip">
        <div class="rfill" style="width:{tick.total ? (tick.done / tick.total) * 100 : 0}%"></div>
      </div>
    {/if}
    {#if error}<p class="remove">{error}</p>{/if}

    <section>
      <h2>Language share</h2>
      <Treemap languages={snapshot.languages} />
    </section>

    <section>
      <h2>By language</h2>
      <LanguageBars languages={snapshot.languages} />
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
  {/if}
</main>

<style>
  main {
    padding: 22px 26px 44px;
    max-width: 720px;
    margin: 0 auto;
  }
  main.wide {
    max-width: 920px;
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
    font-weight: 700;
    line-height: 1;
    letter-spacing: -0.02em;
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
  section {
    margin-top: 26px;
  }
  h2 {
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
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
