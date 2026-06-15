<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import * as api from "$lib/api/client";
  import type { AppSettings, AppearanceSettings, AuthStatus } from "$lib/api/types";

  let settings = $state<AppSettings | null>(null);
  let appearance = $state<AppearanceSettings | null>(null);
  let auth = $state<AuthStatus>({ connected: false, source: null, login: null });
  let emailsText = $state("");
  let cache = $state("…");
  let busy = $state(false);
  let error = $state<string | null>(null);

  const ISSUES_URL = "https://github.com/Jam-Sw/lineage/issues";

  // Destructive actions take an explicit second step. Disconnect is click-again-
  // to-confirm; the full uninstall is click-again, then type "delete".
  let disconnectArmed = $state(false);
  let uninstallStage = $state<0 | 1>(0); // 0 idle, 1 type-to-confirm
  let confirmText = $state("");
  let uninstallBusy = $state(false);

  let disconnectTimer: ReturnType<typeof setTimeout> | undefined;
  const armed = $derived(confirmText.trim().toLowerCase() === "delete");

  onMount(async () => {
    settings = await api.getSettings();
    emailsText = settings.extraEmails.join(", ");
    appearance = await api.getAppearance();
    auth = await api.authStatus();
    void refreshCache();
    // Opened from the tray Help menu: bring the uninstall area into view.
    if (location.hash === "#uninstall") {
      setTimeout(
        () => document.getElementById("uninstall")?.scrollIntoView({ behavior: "smooth" }),
        60,
      );
    }
  });

  // Appearance changes apply live (tray + dashboard react immediately).
  async function saveAppearance() {
    if (appearance) await api.setAppearance(appearance);
  }

  async function refreshCache() {
    try {
      cache = await api.cacheInfo();
    } catch {
      cache = "unknown";
    }
  }

  async function save(resync: boolean) {
    if (!settings) return;
    busy = true;
    error = null;
    try {
      settings.extraEmails = emailsText
        .split(",")
        .map((s) => s.trim())
        .filter(Boolean);
      await api.setSettings(settings);
      if (resync) await api.syncNow();
      goto("/");
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  async function clearCache() {
    busy = true;
    try {
      await api.clearCache();
      await refreshCache();
    } finally {
      busy = false;
    }
  }

  // Disconnect: first click arms (reverts after 3s), second click actually drops
  // the Keychain token, so it is never a one-click accident.
  function clickDisconnect() {
    if (!disconnectArmed) {
      disconnectArmed = true;
      clearTimeout(disconnectTimer);
      disconnectTimer = setTimeout(() => (disconnectArmed = false), 3000);
    } else {
      clearTimeout(disconnectTimer);
      disconnectArmed = false;
      void disconnect();
    }
  }

  async function disconnect() {
    await api.disconnect();
    auth = { connected: false, source: null, login: null };
  }

  function clickUninstall() {
    uninstallStage = 1;
  }

  function cancelUninstall() {
    uninstallStage = 0;
    confirmText = "";
  }

  async function uninstall() {
    if (!armed) return;
    uninstallBusy = true;
    error = null;
    try {
      await api.uninstallApp();
      // The app removes its data and quits itself; this view is going away.
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      uninstallBusy = false;
    }
  }
</script>

<main>
  <header>
    <a href="/" class="back">← Dashboard</a>
    <h1>Settings</h1>
  </header>

  {#if appearance}
    <section>
      <h2>Appearance</h2>
      <label class="field">
        <span>Menu bar icon</span>
        <select bind:value={appearance.trayIcon} onchange={saveAppearance}>
          <option value="plusMinus">Green + / red −</option>
          <option value="diffBars">Diff bars</option>
          <option value="none">None (number only)</option>
        </select>
      </label>
      <label class="row">
        <input type="checkbox" bind:checked={appearance.trayShowNumber} onchange={saveAppearance} />
        <span><b>Show the number in the menu bar</b></span>
      </label>
      <label class="field">
        <span>Number shows</span>
        <select
          bind:value={appearance.trayMetric}
          onchange={saveAppearance}
          disabled={!appearance.trayShowNumber}
        >
          <option value="net">Net diff (+388k)</option>
          <option value="addedRemoved">Added and removed (+388k −97k)</option>
        </select>
      </label>
      <label class="field">
        <span>Dashboard bars</span>
        <select bind:value={appearance.barStyle} onchange={saveAppearance}>
          <option value="language">Language color</option>
          <option value="diff">Added / removed split</option>
        </select>
      </label>
    </section>
  {/if}

  {#if settings}
    <section>
      <h2>When you close the window</h2>
      <label class="field">
        <span>Closing Lineage</span>
        <select bind:value={settings.closeBehavior}>
          <option value="ask">Ask me each time</option>
          <option value="menuBar">Keep running in the menu bar</option>
          <option value="quit">Quit the app completely</option>
        </select>
      </label>
      <small class="dim">Applies on Save.</small>
    </section>

    <section>
      <h2>What counts</h2>
      <label class="row">
        <input type="checkbox" bind:checked={settings.excludeGenerated} />
        <span>
          <b>Exclude generated &amp; vendored files</b>
          <small class="dim">node_modules, lockfiles, minified bundles. The meaningful number.</small>
        </span>
      </label>
      <label class="row">
        <input type="checkbox" bind:checked={settings.includeForks} />
        <span><b>Include forks</b><small class="dim">Counts upstream history you did not write.</small></span>
      </label>
      <label class="row">
        <input type="checkbox" bind:checked={settings.ownerOnly} />
        <span><b>Only repos I own</b><small class="dim">Excludes org and collaborator repositories.</small></span>
      </label>
      <label class="row">
        <input type="checkbox" bind:checked={settings.includeArchived} />
        <span><b>Include archived repos</b></span>
      </label>
    </section>

    <section>
      <h2>Author emails</h2>
      <p class="dim small">
        Your <code>noreply</code> address is always counted. Add any other emails you have
        committed under, comma separated.
      </p>
      <input type="text" bind:value={emailsText} placeholder="me@example.com, old@work.com" />
    </section>

    <section>
      <h2>Account</h2>
      {#if auth.connected}
        <p class="small">
          Connected as <b>{auth.login}</b> <span class="dim">({auth.source})</span>
        </p>
        <button class:armed={disconnectArmed} onclick={clickDisconnect}>
          {disconnectArmed ? "Click again to disconnect" : "Disconnect"}
        </button>
      {:else}
        <p class="dim small">Not connected.</p>
      {/if}
    </section>

    <section>
      <h2>Clone cache</h2>
      <p class="small">Local clones: <b>{cache}</b></p>
      <button onclick={clearCache} disabled={busy}>Clear cache</button>
      <small class="dim">Frees disk; the next sync re-clones.</small>
    </section>

    {#if error}<p class="remove">{error}</p>{/if}

    <div class="foot">
      <button onclick={() => goto("/")} disabled={busy}>Cancel</button>
      <button onclick={() => save(false)} disabled={busy}>Save</button>
      <button class="primary" onclick={() => save(true)} disabled={busy}>Save &amp; re-sync</button>
    </div>

    <section id="uninstall">
      <h2>Help</h2>
      <p class="small">
        Questions, a bug, or an idea? <button class="link" onclick={() => api.openUrl(ISSUES_URL)}
          >Open an issue on GitHub ↗</button
        >. Happy to help.
      </p>

      <div class="uninstall">
        <p class="small">
          <b>Uninstall Lineage.</b>
          <small class="dim"
            >Cleanly removes the app and all of its local data - clones, cache, database, and
            the GitHub token in your Keychain. Your GitHub account is never touched.</small
          >
        </p>
        {#if uninstallStage === 0}
          <div class="actions-row">
            <button onclick={clickUninstall}>Uninstall Lineage</button>
          </div>
        {:else}
          <div class="actions-row">
            <span class="dim small">Type <code>delete</code> to remove</span>
            <input
              class="confirm-input"
              type="text"
              bind:value={confirmText}
              placeholder="delete"
              autocomplete="off"
              autocapitalize="off"
              spellcheck="false"
            />
            <button onclick={cancelUninstall} disabled={uninstallBusy}>Cancel</button>
            <button class:armed onclick={uninstall} disabled={!armed || uninstallBusy}>
              {uninstallBusy ? "Removing…" : "Remove"}
            </button>
          </div>
        {/if}
      </div>
    </section>
  {/if}
</main>

<style>
  main {
    padding: 22px 26px 40px;
    max-width: 560px;
    margin: 0 auto;
  }
  header {
    margin-bottom: 18px;
  }
  .back {
    text-decoration: none;
    font-size: 13px;
  }
  h1 {
    margin: 8px 0 0;
  }
  section {
    margin-top: 22px;
    border-top: 1px solid var(--border);
    padding-top: 16px;
  }
  h2 {
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: var(--track-label);
    color: var(--text-dim);
    margin: 0 0 12px;
  }
  .small {
    font-size: 13px;
  }
  .field {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    padding: 7px 0;
    font-size: 13px;
  }
  .row {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    padding: 7px 0;
    cursor: pointer;
  }
  .row input {
    margin-top: 3px;
  }
  .row span {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .row small {
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
    display: flex;
    gap: 10px;
    justify-content: flex-end;
  }

  /* Inline link button (opens externally via the system browser). */
  .link {
    background: none;
    border: none;
    padding: 0;
    color: var(--accent);
    font: inherit;
    cursor: pointer;
  }
  .link:hover {
    text-decoration: underline;
  }
  .uninstall {
    margin-top: 12px;
  }
  .uninstall p {
    margin: 0 0 10px;
    line-height: 1.5;
  }
  .uninstall small {
    display: block;
    margin-top: 2px;
    font-size: 12px;
  }
  .actions-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .confirm-input {
    width: 120px;
  }
  /* Destructive buttons stay neutral until the user explicitly arms them, so a
     stray click never wipes anything. */
  button.armed {
    background: var(--remove);
    border-color: var(--remove);
    color: #fff;
  }
</style>
