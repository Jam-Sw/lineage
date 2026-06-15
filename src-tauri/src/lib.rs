//! App shell: tray (with the live "Lineage" title), windows, IPC commands,
//! and the background sync that drives `lineage-core`'s deep engine off the UI
//! thread. No business logic here - that lives in the core crate.

use lineage_core::credential::{self, CredentialSource, TokenStore};
use lineage_core::github::GithubClient;
use lineage_core::numstat::ChurnOptions;
use lineage_core::types::{
    AppSettings, AppearanceSettings, AuthStatus, LanguageStat, ProfileStats, RepoChurn, Scope,
    Snapshot, Summary, SyncStatus,
};
use lineage_core::{aggregate, engine, AppError, Store};
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, State};

const REPO_URL: &str = "https://github.com/Jam-Sw/lineage";
const BUNDLE_ID: &str = "com.lineage.app";

struct AppState {
    store: Mutex<Store>,
    cache_dir: PathBuf,
    syncing: AtomicBool,
    /// True while the menu-bar spinner thread should keep animating.
    animating: AtomicBool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CmdError {
    code: String,
    message: String,
}

impl From<AppError> for CmdError {
    fn from(e: AppError) -> Self {
        CmdError { code: e.code().to_string(), message: e.to_string() }
    }
}

type CmdResult<T> = Result<T, CmdError>;

/// A rich, statistically-driving progress event: running totals plus the just-
/// processed repo, so the UI can animate a live tally instead of a dead bar.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncTickPayload {
    done: usize,
    total: usize,
    repo: String,
    repo_added: u64,
    repo_removed: u64,
    repo_top_language: Option<String>,
    repo_commits: u64,
    from_cache: bool,
    added: u64,
    removed: u64,
    net: i64,
    commits: u64,
    languages: Vec<LanguageStat>,
}

/// A lightweight phase signal so the UI shows motion during the gap between the
/// click and the first per-repo tick (token read, repo discovery), instead of
/// looking frozen.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncPhasePayload {
    phase: String,
    message: String,
}

fn emit_phase(app: &AppHandle, phase: &str, message: &str) {
    let _ = app.emit(
        "sync:phase",
        SyncPhasePayload { phase: phase.into(), message: message.into() },
    );
}

impl SyncTickPayload {
    fn start(total: usize) -> Self {
        SyncTickPayload {
            done: 0,
            total,
            repo: String::new(),
            repo_added: 0,
            repo_removed: 0,
            repo_top_language: None,
            repo_commits: 0,
            from_cache: false,
            added: 0,
            removed: 0,
            net: 0,
            commits: 0,
            languages: Vec::new(),
        }
    }
}

/// Running aggregate updated as each repo completes (shared across worker threads).
#[derive(Default)]
struct RunningAgg {
    added: u64,
    removed: u64,
    commits: u64,
    per_language: BTreeMap<String, (u64, u64)>,
}

impl RunningAgg {
    fn add(&mut self, c: &RepoChurn) {
        self.added += c.added;
        self.removed += c.removed;
        self.commits += c.commits;
        for (lang, (a, r)) in &c.per_language {
            let e = self.per_language.entry(lang.clone()).or_insert((0, 0));
            e.0 += a;
            e.1 += r;
        }
    }

    fn payload(&self, done: usize, total: usize, rr: &engine::RepoResult) -> SyncTickPayload {
        SyncTickPayload {
            done,
            total,
            repo: rr.churn.full_name.clone(),
            repo_added: rr.churn.added,
            repo_removed: rr.churn.removed,
            repo_top_language: rr.churn.top_language(),
            repo_commits: rr.churn.commits,
            from_cache: rr.from_cache,
            added: self.added,
            removed: self.removed,
            net: self.added as i64 - self.removed as i64,
            commits: self.commits,
            languages: aggregate::language_stats(&self.per_language)
                .into_iter()
                .take(14)
                .collect(),
        }
    }
}

fn store_lock<'a>(state: &'a State<'_, AppState>) -> CmdResult<std::sync::MutexGuard<'a, Store>> {
    state.store.lock().map_err(|_| CmdError {
        code: "STORAGE_ERROR".into(),
        message: "state lock poisoned".into(),
    })
}

// ---- data commands ----

#[tauri::command]
fn auth_status(state: State<'_, AppState>) -> CmdResult<AuthStatus> {
    Ok(store_lock(&state)?.auth_status()?)
}

#[tauri::command]
fn get_snapshot(state: State<'_, AppState>) -> CmdResult<Option<Snapshot>> {
    Ok(store_lock(&state)?.get_snapshot()?)
}

#[tauri::command]
fn get_sync_status(state: State<'_, AppState>) -> CmdResult<SyncStatus> {
    Ok(store_lock(&state)?.sync_status()?)
}

/// The cached impact-tree profile (contributions graph + avatar). Populated by
/// sync; `refresh_profile` fetches it on demand when the cache is empty.
#[tauri::command]
fn get_profile(state: State<'_, AppState>) -> CmdResult<Option<ProfileStats>> {
    Ok(store_lock(&state)?.get_profile()?)
}

/// Fetch the contributions graph off-thread and emit `profile:done` when ready.
#[tauri::command]
fn refresh_profile(app: AppHandle) -> CmdResult<()> {
    std::thread::spawn(move || {
        let _ = fetch_and_store_profile(&app);
    });
    Ok(())
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> CmdResult<AppSettings> {
    Ok(store_lock(&state)?.get_settings()?)
}

#[tauri::command]
fn set_settings(
    state: State<'_, AppState>,
    app: AppHandle,
    settings: AppSettings,
) -> CmdResult<AppSettings> {
    {
        let s = store_lock(&state)?;
        // Scope/filter affect every repo's churn, so invalidate the cache when any
        // of them changed: the next sync recomputes from clones (which are kept)
        // under the new settings. Cosmetic fields (e.g. seen_tour) leave it intact.
        let prev = s.get_settings()?;
        let scope_changed = prev.include_forks != settings.include_forks
            || prev.owner_only != settings.owner_only
            || prev.include_archived != settings.include_archived
            || prev.exclude_generated != settings.exclude_generated
            || prev.extra_emails != settings.extra_emails;
        s.set_settings(&settings)?;
        if scope_changed {
            s.clear_repo_cache()?;
        }
    }
    let _ = app.emit("settings:changed", &settings);
    Ok(settings)
}

#[tauri::command]
fn get_appearance(state: State<'_, AppState>) -> CmdResult<AppearanceSettings> {
    Ok(store_lock(&state)?.get_appearance()?)
}

#[tauri::command]
fn set_appearance(
    state: State<'_, AppState>,
    app: AppHandle,
    appearance: AppearanceSettings,
) -> CmdResult<AppearanceSettings> {
    store_lock(&state)?.set_appearance(&appearance)?;
    refresh_tray(&app);
    let _ = app.emit("appearance:changed", &appearance);
    Ok(appearance)
}

// ---- auth commands ----

#[tauri::command]
fn gh_available() -> bool {
    credential::gh_cli_available()
}

#[tauri::command]
fn connect_via_gh(state: State<'_, AppState>, app: AppHandle) -> CmdResult<AuthStatus> {
    let token = credential::from_gh_cli()?;
    let (user, src) = credential::connect(token, CredentialSource::GhCli)?;
    store_lock(&state)?.set_credential_meta(src.as_str(), &user.login)?;
    finish_connect(&state, &app)
}

#[tauri::command]
fn connect_via_pat(
    state: State<'_, AppState>,
    app: AppHandle,
    token: String,
) -> CmdResult<AuthStatus> {
    let token = lineage_core::sensitive::Sensitive(token.trim().to_string());
    let (user, src) = credential::connect(token, CredentialSource::Pat)?;
    store_lock(&state)?.set_credential_meta(src.as_str(), &user.login)?;
    finish_connect(&state, &app)
}

fn finish_connect(state: &State<'_, AppState>, app: &AppHandle) -> CmdResult<AuthStatus> {
    let status = store_lock(state)?.auth_status()?;
    let _ = app.emit("auth:changed", &status);
    if let Some(w) = app.get_webview_window("onboarding") {
        let _ = w.hide();
    }
    show_window(app, "dashboard");
    spawn_sync(app.clone());
    Ok(status)
}

#[tauri::command]
fn disconnect(state: State<'_, AppState>, app: AppHandle) -> CmdResult<AuthStatus> {
    let _ = TokenStore::clear();
    {
        let s = store_lock(&state)?;
        s.clear_credential_meta()?;
        let _ = s.clear_profile();
    }
    let status = store_lock(&state)?.auth_status()?;
    let _ = app.emit("auth:changed", &status);
    set_tray_title(&app, "-");
    Ok(status)
}

// ---- actions ----

/// Kick off a sync. Returns `false` when one was already in flight (the existing
/// run keeps streaming, so the UI should stay in its syncing state either way).
#[tauri::command]
fn sync_now(app: AppHandle) -> CmdResult<bool> {
    Ok(spawn_sync(app))
}

#[tauri::command]
fn open_url(url: String) {
    open_with_system(&url);
}

#[tauri::command]
fn open_dashboard(app: AppHandle) {
    show_window(&app, "dashboard");
}

#[tauri::command]
fn open_onboarding(app: AppHandle) {
    show_window(&app, "onboarding");
}

#[tauri::command]
fn open_data_folder(app: AppHandle) {
    if let Ok(dir) = app.path().app_data_dir() {
        open_with_system(&dir.to_string_lossy());
    }
}

/// Record and carry out the user's answer to the first-close prompt: keep running
/// in the menu bar (hide the window) or quit the app entirely. `close_behavior` is
/// a cosmetic field, so this writes settings directly and never clears the churn
/// cache. Any value other than "quit" is treated as the menu-bar choice.
#[tauri::command]
fn resolve_close(state: State<'_, AppState>, app: AppHandle, behavior: String) -> CmdResult<()> {
    let quit = behavior == "quit";
    {
        let s = store_lock(&state)?;
        let mut settings = s.get_settings()?;
        settings.close_behavior = if quit { "quit".into() } else { "menuBar".into() };
        s.set_settings(&settings)?;
    }
    if quit {
        app.exit(0);
    } else if let Some(w) = app.get_webview_window("dashboard") {
        let _ = w.hide();
    }
    Ok(())
}

#[tauri::command]
fn cache_info(state: State<'_, AppState>) -> CmdResult<String> {
    Ok(human_size(dir_size(&state.cache_dir)))
}

#[tauri::command]
fn clear_cache(state: State<'_, AppState>) -> CmdResult<()> {
    store_lock(&state)?.clear_repo_cache()?;
    let _ = std::fs::remove_dir_all(&state.cache_dir);
    Ok(())
}

/// Completely and cleanly uninstall Lineage: drop the GitHub token from the
/// Keychain, delete every file the app scattered across `~/Library` (clones,
/// SQLite cache, WebKit/caches/saved-state - the cruft macOS otherwise leaves
/// behind), move the app bundle to the Trash, and quit.
///
/// The bundle removal is deferred to a detached helper that waits for this
/// process to exit, since a running app cannot cleanly delete its own bundle.
/// The GitHub account itself is never touched - only the local token copy.
#[tauri::command]
fn uninstall_app(app: AppHandle) -> CmdResult<()> {
    // 1. The token in the macOS Keychain.
    let _ = TokenStore::clear();

    // 2. App data (bare clones + SQLite) and the rest of the per-bundle footprint.
    if let Ok(dir) = app.path().app_data_dir() {
        let _ = std::fs::remove_dir_all(&dir);
    }
    if let Ok(home) = app.path().home_dir() {
        for rel in [
            format!("Library/Caches/{BUNDLE_ID}"),
            format!("Library/WebKit/{BUNDLE_ID}"),
            format!("Library/HTTPStorages/{BUNDLE_ID}"),
            format!("Library/Saved Application State/{BUNDLE_ID}.savedState"),
            format!("Library/Preferences/{BUNDLE_ID}.plist"),
        ] {
            let p = home.join(rel);
            let _ = std::fs::remove_file(&p);
            let _ = std::fs::remove_dir_all(&p);
        }
    }

    // 3. Move the .app bundle to the Trash once we have exited. Skip in dev,
    //    where the binary lives under target/ and not inside a .app.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(bundle) = exe.ancestors().nth(3) {
            if bundle.extension().and_then(|e| e.to_str()) == Some("app") {
                let pid = std::process::id();
                let bundle = bundle.to_string_lossy().replace('"', "");
                // Move to the user's Trash (recoverable, no Finder-automation
                // prompt); fall back to a hard delete if the move fails (e.g. a
                // cross-volume install) so the bundle is gone either way.
                let script = format!(
                    "while kill -0 {pid} 2>/dev/null; do sleep 0.2; done; \
                     name=\"$(basename \"{bundle}\")\"; \
                     rm -rf \"$HOME/.Trash/$name\"; \
                     mv \"{bundle}\" \"$HOME/.Trash/\" 2>/dev/null || rm -rf \"{bundle}\""
                );
                let _ = std::process::Command::new("/bin/bash")
                    .arg("-c")
                    .arg(script)
                    .spawn();
            }
        }
    }

    // 4. Quit; the detached helper trashes the bundle right after we go.
    app.exit(0);
    Ok(())
}

/// Persist an exported impact-tree PNG (base64) to the Desktop and reveal it in
/// Finder. Returns the saved path so the UI can confirm. Keeps image export
/// entirely in Rust file IO, no extra Tauri plugins.
#[tauri::command]
fn save_tree_image(app: AppHandle, data_b64: String, login: String) -> CmdResult<String> {
    let bytes = base64_decode(&data_b64).ok_or_else(|| CmdError {
        code: "VALIDATION".into(),
        message: "invalid image data".into(),
    })?;
    let dir = app
        .path()
        .desktop_dir()
        .or_else(|_| app.path().home_dir())
        .map_err(|e| CmdError { code: "STORAGE_ERROR".into(), message: e.to_string() })?;
    let safe: String = login
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    let stem = if safe.is_empty() { "lineage".to_string() } else { format!("lineage-{safe}") };
    let path = dir.join(format!("{stem}.png"));
    std::fs::write(&path, &bytes)
        .map_err(|e| CmdError { code: "STORAGE_ERROR".into(), message: e.to_string() })?;
    let display = path.to_string_lossy().to_string();
    let _ = std::process::Command::new("open").arg("-R").arg(&display).spawn();
    Ok(display)
}

/// Standard base64 decode (RFC 4648), inverse of the core's encoder. Ignores
/// whitespace; rejects other invalid characters.
fn base64_decode(s: &str) -> Option<Vec<u8>> {
    fn val(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    let mut quad = [0u8; 4];
    let mut n = 0;
    for &c in s.as_bytes() {
        if c == b'=' || c.is_ascii_whitespace() {
            continue;
        }
        quad[n] = val(c)?;
        n += 1;
        if n == 4 {
            out.push((quad[0] << 2) | (quad[1] >> 4));
            out.push((quad[1] << 4) | (quad[2] >> 2));
            out.push((quad[2] << 6) | quad[3]);
            n = 0;
        }
    }
    match n {
        0 => {}
        2 => out.push((quad[0] << 2) | (quad[1] >> 4)),
        3 => {
            out.push((quad[0] << 2) | (quad[1] >> 4));
            out.push((quad[1] << 4) | (quad[2] >> 2));
        }
        _ => return None,
    }
    Some(out)
}

// ---- background sync ----

fn spawn_sync(app: AppHandle) -> bool {
    {
        let state = app.state::<AppState>();
        if state.syncing.swap(true, Ordering::SeqCst) {
            return false; // a sync is already running
        }
        // Mark "syncing" right away (before the ~2s of API discovery) so a window
        // opening now goes straight into the live reveal instead of the first-run screen.
        let _ = state.store.lock().map(|s| s.sync_begin(0));
        // Spin the menu-bar icon for the duration so the tray reads as "working"
        // instead of a static glyph. One animator at a time.
        if !state.animating.swap(true, Ordering::SeqCst) {
            let anim_app = app.clone();
            std::thread::spawn(move || animate_tray(anim_app));
        }
    }
    // Immediate feedback: the token read + discovery can take a few seconds (and a
    // first run may block on a Keychain prompt), so signal motion right now.
    emit_phase(&app, "preparing", "Starting sync\u{2026}");
    std::thread::spawn(move || {
        let result = run_sync(&app);
        let state = app.state::<AppState>();
        state.syncing.store(false, Ordering::SeqCst);
        // Stop the spinner, then let its loop observe the flag and exit (one frame
        // is ~110ms) so it queues no more icon swaps. Restoring the real icon goes
        // through the main thread too, so it lands after any pending spinner frame
        // (FIFO) and can't be clobbered by a trailing frame.
        state.animating.store(false, Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(160));
        let a = app.clone();
        let _ = app.run_on_main_thread(move || refresh_tray(&a));
        if let Err(e) = result {
            if let Ok(s) = state.store.lock() {
                let _ = s.sync_error(&e.to_string());
            }
            let _ = app.emit("sync:error", CmdError::from(e));
        }
    });
    true
}

/// Fetch the contributions graph + avatar and cache it, emitting `profile:done`.
/// Best-effort: callers treat a failure as non-fatal (the tree just lacks the
/// contributions layer until the next try).
fn fetch_and_store_profile(app: &AppHandle) -> lineage_core::Result<()> {
    let token = TokenStore::load()?
        .ok_or_else(|| AppError::NotConnected("not connected to GitHub".into()))?;
    let client = GithubClient::new(token.expose().clone());
    let profile = client.fetch_profile()?;
    let state = app.state::<AppState>();
    let _ = state.store.lock().map(|s| s.set_profile(&profile));
    let _ = app.emit("profile:done", &profile);
    Ok(())
}

fn run_sync(app: &AppHandle) -> lineage_core::Result<()> {
    let state = app.state::<AppState>();
    let cache_dir = state.cache_dir.clone();

    let settings = lock_store(&state)?.get_settings()?;
    let token = TokenStore::load()?
        .ok_or_else(|| AppError::NotConnected("not connected to GitHub".into()))?;

    let client = GithubClient::new(token.expose().clone());
    let user = client.get_user()?;

    // Impact-tree profile (contributions graph + avatar), fetched early so the
    // second page can render while the clone loop is still running. Best-effort:
    // a failure here must not abort the churn sync.
    if let Ok(profile) = client.fetch_profile() {
        if let Ok(s) = state.store.lock() {
            let _ = s.set_profile(&profile);
        }
        let _ = app.emit("profile:done", &profile);
    }

    let mut emails = vec![user.noreply_email()];
    emails.extend(settings.extra_emails.clone());

    let scope_cfg = Scope {
        include_forks: settings.include_forks,
        owner_only: settings.owner_only,
        include_archived: settings.include_archived,
        login: user.login.clone(),
    };
    emit_phase(app, "discovering", "Finding your repositories\u{2026}");
    let repos = engine::discover(&client, &scope_cfg)?;
    let cached = lock_store(&state)?.load_repo_cache()?;

    lock_store(&state)?.sync_begin(repos.len() as u32)?;
    let _ = app.emit("sync:tick", SyncTickPayload::start(repos.len()));
    emit_phase(app, "scanning", "Scanning repositories\u{2026}");

    let opts = if settings.exclude_generated {
        ChurnOptions::filtered()
    } else {
        ChurnOptions::raw()
    };

    // Stream a live tally as repos complete (parallel; reuses cache for unchanged repos).
    let running = Mutex::new(RunningAgg::default());
    let results = engine::sync(&cache_dir, &token, &repos, &emails, &opts, &cached, 8, |rr, done, total| {
        if let Ok(s) = state.store.lock() {
            let _ = s.sync_progress(done as u32, &rr.churn.full_name);
        }
        let payload = {
            let mut agg = running.lock().unwrap_or_else(|e| e.into_inner());
            agg.add(&rr.churn);
            agg.payload(done, total, rr)
        };
        let _ = app.emit("sync:tick", payload);
    })?;

    emit_phase(app, "saving", "Finishing up\u{2026}");
    // Persist the per-repo cache for fast incremental re-syncs.
    {
        let s = lock_store(&state)?;
        let mut keep = HashSet::new();
        for rr in &results {
            let _ = s.save_repo_cache(&rr.churn.full_name, rr.pushed_at.as_deref(), &rr.churn);
            keep.insert(rr.churn.full_name.clone());
        }
        let _ = s.prune_repo_cache(&keep);
    }

    let churns: Vec<RepoChurn> = results.into_iter().map(|r| r.churn).collect();
    let rollup = aggregate::rollup(&churns);
    lock_store(&state)?.sync_finish()?;
    let last_synced_at = lock_store(&state)?.sync_status()?.last_finished_at;

    let snapshot = Snapshot {
        summary: rollup.summary,
        languages: rollup.languages,
        repos: rollup.repos,
        filtered: settings.exclude_generated,
        last_synced_at,
    };
    lock_store(&state)?.set_snapshot(&snapshot)?;
    // The tray icon + title are refreshed by spawn_sync once the spinner stops.
    let _ = app.emit("sync:done", &snapshot);
    Ok(())
}

fn lock_store<'a>(state: &'a State<'_, AppState>) -> lineage_core::Result<std::sync::MutexGuard<'a, Store>> {
    state
        .store
        .lock()
        .map_err(|_| AppError::Storage("state lock poisoned".into()))
}

// ---- tray + windows ----

fn set_tray_title(app: &AppHandle, title: &str) {
    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = tray.set_title(Some(title));
    }
}

fn show_window(app: &AppHandle, label: &str) {
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// What closing the dashboard window should do, read from saved settings.
enum CloseChoice {
    /// First close: ask the user (handled by the in-app modal).
    Ask,
    /// Idle to the menu bar (hide the window) - the menu-bar-app norm.
    MenuBar,
    /// Actually terminate the app.
    Quit,
}

fn close_choice(app: &AppHandle) -> CloseChoice {
    let behavior = app
        .state::<AppState>()
        .store
        .lock()
        .ok()
        .and_then(|s| s.get_settings().ok())
        .map(|s| s.close_behavior)
        .unwrap_or_default();
    match behavior.as_str() {
        "menuBar" => CloseChoice::MenuBar,
        "quit" => CloseChoice::Quit,
        _ => CloseChoice::Ask,
    }
}

fn open_with_system(target: &str) {
    let _ = std::process::Command::new("open").arg(target).spawn();
}

/// Bring the dashboard window forward and navigate it to the Settings page,
/// anchored at the Help section where uninstall lives. Used by the tray Help menu
/// so the destructive action always goes through the in-app confirm flow.
fn open_settings(app: &AppHandle) {
    show_window(app, "dashboard");
    // Navigate via a client-side event (handled in the root layout) instead of a
    // hard webview reload, so SvelteKit routing stays intact.
    let _ = app.emit_to("dashboard", "nav", "/settings#uninstall");
}

fn dir_size(path: &Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for e in entries.flatten() {
            match e.metadata() {
                Ok(md) if md.is_dir() => total += dir_size(&e.path()),
                Ok(md) => total += md.len(),
                Err(_) => {}
            }
        }
    }
    total
}

fn human_size(bytes: u64) -> String {
    let mb = bytes as f64 / 1_048_576.0;
    if mb >= 1024.0 {
        format!("{:.1} GB", mb / 1024.0)
    } else {
        format!("{:.0} MB", mb)
    }
}

/// Rasterize rounded-rect shapes (x0,y0,x1,y1,radius,color as 0..1 fractions of a
/// square) into a colored RGBA menu-bar icon. Drawn at 4x then downscaled with
/// alpha-coverage averaging for clean anti-aliased edges. No asset needed.
fn rasterize(shapes: &[(f32, f32, f32, f32, f32, [u8; 3])]) -> tauri::image::Image<'static> {
    const W: usize = 36;
    const H: usize = 36;
    const S: usize = 4; // supersample factor
    let bw = W * S;
    let bh = H * S;
    let mut hi = vec![0u8; bw * bh * 4];

    for py in 0..bh {
        let fy = (py as f32 + 0.5) / bh as f32;
        for px in 0..bw {
            let fx = (px as f32 + 0.5) / bw as f32;
            for &(x0, y0, x1, y1, r, col) in shapes {
                let nx = fx.clamp(x0 + r, x1 - r);
                let ny = fy.clamp(y0 + r, y1 - r);
                let dx = fx - nx;
                let dy = fy - ny;
                if dx * dx + dy * dy <= r * r {
                    let i = (py * bw + px) * 4;
                    hi[i] = col[0];
                    hi[i + 1] = col[1];
                    hi[i + 2] = col[2];
                    hi[i + 3] = 255;
                    break;
                }
            }
        }
    }

    let mut out = vec![0u8; W * H * 4];
    let total = (S * S) as u32;
    for y in 0..H {
        for x in 0..W {
            let (mut cr, mut cg, mut cb, mut cov) = (0u32, 0u32, 0u32, 0u32);
            for dy in 0..S {
                for dx in 0..S {
                    let i = ((y * S + dy) * bw + (x * S + dx)) * 4;
                    if hi[i + 3] == 255 {
                        cr += hi[i] as u32;
                        cg += hi[i + 1] as u32;
                        cb += hi[i + 2] as u32;
                        cov += 1;
                    }
                }
            }
            let o = (y * W + x) * 4;
            if cov > 0 {
                out[o] = (cr / cov) as u8;
                out[o + 1] = (cg / cov) as u8;
                out[o + 2] = (cb / cov) as u8;
            }
            out[o + 3] = (cov * 255 / total) as u8;
        }
    }

    let leaked: &'static [u8] = Box::leak(out.into_boxed_slice());
    tauri::image::Image::new(leaked, W as u32, H as u32)
}

const GREEN: [u8; 3] = [63, 185, 80];
const GRAY: [u8; 3] = [139, 148, 158];
const RED: [u8; 3] = [248, 81, 73];

/// Build the configured menu-bar icon. "none" yields a transparent image so the
/// status item still shows its title.
fn tray_icon(style: &str) -> tauri::image::Image<'static> {
    match style {
        "diffBars" => rasterize(&[
            (0.16, 0.215, 0.86, 0.385, 0.085, GREEN),
            (0.16, 0.425, 0.58, 0.575, 0.075, GRAY),
            (0.16, 0.615, 0.86, 0.785, 0.085, RED),
        ]),
        "none" => rasterize(&[]),
        // "plusMinus" (default): a green plus over a red minus.
        _ => rasterize(&[
            (0.28, 0.29, 0.72, 0.41, 0.04, GREEN), // plus: horizontal arm
            (0.44, 0.18, 0.56, 0.52, 0.04, GREEN), // plus: vertical arm
            (0.28, 0.64, 0.72, 0.76, 0.04, RED),   // minus
        ]),
    }
}

// ---- menu-bar "working" spinner ----

const SPINNER_FRAMES: usize = 8;
const SPINNER_W: u32 = 36;
const SPINNER_H: u32 = 36;

/// Build one frame of a ring-of-dots spinner as a 36x36 RGBA buffer. The image is
/// black with the shape carried entirely in the alpha channel, so shown as a macOS
/// template image it tints itself for a light or dark menu bar. The bright "head"
/// dot is at `frame`'s position and the others fade around the ring (a comet),
/// which reads as motion as the head advances frame to frame.
fn build_spinner_rgba(frame: usize) -> &'static [u8] {
    const S: usize = 4; // supersample for clean anti-aliased dots
    const N: usize = SPINNER_FRAMES;
    const RING: f32 = 0.30; // ring radius, fraction of the square
    const DOT: f32 = 0.085; // dot radius
    const TAIL: f32 = 0.12; // dimmest (tail) dot intensity

    // Per-dot intensity: head brightest, fading backwards around the ring.
    let head = frame % N;
    let mut intensity = [0f32; N];
    for (k, slot) in intensity.iter_mut().enumerate() {
        let behind = (head + N - k) % N; // 0 = head, N-1 = tail
        *slot = (1.0 - behind as f32 / N as f32).max(TAIL);
    }
    // Dot centers on the ring, dot 0 at the top, advancing clockwise.
    let mut centers = [(0f32, 0f32); N];
    for (k, c) in centers.iter_mut().enumerate() {
        let theta = std::f32::consts::TAU * k as f32 / N as f32;
        *c = (0.5 + RING * theta.sin(), 0.5 - RING * theta.cos());
    }

    let (w, h) = (SPINNER_W as usize, SPINNER_H as usize);
    let mut out = vec![0u8; w * h * 4];
    let samples = (S * S) as f32;
    for y in 0..h {
        for x in 0..w {
            let mut acc = 0f32;
            for sy in 0..S {
                let fy = ((y * S + sy) as f32 + 0.5) / (h * S) as f32;
                for sx in 0..S {
                    let fx = ((x * S + sx) as f32 + 0.5) / (w * S) as f32;
                    let mut best = 0f32;
                    for k in 0..N {
                        let dx = fx - centers[k].0;
                        let dy = fy - centers[k].1;
                        if dx * dx + dy * dy <= DOT * DOT && intensity[k] > best {
                            best = intensity[k];
                        }
                    }
                    acc += best;
                }
            }
            // RGB stays 0 (black); template tinting uses the alpha as the mask.
            out[(y * w + x) * 4 + 3] = (acc / samples * 255.0).round() as u8;
        }
    }
    Box::leak(out.into_boxed_slice())
}

/// The eight spinner frame buffers, built once and reused (no per-frame alloc).
fn spinner_buf(frame: usize) -> &'static [u8] {
    use std::sync::OnceLock;
    static FRAMES: OnceLock<Vec<&'static [u8]>> = OnceLock::new();
    let frames = FRAMES.get_or_init(|| (0..SPINNER_FRAMES).map(build_spinner_rgba).collect());
    frames[frame % SPINNER_FRAMES]
}

/// Cycle the menu-bar icon through the spinner while `animating` is set. Exits
/// (leaving the icon untouched) as soon as the flag clears, so `spawn_sync` can
/// deterministically set the final icon afterward.
///
/// Two things keep it smooth rather than strobing: template mode is asserted just
/// once up front (re-asserting it per frame made AppKit clear-and-redraw the
/// status button, a ~10Hz flash), and every icon swap is marshaled to the main
/// thread, since AppKit status-item updates from a background thread tear.
fn animate_tray(app: AppHandle) {
    // Monochrome template icon, menu-bar-adaptive. Set once.
    let a = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(tray) = a.tray_by_id("main-tray") {
            let _ = tray.set_icon_as_template(true);
        }
    });

    let mut frame = 0usize;
    while app.state::<AppState>().animating.load(Ordering::SeqCst) {
        let buf = spinner_buf(frame);
        let a = app.clone();
        let _ = app.run_on_main_thread(move || {
            if let Some(tray) = a.tray_by_id("main-tray") {
                let _ = tray.set_icon(Some(tauri::image::Image::new(buf, SPINNER_W, SPINNER_H)));
            }
        });
        frame = frame.wrapping_add(1);
        std::thread::sleep(Duration::from_millis(110));
    }
}

/// Abbreviate a signed net diff for the menu bar: `+388k`, `-1.2M`, `+512`.
fn abbrev_signed(n: i64) -> String {
    format!("{}{}", if n >= 0 { "+" } else { "-" }, abbrev_unsigned(n.unsigned_abs()))
}

fn abbrev_unsigned(a: u64) -> String {
    if a >= 1_000_000 {
        format!("{:.1}M", a as f64 / 1_000_000.0)
    } else if a >= 1_000 {
        format!("{}k", a / 1_000)
    } else {
        format!("{a}")
    }
}

/// The menu-bar title text for the current appearance + summary.
fn tray_title(a: &AppearanceSettings, summary: Option<&Summary>) -> Option<String> {
    if !a.tray_show_number {
        return None;
    }
    match summary {
        None => Some("-".to_string()),
        Some(s) if a.tray_metric == "addedRemoved" => {
            Some(format!("+{} \u{2212}{}", abbrev_unsigned(s.added), abbrev_unsigned(s.removed)))
        }
        Some(s) => Some(abbrev_signed(s.net)),
    }
}

/// Re-render the tray icon + title from the stored appearance and latest snapshot.
fn refresh_tray(app: &AppHandle) {
    let state = app.state::<AppState>();
    let guard = match state.store.lock() {
        Ok(s) => s,
        Err(_) => return,
    };
    let appearance = guard.get_appearance().unwrap_or_default();
    let summary = guard.get_snapshot().ok().flatten().map(|snap| snap.summary);
    drop(guard);
    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = tray.set_icon(Some(tray_icon(&appearance.tray_icon)));
        let _ = tray.set_icon_as_template(false);
        let _ = tray.set_title(tray_title(&appearance, summary.as_ref()).as_deref());
    }
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            auth_status,
            get_snapshot,
            get_sync_status,
            get_profile,
            refresh_profile,
            get_settings,
            set_settings,
            get_appearance,
            set_appearance,
            gh_available,
            connect_via_gh,
            connect_via_pat,
            disconnect,
            sync_now,
            open_url,
            open_dashboard,
            open_onboarding,
            open_data_folder,
            resolve_close,
            cache_info,
            clear_cache,
            uninstall_app,
            save_tree_image,
        ])
        .setup(|app| {
            let dir = app
                .path()
                .app_data_dir()
                .expect("cannot resolve app data directory");
            std::fs::create_dir_all(&dir)?;
            let store = Store::open(&dir.join("lineage.db"))
                .map_err(|e| format!("cannot open store: {e}"))?;

            let connected = store.auth_status().map(|a| a.connected).unwrap_or(false);
            let appearance = store.get_appearance().unwrap_or_default();
            let summary = store.get_snapshot().ok().flatten().map(|s| s.summary);
            let cache_dir = dir.join("clones");

            app.manage(AppState {
                store: Mutex::new(store),
                cache_dir,
                syncing: AtomicBool::new(false),
                animating: AtomicBool::new(false),
            });

            // Tray - the app's permanent menu-bar presence.
            let open_dash = MenuItem::with_id(app, "open_dashboard", "Open Dashboard", true, None::<&str>)?;
            let sync = MenuItem::with_id(app, "sync_now", "Sync Now", true, None::<&str>)?;
            let about = PredefinedMenuItem::about(
                app,
                Some("About Lineage"),
                Some(AboutMetadata {
                    version: Some(env!("CARGO_PKG_VERSION").into()),
                    website: Some(REPO_URL.into()),
                    website_label: Some("Jam-Sw/lineage".into()),
                    ..Default::default()
                }),
            )?;
            let repo = MenuItem::with_id(app, "open_repo", "Repository on GitHub", true, None::<&str>)?;
            let data = MenuItem::with_id(app, "open_data", "Open Data Folder", true, None::<&str>)?;
            let settings = Submenu::with_items(
                app,
                "Settings",
                true,
                &[&about, &PredefinedMenuItem::separator(app)?, &repo, &data],
            )?;
            // Help groups support and the clean uninstall together: we help the
            // user while they stay, and help them leave cleanly if they go.
            let issue = MenuItem::with_id(app, "open_issue", "Open an Issue", true, None::<&str>)?;
            let uninstall_mi =
                MenuItem::with_id(app, "open_uninstall", "Uninstall Lineage…", true, None::<&str>)?;
            let help = Submenu::with_items(
                app,
                "Help",
                true,
                &[&issue, &PredefinedMenuItem::separator(app)?, &uninstall_mi],
            )?;
            let quit = MenuItem::with_id(app, "quit", "Quit Lineage", true, None::<&str>)?;
            let menu = Menu::with_items(
                app,
                &[
                    &open_dash,
                    &sync,
                    &PredefinedMenuItem::separator(app)?,
                    &settings,
                    &help,
                    &PredefinedMenuItem::separator(app)?,
                    &quit,
                ],
            )?;
            let tray = TrayIconBuilder::with_id("main-tray")
                .icon(tray_icon(&appearance.tray_icon))
                .icon_as_template(false)
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open_dashboard" => show_window(app, "dashboard"),
                    "sync_now" => {
                        spawn_sync(app.clone());
                    }
                    "open_repo" => open_with_system(REPO_URL),
                    "open_issue" => open_with_system(&format!("{REPO_URL}/issues")),
                    "open_uninstall" => open_settings(app),
                    "open_data" => {
                        if let Ok(dir) = app.path().app_data_dir() {
                            open_with_system(&dir.to_string_lossy());
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            let _ = tray.set_title(tray_title(&appearance, summary.as_ref()).as_deref());

            // Closing the dashboard is the one lifecycle choice we hand the user:
            // idle to the menu bar (the menu-bar-app norm) or actually quit - one
            // of the few Mac apps that closes when you close it. The first close
            // asks (via the in-app modal); after that we honor the saved choice.
            // Onboarding is a transient window and always just hides.
            for label in ["dashboard", "onboarding"] {
                if let Some(w) = app.get_webview_window(label) {
                    let win = w.clone();
                    let app_handle = app.handle().clone();
                    let is_dashboard = label == "dashboard";
                    w.on_window_event(move |event| {
                        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                            if !is_dashboard {
                                api.prevent_close();
                                let _ = win.hide();
                                return;
                            }
                            match close_choice(&app_handle) {
                                CloseChoice::Quit => app_handle.exit(0),
                                CloseChoice::MenuBar => {
                                    api.prevent_close();
                                    let _ = win.hide();
                                }
                                CloseChoice::Ask => {
                                    // Hold the window open and let the modal decide.
                                    api.prevent_close();
                                    let _ = win.show();
                                    let _ = win.set_focus();
                                    let _ = app_handle.emit_to("dashboard", "close:prompt", ());
                                }
                            }
                        }
                    });
                }
            }

            // First paint: dashboard if connected (and refresh), else onboarding.
            let handle = app.handle().clone();
            if connected {
                show_window(&handle, "dashboard");
                spawn_sync(handle);
            } else {
                show_window(&handle, "onboarding");
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::{abbrev_signed, base64_decode};

    #[test]
    fn base64_decodes_known_vectors() {
        assert_eq!(base64_decode("").unwrap(), b"");
        assert_eq!(base64_decode("Zg==").unwrap(), b"f");
        assert_eq!(base64_decode("Zm8=").unwrap(), b"fo");
        assert_eq!(base64_decode("Zm9v").unwrap(), b"foo");
        assert_eq!(base64_decode("Zm9vYmFy").unwrap(), b"foobar");
        // tolerant of whitespace/newlines that data URLs sometimes carry
        assert_eq!(base64_decode("Zm9v\nYmFy").unwrap(), b"foobar");
        assert!(base64_decode("@@@@").is_none());
    }

    #[test]
    fn abbreviates_signed() {
        assert_eq!(abbrev_signed(387_816), "+387k");
        assert_eq!(abbrev_signed(-852_684), "-852k");
        assert_eq!(abbrev_signed(1_250_000), "+1.2M");
        assert_eq!(abbrev_signed(-512), "-512");
        assert_eq!(abbrev_signed(0), "+0");
    }
}
