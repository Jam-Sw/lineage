<script lang="ts">
  import type { LanguageStat } from "$lib/api/types";
  import { squarify } from "$lib/treemap";

  let { languages, height = 240 }: { languages: LanguageStat[]; height?: number } = $props();

  let width = $state(600);
  const rects = $derived(
    squarify(
      languages.map((l) => ({ key: l.language, value: l.added + l.removed, color: l.color })),
      width,
      height,
    ),
  );
</script>

<div class="tm" style="height:{height}px" bind:clientWidth={width}>
  {#each rects as r (r.key)}
    <div
      class="tile"
      style="left:{r.x}px;top:{r.y}px;width:{r.w}px;height:{r.h}px;background:{r.color}"
      title={r.key}
    >
      {#if r.w > 56 && r.h > 28}<span class="lbl">{r.key}</span>{/if}
    </div>
  {/each}
</div>

<style>
  .tm {
    position: relative;
    width: 100%;
    border-radius: 10px;
    overflow: hidden;
  }
  .tile {
    position: absolute;
    box-sizing: border-box;
    border: 1.5px solid var(--bg);
    display: flex;
    align-items: flex-start;
    transition:
      left 0.4s ease,
      top 0.4s ease,
      width 0.4s ease,
      height 0.4s ease;
  }
  .lbl {
    font-size: 11px;
    font-weight: 600;
    color: #000;
    background: rgba(255, 255, 255, 0.72);
    padding: 1px 5px;
    margin: 4px;
    border-radius: 4px;
    white-space: nowrap;
  }
</style>
