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
    done,
    total,
    languages,
    feed,
  }: {
    net: number;
    added: number;
    removed: number;
    done: number;
    total: number;
    languages: LanguageStat[];
    feed: FeedItem[];
  } = $props();

  // Animated count-up toward the running net, restarted each tick.
  let display = $state(0);
  $effect(() => {
    const to = net;
    const from = untrack(() => display);
    const start = performance.now();
    const dur = 420;
    let raf = 0;
    const step = (t: number) => {
      const k = Math.min(1, (t - start) / dur);
      const eased = 1 - Math.pow(1 - k, 3);
      display = Math.round(from + (to - from) * eased);
      if (k < 1) raf = requestAnimationFrame(step);
    };
    raf = requestAnimationFrame(step);
    return () => cancelAnimationFrame(raf);
  });

  const pct = $derived(total ? (done / total) * 100 : 0);
</script>

<div class="reveal">
  <div class="head">
    <div class="big mono {display >= 0 ? 'add' : 'remove'}">{signed(display)}</div>
    <div class="subline">
      <span class="add">+{commas(added)}</span>
      <span class="remove">−{commas(removed)}</span>
      <span class="dim">net lines, building your master diff…</span>
    </div>
    <div class="prog">
      <div class="track"><div class="fill" style="width:{pct}%"></div></div>
      <div class="ptext dim">{done} / {total} repositories scanned</div>
    </div>
  </div>

  <div class="cols">
    <div class="col">
      <h3>Languages emerging</h3>
      {#if languages.length}
        <LanguageBars {languages} />
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
    font-weight: 700;
    line-height: 1;
    letter-spacing: -0.02em;
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
    letter-spacing: 0.06em;
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
