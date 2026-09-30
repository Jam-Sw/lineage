<script lang="ts">
  import { onMount } from "svelte";
  import type { Snapshot, ProfileStats, PathNode } from "$lib/api/types";
  import { commas, signed } from "$lib/format";
  import * as api from "$lib/api/client";
  import { repoShortName, trailKey } from "$lib/tree-diff";

  let {
    snapshot,
    profile,
    grownTrails = new Set<string>(),
    surge = 0,
  }: {
    snapshot: Snapshot;
    profile: ProfileStats | null;
    /** Trails (language/repo/.../file) that grew since the tree was last seen. */
    grownTrails?: Set<string>;
    /** Bumped by the page to play the energy wave; the tree only watches it. */
    surge?: number;
  } = $props();

  const TAU = Math.PI * 2;
  const AVATAR_R = 54;
  const FALLBACK = "#8b949e";
  const BG = "#0f1115";
  const DIM = "#9aa3b2";
  const FAINT = "#6b7280";
  const TEXT = "#e6e9ef";
  const ADD = "#3fb950";
  const REMOVE = "#f85149";
  const FONT = "-apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif";
  const MONO = "ui-monospace, SFMono-Regular, 'SF Mono', Menlo, monospace";

  const r1 = (n: number) => Math.round(n * 10) / 10;

  const langColor = $derived.by(() => {
    const m = new Map<string, string>();
    for (const l of snapshot.languages) m.set(l.language, l.color);
    return m;
  });

  // The tree is one hierarchy drawn the same way at every level:
  //   you -> language -> repository -> folder -> subfolder -> file.
  // Each level branches from its parent's tip, fanned by share of lines, with
  // width split by da Vinci's rule (children's cross-sections sum to the
  // parent's). Deeper levels are small, so they only draw once you zoom in far
  // enough to see them: the finer structure opens up as you go deeper.
  type Src = {
    name: string;
    kind: "lang" | "repo" | "path";
    added: number;
    removed: number;
    language: string;
    commits: number;
    children: Src[];
  };
  type Node = {
    id: number;
    parent: number;
    /** Index one past this node's last descendant (nodes are in pre-order). */
    end: number;
    depth: number;
    kind: Src["kind"];
    name: string;
    trail: string[];
    added: number;
    removed: number;
    color: string;
    d: string;
    width: number;
    len: number;
    ang: number;
    x: number;
    y: number;
    leaf: boolean;
    /** Bounds of this node and everything below it. */
    box: [number, number, number, number];
    delay: number;
    /** True when this limb grew since the tree was last looked at. */
    grown: boolean;
    /** When the energy wave reaches this limb, marching out from the centre. */
    waveDelay: number;
  };
  type Label = { x: number; y: number; text: string; anchor: "start" | "end"; delay: number };
  type Dot = { x: number; y: number; size: number; color: string; delay: number };

  const INNER = AVATAR_R + 26;
  const LANG_LEN = 120;
  const SWIRL = 0.16;
  const PATH_SPREAD = 1.5;

  function fromPath(p: PathNode): Src {
    return {
      name: p.name,
      kind: "path",
      added: p.added,
      removed: p.removed,
      language: p.language,
      commits: 0,
      children: (p.children ?? []).map(fromPath),
    };
  }

  // Largest child in the middle of the fan, the rest alternating outward, so
  // every fan is balanced and the heavy limb continues its parent's line.
  function fanOrder<T>(sorted: T[]): T[] {
    const out: T[] = [];
    sorted.forEach((c, i) => (i % 2 ? out.unshift(c) : out.push(c)));
    return out;
  }
  const churnOf = (s: { added: number; removed: number }) => s.added + s.removed;

  const layout = $derived.by(() => {
    const repos = snapshot.repos.filter((r) => churnOf(r) > 0);
    const maxCommits = repos.reduce((m, r) => Math.max(m, r.commits), 1);
    const byLang = new Map<string, Src>();
    for (const r of repos) {
      const lang = r.topLanguage ?? "Other";
      const g = byLang.get(lang) ?? { name: lang, kind: "lang", added: 0, removed: 0, language: lang, commits: 0, children: [] };
      g.added += r.added;
      g.removed += r.removed;
      g.commits += r.commits;
      g.children.push({
        name: repoShortName(r.fullName),
        kind: "repo",
        added: r.added,
        removed: r.removed,
        language: lang,
        commits: r.commits,
        children: (r.tree?.children ?? []).map(fromPath),
      });
      byLang.set(lang, g);
    }
    const langs = [...byLang.values()].sort((x, y) => churnOf(y) - churnOf(x));
    const total = langs.reduce((t, g) => t + churnOf(g), 0) || 1;

    let nodes: Node[] = [];
    let order = 0;
    // Lengths are relative; `fit` scales them so the whole tree fills the
    // frame at the default zoom (measured on a first pass, applied on a second).
    let fit = 1;

    function grow(
      src: Src,
      parent: number,
      depth: number,
      trail: string[],
      sx: number,
      sy: number,
      dir: number,
      len: number,
      width: number,
      spread: number,
    ): Node {
      const color = langColor.get(src.language) ?? FALLBACK;
      // A gentle, consistent bend (the same way at every level) so fans read
      // as growth rather than spokes.
      const bend = dir + SWIRL;
      const reach = len * fit;
      const x = sx + Math.cos(bend) * reach;
      const y = sy + Math.sin(bend) * reach;
      const qx = sx + Math.cos(dir) * reach * 0.55;
      const qy = sy + Math.sin(dir) * reach * 0.55;
      const here = [...trail, src.name];
      const pad = width / 2 + 2;
      const node: Node = {
        id: nodes.length,
        parent,
        end: 0,
        depth,
        kind: src.kind,
        name: src.name,
        trail: here,
        added: src.added,
        removed: src.removed,
        color,
        d: `M ${r1(sx)} ${r1(sy)} Q ${r1(qx)} ${r1(qy)} ${r1(x)} ${r1(y)}`,
        width,
        len: reach,
        ang: bend,
        x,
        y,
        leaf: src.children.length === 0,
        box: [Math.min(sx, x) - pad, Math.min(sy, y) - pad, Math.max(sx, x) + pad, Math.max(sy, y) + pad],
        delay: depth <= 2 ? 150 + depth * 420 + (order++ % 60) * 9 : 0,
        grown: grownTrails.has(trailKey(here)),
        waveDelay: depth * 240 + (nodes.length % 7) * 30,
      };
      nodes.push(node);

      const kids = [...src.children].sort((p, q) => churnOf(q) - churnOf(p));
      const sum = kids.reduce((t, k) => t + churnOf(k), 0) || 1;
      const fan = fanOrder(kids);
      // Slots are sized by share, with a floor so a tiny limb still has room.
      const floor = 0.3 / Math.max(kids.length, 1);
      const slots = fan.map((k) => Math.max(churnOf(k) / sum, floor));
      const slotSum = slots.reduce((t, w) => t + w, 0) || 1;
      let a = bend - spread / 2;
      fan.forEach((k, i) => {
        const span = (slots[i] / slotSum) * spread;
        const share = churnOf(k) / sum;
        const childDir = a + span / 2;
        a += span;
        let childLen: number;
        let childSpread: number;
        if (k.kind === "repo") {
          // Repositories: length is commits, so long-lived work reaches further.
          const kf = Math.log1p(k.commits) / Math.log1p(maxCommits);
          childLen = 55 + kf * 125;
          childSpread = PATH_SPREAD * 1.05;
        } else {
          childLen = len * (0.5 + 0.28 * Math.sqrt(share));
          childSpread = PATH_SPREAD;
        }
        const child = grow(k, node.id, depth + 1, here, x, y, childDir, childLen, Math.max(0.2, width * Math.sqrt(share)), childSpread);
        node.box = [
          Math.min(node.box[0], child.box[0]),
          Math.min(node.box[1], child.box[1]),
          Math.max(node.box[2], child.box[2]),
          Math.max(node.box[3], child.box[3]),
        ];
      });
      node.end = nodes.length;
      return node;
    }

    // Languages share the full circle by lines, largest first from 12 o'clock.
    const GAP = 0.03;
    const usable = TAU - GAP * langs.length;
    const plant = () => {
    nodes = [];
    order = 0;
    let cursor = -Math.PI / 2;
    for (const g of langs) {
      const share = churnOf(g) / total;
      const span = Math.max(share * usable, 0.06);
      const mid = cursor + GAP / 2 + span / 2;
      grow(
        g,
        -1,
        0,
        [],
        Math.cos(mid) * INNER,
        Math.sin(mid) * INNER,
        mid,
        LANG_LEN * (0.7 + 0.3 * Math.sqrt(share)),
        3 + 15 * Math.sqrt(share),
        Math.min(1.7, Math.max(0.6, span * 1.6)),
      );
      cursor += span + GAP;
    }
    };
    plant();
    const extent = nodes.reduce((m, n) => Math.max(m, Math.abs(n.box[0]), Math.abs(n.box[1]), Math.abs(n.box[2]), Math.abs(n.box[3])), 1);
    fit = Math.min(1, 470 / extent);
    plant();

    // Poster labels: the biggest repositories.
    const labels: Label[] = nodes
      .filter((n) => n.kind === "repo")
      .sort((p, q) => churnOf(q) - churnOf(p))
      .slice(0, 6)
      .map((n) => {
        const dist = Math.hypot(n.x, n.y) || 1;
        return {
          x: n.x + (n.x / dist) * 12,
          y: n.y + (n.y / dist) * 12 + 4,
          text: n.name,
          anchor: n.x >= 0 ? "start" : "end",
          delay: 0,
        };
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

    return { nodes, labels, halo };
  });

  // ---- pan + zoom ----
  // View state is written at most once per animation frame. Pointer and wheel
  // events only accumulate into a pending target, so a burst of 120Hz events
  // costs one SVG repaint, not five.
  let svgEl: SVGSVGElement;
  let scale = $state(1);
  let tx = $state(0);
  let ty = $state(0);
  let dragging = $state(false);
  // The center ring pulses once per click on the avatar (a quiet little touch),
  // never on a loop. Bumping the counter remounts the circle to replay it.
  let pulses = $state(0);
  // The growth wave follows the page's `surge` token, and an avatar tap adds a
  // local bump so the session's growth can be replayed by hand.
  let replay = $state(0);
  const wave = $derived(surge + replay);
  function firePulse() {
    pulses += 1;
    replay += 1;
  }
  const transformed = $derived(scale !== 1 || tx !== 0 || ty !== 0);

  const MIN_SCALE = 0.6;
  // Deep enough to read a file-level twig of the largest repo.
  const MAX_SCALE = 90;
  const reduceMotion =
    typeof matchMedia !== "undefined" && matchMedia("(prefers-reduced-motion: reduce)").matches;

  let frame = 0;
  let pendingDx = 0;
  let pendingDy = 0;
  let vx = 0; // svg units per ms, for the release glide
  let vy = 0;
  let lastX = 0;
  let lastY = 0;
  let lastT = 0;
  let downX = 0;
  let downY = 0;
  let pointerId: number | null = null;
  let tween: { s: number; x: number; y: number; t0: number; from: { s: number; x: number; y: number } } | null =
    null;
  let glideT = 0;
  let idle: ReturnType<typeof setTimeout> | undefined;

  // What is drawn depends on the view, but recomputing it every frame would
  // churn thousands of elements. `cull` is a snapshot of the view that only
  // moves when the real view has drifted a meaningful amount (or settles).
  let size = $state({ w: 1000, h: 1000 });
  let cull = $state({ s: 1, x: 0, y: 0 });
  // svg units -> screen px at scale 1 (the viewBox is 1000 units, "meet").
  const ppu = () => Math.min(size.w, size.h) / 1000;
  function settle() {
    if (cull.s !== scale || cull.x !== tx || cull.y !== ty) cull = { s: scale, x: tx, y: ty };
  }
  function drifted() {
    const zoomed = Math.abs(Math.log2(scale / cull.s)) > 0.35;
    const panned = Math.hypot(tx - cull.x, ty - cull.y) * ppu() > Math.min(size.w, size.h) * 0.3;
    return zoomed || panned;
  }

  // Draw a node when it is long and thick enough on screen to see, and its
  // subtree is near the viewport. Nodes are pre-order, so a rejected node
  // skips its whole subtree (every descendant is shorter).
  const MIN_PX = 8;
  const MIN_WIDTH_PX = 0.3;
  const visible = $derived.by(() => {
    const nodes = layout.nodes;
    const k = cull.s * ppu();
    // Viewport in tree units, padded by half a screen each way so panning
    // reveals already-drawn branches.
    const hw = size.w / k;
    const hh = size.h / k;
    const cx = -cull.x / cull.s;
    const cy = -cull.y / cull.s;
    const out: Node[] = [];
    let i = 0;
    while (i < nodes.length) {
      const n = nodes[i];
      const [x0, y0, x1, y1] = n.box;
      const onScreen = x1 > cx - hw && x0 < cx + hw && y1 > cy - hh && y0 < cy + hh;
      const tooSmall = n.len * k < MIN_PX || n.width * k < MIN_WIDTH_PX;
      if (!onScreen || (n.depth > 1 && tooSmall)) {
        i = n.end;
        continue;
      }
      out.push(n);
      i += 1;
    }
    return out;
  });
  // Tips: files and other ends of the line, filled when net-positive.
  const tips = $derived(visible.filter((n) => n.leaf));
  // The limbs that grew since the tree was last seen: the wave rides these and
  // the glow keeps them softly lit for the session.
  const grownVisible = $derived(visible.filter((n) => n.grown));
  const grownTips = $derived(grownVisible.filter((n) => n.leaf));
  const fontUnits = $derived(12 / (cull.s * ppu()));
  // Names appear once a limb is long enough on screen to carry one. Shallow
  // levels win (languages, then repositories, then folders), and a name that
  // would overlap one already placed is skipped rather than stacked.
  const names = $derived.by(() => {
    const k = cull.s * ppu();
    const f = fontUnits;
    const placed: [number, number, number, number][] = [];
    const out: Node[] = [];
    const ranked = visible
      .filter((n) => n.len * k > 60)
      .sort((p, q) => p.depth - q.depth || churnOf(q) - churnOf(p));
    for (const n of ranked) {
      const w = n.name.length * f * 0.6;
      const right = Math.cos(n.ang) >= 0;
      const x0 = right ? n.x : n.x - w;
      const box: [number, number, number, number] = [x0 - f * 0.3, n.y - f * 0.8, x0 + w + f * 0.3, n.y + f * 0.8];
      if (placed.some((b) => box[0] < b[2] && box[2] > b[0] && box[1] < b[3] && box[3] > b[1])) continue;
      placed.push(box);
      out.push(n);
      if (out.length >= 28) break;
    }
    return out;
  });

  // Hovering a limb lights its whole lineage back to you.
  let hovered = $state<Node | null>(null);
  const lineage = $derived.by(() => {
    const set = new Set<number>();
    for (let n = hovered; n; n = n.parent >= 0 ? layout.nodes[n.parent] : null) set.add(n.id);
    return set;
  });

  // Hover is resolved once at the svg, from whatever is under the pointer.
  // Per-branch enter/leave is unreliable on thin strokes and never fires for
  // a branch that culling removes, which left highlights stuck on.
  function onHover(e: PointerEvent) {
    if (dragging) return;
    const id = (e.target as Element).closest?.(".branch")?.getAttribute("data-id");
    const n = id == null ? null : (layout.nodes[Number(id)] ?? null);
    if (n !== hovered) hovered = n;
  }

  function clientToSvg(x: number, y: number) {
    const ctm = svgEl?.getScreenCTM();
    if (!ctm) return { x: 0, y: 0 };
    const p = new DOMPoint(x, y).matrixTransform(ctm.inverse());
    return { x: p.x, y: p.y };
  }
  function pxToSvg() {
    return svgEl?.getScreenCTM()?.a || 1;
  }
  function schedule() {
    if (!frame) frame = requestAnimationFrame(tick);
  }
  function stopMotion() {
    tween = null;
    vx = vy = 0;
  }

  function tick(now: number) {
    frame = 0;
    let again = false;
    if (pendingDx || pendingDy) {
      tx += pendingDx;
      ty += pendingDy;
      pendingDx = pendingDy = 0;
    }
    if (tween) {
      const k = Math.min(1, (now - tween.t0) / 260);
      const e = 1 - Math.pow(1 - k, 3);
      scale = tween.from.s + (tween.s - tween.from.s) * e;
      tx = tween.from.x + (tween.x - tween.from.x) * e;
      ty = tween.from.y + (tween.y - tween.from.y) * e;
      if (k < 1) again = true;
      else tween = null;
    } else if (!dragging && (vx || vy)) {
      // Release glide: exponential decay, frame-rate independent.
      const dt = Math.min(48, now - (glideT || now - 16));
      glideT = now;
      tx += vx * dt;
      ty += vy * dt;
      const decay = Math.pow(0.994, dt);
      vx *= decay;
      vy *= decay;
      if (Math.hypot(vx, vy) * pxToSvg() < 0.02) vx = vy = 0;
      else again = true;
    }
    // Redraw detail when the view has moved far, and once it comes to rest.
    if (drifted()) settle();
    clearTimeout(idle);
    idle = setTimeout(settle, 140);
    if (again) schedule();
  }

  function animateTo(target: { s: number; x: number; y: number }, animate = true) {
    vx = vy = 0;
    if (!animate || reduceMotion) {
      tween = null;
      scale = target.s;
      tx = target.x;
      ty = target.y;
      schedule();
      return;
    }
    tween = { ...target, t0: performance.now(), from: { s: scale, x: tx, y: ty } };
    schedule();
  }
  // Zoom about a point given in svg coordinates. Animated for discrete steps
  // (buttons, double-click); immediate for continuous pinch.
  function zoomAt(factor: number, cx: number, cy: number, animate = true) {
    const base = tween ? { s: tween.s, x: tween.x, y: tween.y } : { s: scale, x: tx, y: ty };
    const ns = Math.min(MAX_SCALE, Math.max(MIN_SCALE, base.s * factor));
    const k = ns / base.s;
    animateTo({ s: ns, x: cx - k * (cx - base.x), y: cy - k * (cy - base.y) }, animate);
  }
  // Frame a node and everything that grows from it.
  function dive(n: Node) {
    const [x0, y0, x1, y1] = n.box;
    const fit = 0.86 * Math.min(size.w / ppu() / (x1 - x0), size.h / ppu() / (y1 - y0));
    const ns = Math.min(MAX_SCALE, Math.max(MIN_SCALE, fit));
    animateTo({ s: ns, x: -((x0 + x1) / 2) * ns, y: -((y0 + y1) / 2) * ns });
  }
  function reset() {
    animateTo({ s: 1, x: 0, y: 0 });
  }

  function onWheel(e: WheelEvent) {
    // Only intercept pinch (ctrl+wheel on macOS trackpads); a plain two-finger
    // swipe stays free to page between dashboard and tree.
    if (!e.ctrlKey) return;
    e.preventDefault();
    const c = clientToSvg(e.clientX, e.clientY);
    zoomAt(Math.exp(-e.deltaY * 0.01), c.x, c.y, false);
  }
  function onDblClick(e: MouseEvent) {
    const c = clientToSvg(e.clientX, e.clientY);
    zoomAt(e.shiftKey ? 1 / 2 : 2, c.x, c.y);
  }
  function onPointerDown(e: PointerEvent) {
    if (e.button !== 0 || pointerId !== null) return;
    // Without this WebKit starts a text selection across the labels mid-drag.
    e.preventDefault();
    stopMotion();
    pointerId = e.pointerId;
    lastX = downX = e.clientX;
    lastY = downY = e.clientY;
    lastT = e.timeStamp;
  }
  function onPointerMove(e: PointerEvent) {
    if (e.pointerId !== pointerId) return;
    // A few pixels of slop so a click on the avatar stays a click.
    if (!dragging) {
      if (Math.hypot(e.clientX - downX, e.clientY - downY) < 4) return;
      dragging = true;
      hovered = null;
      svgEl.setPointerCapture(e.pointerId);
    }
    const u = 1 / pxToSvg();
    const dx = (e.clientX - lastX) * u;
    const dy = (e.clientY - lastY) * u;
    const dt = Math.max(1, e.timeStamp - lastT);
    // Smoothed velocity, so the glide follows the flick rather than the last jitter.
    vx = vx * 0.6 + (dx / dt) * 0.4;
    vy = vy * 0.6 + (dy / dt) * 0.4;
    pendingDx += dx;
    pendingDy += dy;
    lastX = e.clientX;
    lastY = e.clientY;
    lastT = e.timeStamp;
    schedule();
  }
  function onPointerUp(e: PointerEvent) {
    if (e.pointerId !== pointerId) return;
    pointerId = null;
    if (!dragging) return;
    dragging = false;
    if (svgEl.hasPointerCapture(e.pointerId)) svgEl.releasePointerCapture(e.pointerId);
    // Holding still before letting go means "put it here", not "throw it".
    if (e.timeStamp - lastT > 60 || reduceMotion || e.type === "pointercancel") vx = vy = 0;
    glideT = 0;
    schedule();
  }

  onMount(() => {
    svgEl.addEventListener("wheel", onWheel, { passive: false });
    const ro = new ResizeObserver(([e]) => {
      size = { w: e.contentRect.width || 1000, h: e.contentRect.height || 1000 };
    });
    ro.observe(svgEl);
    return () => {
      svgEl.removeEventListener("wheel", onWheel);
      ro.disconnect();
      cancelAnimationFrame(frame);
      clearTimeout(idle);
    };
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

    // The poster is a still, so it draws every limb that would be at least
    // half a pixel long at its size.
    const drawn = L.nodes.filter((n) => n.len * s >= 1.5);
    const branches = drawn
      .map(
        (n) =>
          `<path d="${n.d}" fill="none" stroke="${n.color}" stroke-width="${Math.max(0.35, r1(n.width))}" stroke-linecap="round" opacity="0.92"/>`,
      )
      .join("");
    const tips = drawn
      .filter((n) => n.leaf)
      .map((n) => {
        const r = r1(Math.max(n.width * 0.9, 1.2));
        return n.added >= n.removed
          ? `<circle cx="${r1(n.x)}" cy="${r1(n.y)}" r="${r}" fill="${n.color}"/>`
          : `<circle cx="${r1(n.x)}" cy="${r1(n.y)}" r="${r}" fill="${BG}" stroke="${n.color}" stroke-width="${r1(r * 0.4)}"/>`;
      })
      .join("");
    const labels = L.labels
      .map(
        (l) =>
          `<text x="${r1(l.x)}" y="${r1(l.y)}" text-anchor="${l.anchor}" font-size="15" font-weight="600" fill="${TEXT}">${esc(l.text)}</text>`,
      )
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
    ${tips}
    ${halo}
    ${labels}
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
      const path = await api.saveTreeImage(buildPoster(), profile?.login ?? "me");
      flash(`Saved to ${path.replace(/^.*\//, "")} on your Desktop`);
    } catch (e) {
      flash(e instanceof Error ? e.message : "Could not save image");
    } finally {
      exporting = false;
    }
  }
</script>

<div class="tree" class:dragging class:hovering={hovered !== null}>
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
    onpointerover={onHover}
    onpointerleave={() => (hovered = null)}
    ondblclick={onDblClick}
  >
    <g transform="translate({tx} {ty}) scale({scale})">
      {#if grownTrails.size > 0}
        <!-- soft aura -->
        <g class="glowwrap">
          <g class="glow" class:still={reduceMotion}>
            {#each grownVisible as n (n.id)}
              <path
                class="glowpath"
                d={n.d}
                stroke={n.color}
                stroke-width={Math.max(n.width * 2.2, 2.2 / (cull.s * ppu()))}
              />
            {/each}
            {#each grownTips as n (n.id)}
              {@const r = Math.max(n.width * 1.4, 2.4 / (cull.s * ppu()))}
              <circle class="glowtip" cx={n.x} cy={n.y} {r} fill={n.color} />
            {/each}
          </g>
        </g>
      {/if}

      <g class="branches">
        {#each visible as n (n.id)}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <path
            class="branch"
            class:lit={lineage.has(n.id)}
            d={n.d}
            stroke={n.color}
            stroke-width={Math.max(n.width, 0.8 / (cull.s * ppu()))}
            pathLength="100"
            style="--delay:{n.delay}ms"
            data-id={n.id}
            onclick={() => dive(n)}
          />
        {/each}
      </g>

      <g class="tips">
        {#each tips as n (n.id)}
          {@const r = Math.max(n.width * 0.9, 1.6 / (cull.s * ppu()))}
          <circle
            class="tip"
            class:lit={lineage.has(n.id)}
            cx={n.x}
            cy={n.y}
            {r}
            fill={n.added >= n.removed ? n.color : "var(--bg)"}
            stroke={n.color}
            stroke-width={n.added >= n.removed ? 0 : r * 0.4}
            style="--delay:{n.delay + 600}ms"
          />
        {/each}
      </g>

      {#if wave > 0 && !reduceMotion}
        <!-- The energy wave: a comet rides out from the centre along every
             grown limb, in depth order, and bursts where the new lines landed.
             Keyed by the counter so a new token replays it from scratch. -->
        {#key wave}
          <g class="wave">
            {#each grownVisible as n (n.id)}
              <path
                class="comet halo"
                d={n.d}
                pathLength="100"
                style="--wd:{n.waveDelay}ms; stroke-width:{r1(Math.max(4.4 / (cull.s * ppu()), 2))}"
              />
              <path
                class="comet core"
                d={n.d}
                pathLength="100"
                style="--wd:{n.waveDelay}ms; stroke-width:{r1(Math.max(1.5 / (cull.s * ppu()), 0.8))}"
              />
            {/each}
            {#each grownTips as n (n.id)}
              {@const r = Math.max(n.width * 0.9, 1.6 / (cull.s * ppu()))}
              <circle class="burst" cx={n.x} cy={n.y} {r} style="--wd:{n.waveDelay + 420}ms" />
            {/each}
          </g>
        {/key}
      {/if}

      <g class="halo">
        {#each layout.halo as h, i (i)}
          <circle class="hdot" cx={h.x} cy={h.y} r={h.size} fill={h.color} style="--delay:{h.delay}ms" />
        {/each}
      </g>

      <g class="labels" style="font-size:{fontUnits}px; stroke-width:{fontUnits * 0.3}px">
        {#each names as n (n.id)}
          {@const right = Math.cos(n.ang) >= 0}
          <text
            class="rlabel"
            class:lit={lineage.has(n.id)}
            x={n.x + Math.cos(n.ang) * fontUnits * 0.8}
            y={n.y + Math.sin(n.ang) * fontUnits * 0.8 + fontUnits * 0.35}
            text-anchor={right ? "start" : "end"}
            style="--delay:{n.delay + 700}ms">{n.name}</text
          >
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
      <circle cx="0" cy="0" r={AVATAR_R} fill="transparent" onclick={() => (firePulse(), reset())} />
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
    <div class="hint"></div>
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

  {#if hovered}
    <div class="readout">
      <div class="trail">
        {#each hovered.trail as part, i (i)}{#if i > 0}<span class="sep">/</span>{/if}<span
            class:leafname={i === hovered.trail.length - 1}>{part}</span
          >{/each}
      </div>
      <div class="mono nums">
        <span class="add">+{commas(hovered.added)}</span>
        <span class="remove">−{commas(hovered.removed)}</span>
        <span class="muted">· {Math.round((churnOf(hovered) / Math.max(1, snapshot.summary.added + snapshot.summary.removed)) * 1000) / 10}% of everything you've written</span>
      </div>
    </div>
  {:else}
    <div class="usehint">Explore</div>
  {/if}

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
  /* Nothing in the tree is text to select; dragging must never start a selection. */
  .tree,
  .tree * {
    -webkit-user-select: none;
    user-select: none;
    -webkit-user-drag: none;
  }
  /* Mid-drag, branches must not run hover filters as the cursor sweeps over them. */
  .tree.dragging svg * {
    pointer-events: none;
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
    transition: opacity 0.25s ease;
  }
  .branch {
    cursor: pointer;
  }
  /* Hovering lights one lineage, you -> language -> repo -> ... -> this, and
     sinks everything else. Opacity only: cheap to repaint. */
  .tree.hovering .branch {
    opacity: 0.22;
  }
  .tree.hovering .branch.lit {
    opacity: 1;
  }
  /* Tips and names fade in with a filled-forward animation that owns their
     opacity, so they dim through fill/stroke opacity instead. */
  .tip,
  .rlabel {
    transition: fill-opacity 0.25s ease, stroke-opacity 0.25s ease;
  }
  .tree.hovering .tip:not(.lit) {
    fill-opacity: 0.22;
    stroke-opacity: 0.22;
  }
  .tree.hovering .rlabel:not(.lit) {
    fill-opacity: 0.35;
  }
  @keyframes grow {
    to {
      stroke-dashoffset: 0;
    }
  }
  /* Size and halo width come from the group, in tree units, so names stay a
     constant size on screen at any zoom. */
  .rlabel {
    fill: var(--text);
    font-weight: 600;
    paint-order: stroke;
    stroke: var(--bg);
    stroke-linejoin: round;
    opacity: 0;
    animation: fade 0.5s ease forwards;
    animation-delay: var(--delay);
    pointer-events: none;
  }
  .tip {
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
  /* The growth aura: a soft underlay where the limbs grew, so they stay easy
     to spot while panning and zooming. It breathes; hover dims it so the
     lineage highlight stays in charge. */
  .glowwrap {
    transition: opacity 0.25s ease;
    /* The aura must never intercept the pointer: clicks still dive the limb
       underneath and hover still resolves to the branch. */
    pointer-events: none;
  }
  .tree.hovering .glowwrap {
    opacity: 0.3;
  }
  .glow {
    animation: breathe 3.6s ease-in-out infinite;
  }
  .glow.still {
    animation: none;
    opacity: 0.22;
  }
  @keyframes breathe {
    0%,
    100% {
      opacity: 0.16;
    }
    50% {
      opacity: 0.42;
    }
  }
  .glowpath {
    fill: none;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .glowtip {
    opacity: 0.7;
  }
  /* The energy wave: a white comet sweeping each grown limb tip-ward, then a
     burst where the new lines landed. The dash is a 22-unit streak slid along
     the path (pathLength=100), delayed per limb so the wave marches out from
     the centre. */
  .wave {
    pointer-events: none;
  }
  .comet {
    fill: none;
    stroke: #fff;
    stroke-linecap: round;
    stroke-dasharray: 22 100;
    stroke-dashoffset: 22;
    animation: travel 0.55s cubic-bezier(0.3, 0, 0.7, 1) var(--wd, 0ms) forwards;
  }
  .comet.halo {
    opacity: 0.3;
  }
  .comet.core {
    opacity: 0.95;
  }
  @keyframes travel {
    to {
      stroke-dashoffset: -100;
    }
  }
  .burst {
    fill: none;
    stroke: #fff;
    transform-box: fill-box;
    transform-origin: center;
    opacity: 0;
    animation: burst 0.6s ease-out var(--wd, 0ms) forwards;
  }
  @keyframes burst {
    0% {
      opacity: 0;
      transform: scale(0.4);
    }
    35% {
      opacity: 0.9;
    }
    100% {
      opacity: 0;
      transform: scale(2.6);
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
    transition: opacity 0.25s ease;
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

  .readout {
    position: absolute;
    bottom: 16px;
    left: 26px;
    right: 220px;
    pointer-events: none;
    text-shadow: 0 1px 12px var(--bg);
  }
  .readout .trail {
    font-size: 14px;
    color: var(--text-dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .readout .sep {
    margin: 0 5px;
    color: var(--text-faint);
  }
  .readout .leafname {
    color: var(--text);
    font-weight: 600;
  }
  .readout .nums {
    margin-top: 3px;
    font-size: 12px;
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
  @media (prefers-reduced-motion: reduce) {
    .branch,
    .tip,
    .hdot,
    .rlabel {
      animation-duration: 1ms;
      animation-delay: 0ms;
    }
  }
</style>
