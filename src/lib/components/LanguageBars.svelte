<script lang="ts">
  import type { LanguageStat } from "$lib/api/types";
  import { abbrevU } from "$lib/format";

  let { languages, style = "language" }: { languages: LanguageStat[]; style?: string } = $props();
  const peak = $derived(Math.max(1, ...languages.map((l) => l.added + l.removed)));
  const widthOf = (l: LanguageStat) => ((l.added + l.removed) / peak) * 100;
</script>

<div class="langs">
  {#each languages as l (l.language)}
    <div class="lang">
      <div class="name"><span class="sw" style="background:{l.color}"></span>{l.language}</div>
      <div class="track">
        {#if style === "diff"}
          <div class="seg" style="width:{widthOf(l)}%">
            <div class="g" style="flex:{l.added}"></div>
            <div class="r" style="flex:{l.removed}"></div>
          </div>
        {:else}
          <div class="fill" style="width:{widthOf(l)}%; background:{l.color}"></div>
        {/if}
      </div>
      <div class="nums mono">
        <span class="add">+{abbrevU(l.added)}</span>
        <span class="remove">−{abbrevU(l.removed)}</span>
      </div>
    </div>
  {/each}
</div>

<style>
  .langs {
    display: flex;
    flex-direction: column;
    gap: 7px;
  }
  .lang {
    display: grid;
    grid-template-columns: 126px 1fr 104px;
    align-items: center;
    gap: 10px;
  }
  .name {
    display: flex;
    align-items: center;
    gap: 7px;
    overflow: hidden;
    white-space: nowrap;
    font-size: 13px;
  }
  .sw {
    width: 10px;
    height: 10px;
    border-radius: 3px;
    flex: none;
  }
  .track {
    height: 10px;
    background: var(--bg-elev-2);
    border-radius: 999px;
    overflow: hidden;
  }
  .fill,
  .seg {
    height: 100%;
    border-radius: 999px;
    min-width: 2px;
    display: flex;
    overflow: hidden;
    transition: width 0.45s cubic-bezier(0.22, 1, 0.36, 1);
  }
  .g {
    background: var(--add);
    transition: flex-grow 0.45s ease;
  }
  .r {
    background: var(--remove);
    transition: flex-grow 0.45s ease;
  }
  .nums {
    display: flex;
    gap: 7px;
    justify-content: flex-end;
    font-size: 11px;
  }
</style>
