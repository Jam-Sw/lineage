<script lang="ts">
  import type { LanguageStat } from "$lib/api/types";
  import { signed } from "$lib/format";

  let { languages }: { languages: LanguageStat[] } = $props();
  const peak = $derived(Math.max(1, ...languages.map((l) => l.added + l.removed)));
</script>

<div class="langs">
  {#each languages as l (l.language)}
    <div class="lang">
      <div class="name"><span class="sw" style="background:{l.color}"></span>{l.language}</div>
      <div class="bar">
        <div
          class="fill"
          style="width:{((l.added + l.removed) / peak) * 100}%; background:{l.color}"
        ></div>
      </div>
      <div class="net mono {l.net >= 0 ? 'add' : 'remove'}">{signed(l.net)}</div>
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
    grid-template-columns: 130px 1fr 84px;
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
  .bar {
    height: 10px;
    background: var(--bg-elev-2);
    border-radius: 999px;
    overflow: hidden;
  }
  .fill {
    height: 100%;
    border-radius: 999px;
    min-width: 2px;
    transition: width 0.45s cubic-bezier(0.22, 1, 0.36, 1);
  }
  .net {
    text-align: right;
    font-size: 12px;
  }
</style>
