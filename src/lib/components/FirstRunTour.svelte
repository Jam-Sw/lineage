<script lang="ts">
  // First-run coachmark shown once on the first real dashboard. It teaches the one
  // thing the UI cannot otherwise reveal: there are more pages, reachable by
  // swiping. (What "Sync now" does is explained before the first sync, on the
  // pre-sync screen, so it is not repeated here.)
  //
  // No friction, no "Skip": any click or keypress anywhere fades it out, and the
  // caller persists that it has been seen. It never gates the real controls.
  let { ondone }: { ondone: () => void } = $props();

  let leaving = $state(false);

  function dismiss() {
    if (leaving) return;
    leaving = true;
    setTimeout(ondone, 220); // let the fade play, then unmount + persist
  }
</script>

<svelte:window onkeydowncapture={dismiss} />

<!-- A teaching scrim, not a gate: one click both learns and gets out of the way. -->
<div
  class="tour"
  class:leaving
  role="button"
  tabindex="-1"
  aria-label="Dismiss tour"
  onclick={dismiss}
  onkeydown={dismiss}
>
  <!-- Swipe hint, anchored just above the page dots (bottom-center). -->
  <div class="card swipe" role="presentation">
    <span class="cue">‹ swipe ›</span>
    <span class="pages">Dashboard · Impact tree</span>
  </div>

  <div class="hint">click anywhere to continue</div>
</div>

<style>
  .tour {
    position: fixed;
    inset: 0;
    z-index: 50;
    background: rgba(15, 17, 21, 0.55);
    backdrop-filter: blur(1.5px);
    cursor: pointer;
    opacity: 1;
    transition: opacity 0.2s ease;
  }
  .tour.leaving {
    opacity: 0;
  }

  .card {
    position: absolute;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow: 0 8px 30px rgba(0, 0, 0, 0.45);
    color: var(--text);
  }

  .swipe {
    bottom: 40px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 14px;
    animation: bob 1.6s ease-in-out infinite;
  }
  .swipe .cue {
    color: var(--accent);
    font-weight: 700;
    letter-spacing: 0.04em;
  }
  .swipe .pages {
    font-size: 12px;
    color: var(--text-dim);
  }
  @keyframes bob {
    0%,
    100% {
      transform: translateX(-50%);
    }
    50% {
      transform: translateX(calc(-50% + 4px));
    }
  }

  .hint {
    position: absolute;
    bottom: 12px;
    left: 0;
    right: 0;
    text-align: center;
    font-size: 11px;
    color: var(--text-faint);
  }
</style>
