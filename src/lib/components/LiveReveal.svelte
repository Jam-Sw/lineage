<script lang="ts">
  import { untrack } from "svelte";
  import type { LanguageStat } from "$lib/api/types";
  import LanguageBars from "./LanguageBars.svelte";
  import { commas, signed } from "$lib/format";

  type FeedItem = {
    repo: string;
    added: number;
    removed: number;
    lang: string | null;
    cached: boolean;
  };

  let {
    net,
    added,
    removed,
    commits,
    done,
    total,
    languages,
    barStyle = "language",
    message = "Starting sync…",
    feed,
  }: {
    net: number;
    added: number;
    removed: number;
    commits: number;
    done: number;
    total: number;
    languages: LanguageStat[];
    barStyle?: string;
    message?: string;
    feed: FeedItem[];
  } = $props();

  // Animated count-up toward the running totals. Repos arrive one tick at a
  // time and a slow clone can be seconds apart, so a fixed-duration count-up
  // would reach the number and then sit frozen until the next repo. Instead we
  // measure the real gap between ticks and stretch each count-up across (a touch
  // beyond) that gap, moving near-linearly. The headline then streams upward at
  // the true rate - a constant flow rather than bursts followed by a freeze.
  let dNet = $state(0);
  let dAdded = $state(0);
  let dRemoved = $state(0);
  let dCommits = $state(0);

  // Smoothed milliseconds between repo ticks, seeded with a sane first guess.
  let gap = 700;
  let lastDone = -1;
  let lastTickAt = 0;

  $effect(() => {
    // Reading the targets registers them as deps, so this re-runs on every tick.
    const toNet = net,
      toAdded = added,
      toRemoved = removed,
      toCommits = commits;

    const now = performance.now();
    if (done !== lastDone) {
      if (lastTickAt) {
        const observed = Math.min(Math.max(now - lastTickAt, 120), 6000);
        gap = gap * 0.55 + observed * 0.45;
      }
      lastTickAt = now;
      lastDone = done;
    }

    const fromNet = untrack(() => dNet);
    const fromAdded = untrack(() => dAdded);
    const fromRemoved = untrack(() => dRemoved);
    const fromCommits = untrack(() => dCommits);

    // Stretch a little past the measured gap so we are usually still moving when
    // the next repo lands; clamp so fast (cached) and very slow repos stay sane.
    const dur = Math.min(5000, Math.max(280, gap * 1.2));
    const start = now;
    let raf = 0;
    const step = (t: number) => {
      const k = Math.min(1, (t - start) / dur); // linear: steady stream
      dNet = Math.round(fromNet + (toNet - fromNet) * k);
      dAdded = Math.round(fromAdded + (toAdded - fromAdded) * k);
      dRemoved = Math.round(fromRemoved + (toRemoved - fromRemoved) * k);
      dCommits = Math.round(fromCommits + (toCommits - fromCommits) * k);
      if (k < 1) raf = requestAnimationFrame(step);
    };
    raf = requestAnimationFrame(step);
    return () => cancelAnimationFrame(raf);
  });

  const pct = $derived(total ? (done / total) * 100 : 0);
</script>

<div class="reveal">
  <div class="head">
    <div class="big mono {dNet >= 0 ? 'add' : 'remove'}">{signed(dNet)}</div>
    <div class="subline">
      <span class="add">+{commas(dAdded)}</span>
      <span class="remove">−{commas(dRemoved)}</span>
      <span class="dim">· {commas(dCommits)} commits, building your Lineage…</span>
    </div>
    <div class="prog">
      <div class="track" class:indeterminate={!total}>
        <div class="fill" style={total ? `width:${pct}%` : ""}></div>
      </div>
      <div class="ptext dim">
        {#if total}{done} / {total} repositories scanned{:else}{message}{/if}
      </div>
    </div>
  </div>

  <div class="cols">
    <div class="col">
      <h3>Languages emerging</h3>
      {#if languages.length}
        <LanguageBars {languages} style={barStyle} />
      {:else}
        <p class="dim small">waiting for the first results…</p>
      {/if}
    </div>
    <div class="col">
      <h3>Tallying repositories</h3>
      <div class="feed">
        {#each feed as f (f.repo)}
          <div class="frow" class:cached={f.cached}>
            <span class="repo mono">{f.repo}</span>
            <span class="nums mono">
              {#if f.lang}<span class="tag">{f.lang}</span>{/if}
              <span class="add">+{commas(f.added)}</span>
              <span class="remove">−{commas(f.removed)}</span>
            </span>
          </div>
        {/each}
      </div>
    </div>
  </div>
</div>

<style>
  .reveal {
    padding: 26px 30px 40px;
    max-width: 900px;
    margin: 0 auto;
  }
  .head {
    text-align: center;
    margin-bottom: 26px;
  }
  .big {
    font-size: 72px;
    font-weight: 800;
    line-height: 1;
    letter-spacing: var(--track-display);
  }
  .subline {
    margin-top: 10px;
    display: flex;
    gap: 10px;
    justify-content: center;
    align-items: baseline;
    flex-wrap: wrap;
  }
  .prog {
    margin-top: 18px;
    max-width: 460px;
    margin-left: auto;
    margin-right: auto;
  }
  .track {
    height: 5px;
    background: var(--bg-elev-2);
    border-radius: 999px;
    overflow: hidden;
  }
  .track .fill {
    height: 100%;
    background: linear-gradient(90deg, var(--accent), #7aa9ff);
    border-radius: 999px;
    transition: width 0.3s ease;
  }
  /* Discovery: repo count unknown, sweep an indeterminate bar. */
  .track.indeterminate .fill {
    width: 38%;
    animation: sweep 1.1s ease-in-out infinite;
  }
  @keyframes sweep {
    0% {
      margin-left: -40%;
    }
    100% {
      margin-left: 100%;
    }
  }
  .ptext {
    margin-top: 7px;
    font-size: 12px;
    text-align: center;
  }
  .cols {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 26px;
  }
  h3 {
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: var(--track-label);
    color: var(--text-dim);
    margin: 0 0 12px;
  }
  .small {
    font-size: 12px;
  }
  .feed {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .frow {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
    padding: 5px 9px;
    border-radius: 7px;
    background: var(--bg-elev);
    font-size: 12px;
    animation: slide 0.28s ease;
  }
  .frow.cached {
    opacity: 0.55;
  }
  .repo {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .nums {
    display: flex;
    gap: 8px;
    align-items: center;
    flex: none;
  }
  .tag {
    font-size: 10px;
    color: var(--text-faint);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0 6px;
  }
  @keyframes slide {
    from {
      opacity: 0;
      transform: translateY(-6px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
