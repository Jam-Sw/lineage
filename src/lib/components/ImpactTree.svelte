<script lang="ts">
  import { onMount } from "svelte";
  import type { Snapshot, ProfileStats } from "$lib/api/types";
  import { commas, signed } from "$lib/format";
  import * as api from "$lib/api/client";

  let { snapshot, profile }: { snapshot: Snapshot; profile: ProfileStats | null } = $props();

  const TAU = Math.PI * 2;
  const AVATAR_R = 54;
  const MAX_LEN = 430;
  const FALLBACK = "#8b949e";
  const BG = "#0f1115";
  const DIM = "#9aa3b2";
  const FAINT = "#6b7280";
  const TEXT = "#e6e9ef";
  const ADD = "#3fb950";
  const REMOVE = "#f85149";
  const FONT = "-apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif";
  const MONO = "ui-monospace, SFMono-Regular, 'SF Mono', Menlo, monospace";

  // Stable per-repo pseudo-randomness so the tree looks the same every render.
  function strHash(s: string): number {
    let h = 2166136261 >>> 0;
    for (let i = 0; i < s.length; i++) {
      h ^= s.charCodeAt(i);
      h = Math.imul(h, 16777619);
    }
    return h >>> 0;
  }
  function mulberry(seed: number): () => number {
    return () => {
      seed = (seed + 0x6d2b79f5) | 0;
      let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
      t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  }
  const r1 = (n: number) => Math.round(n * 10) / 10;

  const langColor = $derived.by(() => {
    const m = new Map<string, string>();
    for (const l of snapshot.languages) m.set(l.language, l.color);
    return m;
  });

  type Branch = {
    d: string;
    color: string;
    width: number;
    delay: number;
    repo: string;
    lang: string | null;
    net: number;
    commits: number;
  };
  type Leaf = { x: number; y: number; r: number; color: string; delay: number };
  type Dot = { x: number; y: number; size: number; color: string; delay: number };

  const layout = $derived.by(() => {
    const repos = snapshot.repos.filter((r) => r.added + r.removed > 0);
    const n = Math.max(1, repos.length);
    const maxChurn = repos.reduce((m, r) => Math.max(m, r.added + r.removed), 1);
    const maxCommits = repos.reduce((m, r) => Math.max(m, r.commits), 1);

    const branches: Branch[] = [];
    const leaves: Leaf[] = [];

    repos.forEach((repo, i) => {
      const rnd = mulberry(strHash(repo.fullName));
      const churn = repo.added + repo.removed;
      const cf = Math.sqrt(churn / maxChurn);
      const angle = (i / n) * TAU - Math.PI / 2 + (rnd() - 0.5) * (TAU / n) * 0.7;
      const len = AVATAR_R + 56 + cf * (MAX_LEN - AVATAR_R - 56);
      const width = 1.2 + cf * 6.5;
      const color = langColor.get(repo.topLanguage ?? "") ?? FALLBACK;

      const sx = Math.cos(angle) * AVATAR_R;
      const sy = Math.sin(angle) * AVATAR_R;
      const tx = Math.cos(angle) * len;
      const ty = Math.sin(angle) * len;
      const px = -Math.sin(angle);
      const py = Math.cos(angle);
      const sweep = (rnd() - 0.5) * len * 0.38;
      const cxp = (sx + tx) / 2 + px * sweep;
      const cyp = (sy + ty) / 2 + py * sweep;
      const delay = i * 6;

      branches.push({
        d: `M ${r1(sx)} ${r1(sy)} Q ${r1(cxp)} ${r1(cyp)} ${r1(tx)} ${r1(ty)}`,
        color,
        width,
        delay,
        repo: repo.fullName,
        lang: repo.topLanguage,
        net: repo.net,
        commits: repo.commits,
      });

      const commitFrac = repo.commits / maxCommits;
      const leafN = Math.max(1, Math.min(12, Math.round(1 + commitFrac * 11)));
      for (let k = 0; k < leafN; k++) {
        const spread = (k - (leafN - 1) / 2) * 0.16;
        const la = angle + spread + (rnd() - 0.5) * 0.05;
        const lr = len + 6 + rnd() * 16;
        leaves.push({
          x: Math.cos(la) * lr,
          y: Math.sin(la) * lr,
          r: 1.4 + cf * 2.2 + rnd() * 1.4,
          color,
          delay: delay + 260 + k * 16,
        });
      }
    });

    const cal = profile?.calendar ?? [];
    const m = Math.max(1, cal.length);
    const halo: Dot[] = cal.map((day, i) => {
      const a = (i / m) * TAU - Math.PI / 2;
      const active = day.count > 0;
      const rr = AVATAR_R + 9 + (active ? Math.min(day.count, 16) * 0.5 : 0);
      return {
        x: Math.cos(a) * rr,
        y: Math.sin(a) * rr,
        size: active ? 1.6 + Math.min(day.count, 16) * 0.12 : 1.1,
        color: active ? day.color : "#21262d",
        delay: 200 + i * 1.1,
      };
    });

    return { branches, leaves, halo };
  });

  // ---- pan + zoom ----
  let svgEl: SVGSVGElement;
  let scale = $state(1);
  let tx = $state(0);
  let ty = $state(0);
  let dragging = $state(false);
  let lastX = 0;
  let lastY = 0;
  // The center ring pulses once per click on the avatar (a quiet little touch),
  // never on a loop. Bumping the counter remounts the circle to replay it.
  let pulses = $state(0);
  function firePulse() {
    pulses += 1;
  }
  const transformed = $derived(scale !== 1 || tx !== 0 || ty !== 0);

  function clientToSvg(x: number, y: number) {
    const ctm = svgEl?.getScreenCTM();
    if (!ctm) return { x: 0, y: 0 };
    const p = new DOMPoint(x, y).matrixTransform(ctm.inverse());
    return { x: p.x, y: p.y };
  }
  function zoomAt(factor: number, cx: number, cy: number) {
    const ns = Math.min(6, Math.max(0.6, scale * factor));
    const k = ns / scale;
    tx = cx - k * (cx - tx);
    ty = cy - k * (cy - ty);
    scale = ns;
  }
  function onWheel(e: WheelEvent) {
    // Only intercept pinch (ctrl+wheel on macOS trackpads); a plain two-finger
    // swipe stays free to page between dashboard and tree.
    if (!e.ctrlKey) return;
    e.preventDefault();
    const c = clientToSvg(e.clientX, e.clientY);
    zoomAt(Math.exp(-e.deltaY * 0.01), c.x, c.y);
  }
  function onPointerDown(e: PointerEvent) {
    if (e.button !== 0) return;
    dragging = true;
    lastX = e.clientX;
    lastY = e.clientY;
    svgEl.setPointerCapture(e.pointerId);
  }
  function onPointerMove(e: PointerEvent) {
    if (!dragging) return;
    const ctm = svgEl.getScreenCTM();
    if (!ctm) return;
    tx += (e.clientX - lastX) / ctm.a;
    ty += (e.clientY - lastY) / ctm.d;
    lastX = e.clientX;
    lastY = e.clientY;
  }
  function onPointerUp(e: PointerEvent) {
    dragging = false;
    try {
      svgEl.releasePointerCapture(e.pointerId);
    } catch {
      /* pointer already released */
    }
  }
  function reset() {
    scale = 1;
    tx = 0;
    ty = 0;
  }

  onMount(() => {
    svgEl.addEventListener("wheel", onWheel, { passive: false });
    return () => svgEl.removeEventListener("wheel", onWheel);
  });

  // ---- screenshot export: a self-contained, high-res poster PNG ----
  let exporting = $state(false);
  let toast = $state<string | null>(null);
  let toastTimer: ReturnType<typeof setTimeout> | null = null;
  function flash(msg: string) {
    toast = msg;
    if (toastTimer) clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (toast = null), 3200);
  }

  const esc = (s: string) =>
    s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

  function buildPoster(): string {
    const W = 1600;
    const H = 1000;
    const cx = 1060;
    const cy = 500;
    const s = 0.84;
    const L = layout;

    const branches = L.branches
      .map(
        (b) =>
          `<path d="${b.d}" fill="none" stroke="${b.color}" stroke-width="${r1(b.width)}" stroke-linecap="round" opacity="0.92"/>`,
      )
      .join("");
    const leaves = L.leaves
      .map((lf) => `<circle cx="${r1(lf.x)}" cy="${r1(lf.y)}" r="${r1(lf.r)}" fill="${lf.color}"/>`)
      .join("");
    const halo = L.halo
      .map((h) => `<circle cx="${r1(h.x)}" cy="${r1(h.y)}" r="${r1(h.size)}" fill="${h.color}"/>`)
      .join("");

    const avatar = profile?.avatarDataUri
      ? `<clipPath id="pc"><circle cx="0" cy="0" r="${AVATAR_R}"/></clipPath>
         <image href="${profile.avatarDataUri}" x="${-AVATAR_R}" y="${-AVATAR_R}" width="${AVATAR_R * 2}" height="${AVATAR_R * 2}" clip-path="url(#pc)"/>`
      : `<circle cx="0" cy="0" r="${AVATAR_R}" fill="#21262d"/>`;

    const handle = esc(profile ? `@${profile.login}` : "Lineage");
    const big = commas(profile?.totalContributions ?? snapshot.summary.commits);
    const sum = snapshot.summary;
    const since = profile ? `LIFETIME CONTRIBUTIONS · PUBLIC + PRIVATE` : "LIFETIME CONTRIBUTIONS";
    const lines = `${signed(sum.net)} net lines`;
    const detail = `${commas(sum.commits)} commits · ${sum.repoCount} repos · ${sum.languageCount} languages`;
    const since2 = profile ? `since ${profile.createdYear}` : "";

    return `<svg xmlns="http://www.w3.org/2000/svg" width="${W}" height="${H}" viewBox="0 0 ${W} ${H}" font-family="${FONT}">
  <defs>
    <radialGradient id="vig" cx="66%" cy="50%" r="75%">
      <stop offset="0%" stop-color="#161b26"/>
      <stop offset="60%" stop-color="${BG}"/>
      <stop offset="100%" stop-color="#0a0c10"/>
    </radialGradient>
  </defs>
  <rect width="${W}" height="${H}" fill="url(#vig)"/>
  <g transform="translate(${cx} ${cy}) scale(${s})">
    ${branches}
    ${leaves}
    ${halo}
    <circle cx="0" cy="0" r="${AVATAR_R + 3}" fill="none" stroke="rgba(255,255,255,0.16)" stroke-width="2"/>
    ${avatar}
  </g>
  <g transform="translate(84 0)" fill="${TEXT}">
    <text x="0" y="356" font-size="34" font-weight="600">${handle}</text>
    <text x="-2" y="476" font-size="104" font-weight="800" font-family="${MONO}" letter-spacing="-2" style="font-feature-settings:'tnum' 1">${big}</text>
    <text x="0" y="520" font-size="20" font-weight="600" letter-spacing="2.6" fill="${DIM}">${since}</text>
    <text x="0" y="600" font-size="30" font-weight="600" font-family="${MONO}" style="font-feature-settings:'tnum' 1">
      <tspan fill="${ADD}">+${commas(sum.added)}</tspan><tspan fill="${DIM}">  </tspan><tspan fill="${REMOVE}">−${commas(sum.removed)}</tspan><tspan fill="${DIM}"> lines</tspan>
    </text>
    <text x="0" y="644" font-size="20" fill="${DIM}" font-family="${MONO}" style="font-feature-settings:'tnum' 1">${esc(detail)}</text>
    <text x="0" y="688" font-size="18" fill="${FAINT}">${esc(lines)} · ${esc(since2)}</text>
  </g>
  <text x="${W - 64}" y="${H - 60}" text-anchor="end" font-size="22" font-weight="700" letter-spacing="4" fill="${DIM}">MASTER<tspan fill="${ADD}"> +</tspan><tspan fill="${REMOVE}">−</tspan> DIFF</text>
</svg>`;
  }

  async function exportPng() {
    if (exporting) return;
    exporting = true;
    try {
      const svg = buildPoster();
      const blob = new Blob([svg], { type: "image/svg+xml;charset=utf-8" });
      const url = URL.createObjectURL(blob);
      const img = new Image();
      await new Promise<void>((resolve, reject) => {
        img.onload = () => resolve();
        img.onerror = () => reject(new Error("render failed"));
        img.src = url;
      });
      const px = 2;
      const canvas = document.createElement("canvas");
      canvas.width = 1600 * px;
      canvas.height = 1000 * px;
      const ctx = canvas.getContext("2d");
      if (!ctx) throw new Error("no canvas context");
      ctx.scale(px, px);
      ctx.drawImage(img, 0, 0, 1600, 1000);
      URL.revokeObjectURL(url);
      const b64 = canvas.toDataURL("image/png").split(",")[1];
      const path = await api.saveTreeImage(b64, profile?.login ?? "me");
      flash(`Saved to ${path.replace(/^.*\//, "")} on your Desktop`);
    } catch (e) {
      flash(e instanceof Error ? e.message : "Could not save image");
    } finally {
      exporting = false;
    }
  }
</script>

<div class="tree" class:dragging>
  <svg
    bind:this={svgEl}
    viewBox="-500 -500 1000 1000"
    preserveAspectRatio="xMidYMid meet"
    role="img"
    aria-label="Impact tree"
    onpointerdown={onPointerDown}
    onpointermove={onPointerMove}
    onpointerup={onPointerUp}
    onpointercancel={onPointerUp}
  >
    <g transform="translate({tx} {ty}) scale({scale})">
      <g class="branches">
        {#each layout.branches as b (b.repo)}
          <path
            class="branch"
            d={b.d}
            stroke={b.color}
            stroke-width={b.width}
            pathLength="100"
            style="--delay:{b.delay}ms"
          >
            <title>{b.repo} · {b.lang ?? "—"} · {signed(b.net)} · {b.commits} commits</title>
          </path>
        {/each}
      </g>

      <g class="leaves">
        {#each layout.leaves as lf, i (i)}
          <circle class="leaf" cx={lf.x} cy={lf.y} r={lf.r} fill={lf.color} style="--delay:{lf.delay}ms" />
        {/each}
      </g>

      <g class="halo">
        {#each layout.halo as h, i (i)}
          <circle class="hdot" cx={h.x} cy={h.y} r={h.size} fill={h.color} style="--delay:{h.delay}ms" />
        {/each}
      </g>

      {#if pulses > 0}
        {#key pulses}
          <circle class="pulse" cx="0" cy="0" r={AVATAR_R + 3} />
        {/key}
      {/if}
      <circle class="ring" cx="0" cy="0" r={AVATAR_R + 3} />
      <clipPath id="avatar-clip"><circle cx="0" cy="0" r={AVATAR_R} /></clipPath>
      {#if profile?.avatarDataUri}
        <image
          href={profile.avatarDataUri}
          x={-AVATAR_R}
          y={-AVATAR_R}
          width={AVATAR_R * 2}
          height={AVATAR_R * 2}
          clip-path="url(#avatar-clip)"
        />
      {:else}
        <circle cx="0" cy="0" r={AVATAR_R} fill="#21262d" />
        <text class="initial" x="0" y="2">{(profile?.login ?? "?").slice(0, 1).toUpperCase()}</text>
      {/if}
      <!-- Decorative: clicking the avatar just plays a one-off pulse, no action. -->
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <circle cx="0" cy="0" r={AVATAR_R} fill="transparent" onclick={firePulse} />
    </g>
  </svg>

  <div class="overlay">
    {#if profile}
      <div class="who">@{profile.login}</div>
      <div class="big mono">{commas(profile.totalContributions)}</div>
      <div class="label">lifetime contributions</div>
      <div class="label sub">public + private · since {profile.createdYear}</div>
    {:else}
      <div class="who">Your reach</div>
      <div class="label">loading contributions…</div>
    {/if}
    <div class="stats mono">
      <span class="add">+{commas(snapshot.summary.added)}</span>
      <span class="remove">−{commas(snapshot.summary.removed)}</span>
      <span class="muted">
        · {commas(snapshot.summary.commits)} commits · {snapshot.summary.repoCount} repos · {snapshot
          .summary.languageCount} languages
      </span>
    </div>
    <div class="hint">each branch is a repository, sized by lines written, colored by its language</div>
  </div>

  <div class="wordmark">MASTER<span class="add"> +</span><span class="remove">−</span> DIFF</div>

  <div class="controls">
    <button class="primary save" onclick={exportPng} disabled={exporting}>
      {exporting ? "Saving…" : "Save image"}
    </button>
    <div class="zoom">
      <button onclick={() => zoomAt(1 / 1.3, 0, 0)} title="Zoom out" aria-label="Zoom out">−</button>
      <button onclick={reset} class:dimmed={!transformed} title="Reset view" aria-label="Reset view">⌂</button>
      <button onclick={() => zoomAt(1.3, 0, 0)} title="Zoom in" aria-label="Zoom in">+</button>
    </div>
  </div>

  <div class="usehint">pinch to zoom · drag to pan</div>

  {#if toast}
    <div class="snackbar">{toast}</div>
  {/if}
</div>

<style>
  .tree {
    position: relative;
    width: 100%;
    height: 100%;
    min-height: 560px;
    overflow: hidden;
    background: radial-gradient(120% 90% at 62% 48%, #161b26 0%, var(--bg) 58%, #0a0c10 100%);
    cursor: grab;
  }
  .tree.dragging {
    cursor: grabbing;
  }
  svg {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    display: block;
    touch-action: none;
  }
  .branch {
    fill: none;
    stroke-linecap: round;
    opacity: 0.9;
    stroke-dasharray: 100;
    stroke-dashoffset: 100;
    animation: grow 0.85s cubic-bezier(0.33, 0, 0.2, 1) forwards;
    animation-delay: var(--delay);
    transition: filter 0.15s, opacity 0.15s;
  }
  .branch:hover {
    opacity: 1;
    filter: brightness(1.45);
  }
  @keyframes grow {
    to {
      stroke-dashoffset: 0;
    }
  }
  .leaf {
    opacity: 0;
    transform-box: fill-box;
    transform-origin: center;
    animation: pop 0.45s ease forwards;
    animation-delay: var(--delay);
  }
  @keyframes pop {
    0% {
      opacity: 0;
      transform: scale(0.2);
    }
    100% {
      opacity: 0.95;
      transform: scale(1);
    }
  }
  .hdot {
    opacity: 0;
    animation: fade 0.5s ease forwards;
    animation-delay: var(--delay);
  }
  @keyframes fade {
    to {
      opacity: 0.92;
    }
  }
  .ring {
    fill: none;
    stroke: rgba(255, 255, 255, 0.16);
    stroke-width: 2;
  }
  .pulse {
    fill: none;
    stroke: rgba(79, 140, 255, 0.5);
    stroke-width: 2;
    transform-box: fill-box;
    transform-origin: center;
    animation: pulse 0.7s ease-out;
  }
  @keyframes pulse {
    0% {
      transform: scale(1);
      opacity: 0.5;
    }
    70% {
      opacity: 0;
    }
    100% {
      transform: scale(1.9);
      opacity: 0;
    }
  }
  .initial {
    fill: var(--text);
    font-size: 44px;
    font-weight: 700;
    text-anchor: middle;
    dominant-baseline: central;
  }

  /* Overlay caption: the shareable headline, premium and consistent. */
  .overlay {
    position: absolute;
    top: 22px;
    left: 26px;
    pointer-events: none;
    text-shadow: 0 1px 16px var(--bg), 0 0 2px var(--bg);
  }
  .who {
    font-size: 15px;
    font-weight: 600;
    letter-spacing: -0.01em;
    margin-bottom: 8px;
  }
  .big {
    font-size: 46px;
    font-weight: 800;
    line-height: 1;
    letter-spacing: var(--track-display);
  }
  .label {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: var(--track-label);
    color: var(--text-dim);
    margin-top: 8px;
  }
  .label.sub {
    margin-top: 3px;
    color: var(--text-faint);
  }
  .stats {
    margin-top: 16px;
    display: flex;
    gap: 8px;
    align-items: baseline;
    flex-wrap: wrap;
    font-size: 13px;
    max-width: 360px;
  }
  .stats .muted {
    color: var(--text-dim);
  }
  .hint {
    margin-top: 12px;
    font-size: 11px;
    line-height: 1.5;
    color: var(--text-faint);
    max-width: 300px;
  }

  .wordmark {
    position: absolute;
    right: 22px;
    bottom: 18px;
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.22em;
    color: var(--text-faint);
    pointer-events: none;
    user-select: none;
  }

  /* Controls: subtle by default so manual screenshots stay clean; the export
     poster never includes them. */
  .controls {
    position: absolute;
    top: 18px;
    right: 20px;
    display: flex;
    align-items: center;
    gap: 10px;
    opacity: 0.35;
    transition: opacity 0.18s;
  }
  .tree:hover .controls {
    opacity: 1;
  }
  .controls .save {
    font-size: 13px;
    font-weight: 600;
    padding: 7px 14px;
  }
  .zoom {
    display: flex;
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    border-radius: 9px;
    overflow: hidden;
  }
  .zoom button {
    border: none;
    border-radius: 0;
    background: transparent;
    width: 34px;
    height: 32px;
    padding: 0;
    font-size: 16px;
    line-height: 1;
    color: var(--text);
  }
  .zoom button + button {
    border-left: 1px solid var(--border);
  }
  .zoom button.dimmed {
    color: var(--text-faint);
  }

  .usehint {
    position: absolute;
    bottom: 18px;
    left: 26px;
    font-size: 11px;
    letter-spacing: 0.04em;
    color: var(--text-faint);
    pointer-events: none;
    opacity: 0.5;
  }

  .snackbar {
    position: absolute;
    bottom: 46px;
    left: 50%;
    transform: translateX(-50%);
    background: var(--bg-elev-2);
    border: 1px solid var(--border);
    color: var(--text);
    font-size: 13px;
    padding: 9px 16px;
    border-radius: 10px;
    box-shadow: 0 8px 28px rgba(0, 0, 0, 0.45);
    animation: rise 0.25s ease;
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translate(-50%, 8px);
    }
    to {
      opacity: 1;
      transform: translate(-50%, 0);
    }
  }
</style>
