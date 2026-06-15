// =============================================================================
// FEATURE NOTE (parked): subtle rising "sync" sound
// =============================================================================
//
// Idea: during the first-run sync, as repos stream in and the lineage
// headline climbs, play a soft tone that rises in pitch with progress and
// swells slightly while the number is actively counting up. A small "number go
// up" reward for the one-time reveal.
//
// Status: PARKED, not wired in. In the Tauri macOS webview (WKWebView) no sound
// was audible in testing even though the AudioContext was unlocked inside the
// GO click gesture. Likely WKWebView autoplay/output quirks; would need real
// investigation (or shipping a tiny pre-rendered audio asset and playing it via
// an <audio>/AudioBufferSourceNode instead of live oscillators) before it earns
// its place. Everything below is the full implementation + wiring, commented
// out, so it can be revived quickly if we decide to chase it.
//
// To revive: copy the module block into `src/lib/audio.ts`, then re-apply the
// three wiring snippets at the bottom.
//
// -----------------------------------------------------------------------------
// MODULE: src/lib/audio.ts
// -----------------------------------------------------------------------------
//
// type Voice = {
//   ctx: AudioContext;
//   master: GainNode;
//   oscA: OscillatorNode;
//   oscB: OscillatorNode;
//   started: boolean;
// };
//
// let voice: Voice | null = null;
// let enabled = true;
//
// const BASE_GAIN = 0.03; // idle drone, barely there
// const ACTIVE_GAIN = 0.05; // a hair louder while the number climbs
// const LO_HZ = 174.61; // F3 - the bottom of the rise (sync just starting)
// const HI_HZ = 659.25; // E5 - the top of the rise (sync complete)
//
// function audioCtor(): typeof AudioContext | null {
//   if (typeof window === "undefined") return null;
//   return window.AudioContext ?? (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext ?? null;
// }
//
// export function setSyncSoundEnabled(on: boolean) {
//   enabled = on;
//   if (!on) stopSyncSound();
// }
//
// // Call from the click handler that kicks off a sync. Browsers only allow audio
// // to start inside a user gesture, so this is where the context is unlocked. The
// // oscillators stay silent until the first repo result lands (updateSyncSound).
// export function primeSyncSound() {
//   if (!enabled || voice) return;
//   try {
//     const Ctor = audioCtor();
//     if (!Ctor) return;
//     const ctx = new Ctor();
//     const master = ctx.createGain();
//     master.gain.value = 0;
//     const filter = ctx.createBiquadFilter();
//     filter.type = "lowpass";
//     filter.frequency.value = 1400;
//     filter.Q.value = 0.6;
//     const oscA = ctx.createOscillator();
//     oscA.type = "sine";
//     oscA.frequency.value = LO_HZ;
//     const oscB = ctx.createOscillator();
//     oscB.type = "triangle";
//     oscB.frequency.value = LO_HZ;
//     oscB.detune.value = -8; // gentle chorus against oscA
//     oscA.connect(filter);
//     oscB.connect(filter);
//     filter.connect(master);
//     master.connect(ctx.destination);
//     voice = { ctx, master, oscA, oscB, started: false };
//     void ctx.resume();
//   } catch {
//     voice = null;
//   }
// }
//
// // progress: 0..1 across the whole sync, drives the pitch rise.
// // climbing: true while the headline number is actively counting up, swells the volume.
// export function updateSyncSound(progress: number, climbing: boolean) {
//   if (!enabled || !voice) return;
//   try {
//     const { ctx, master, oscA, oscB } = voice;
//     const t = ctx.currentTime;
//     const p = Math.min(1, Math.max(0, progress));
//
//     if (!voice.started) {
//       oscA.start();
//       oscB.start();
//       voice.started = true;
//     }
//
//     // Geometric pitch mapping so equal progress steps feel like equal musical
//     // steps; setTargetAtTime glides smoothly toward it without zipper noise.
//     const hz = LO_HZ * Math.pow(HI_HZ / LO_HZ, p);
//     oscA.frequency.setTargetAtTime(hz, t, 0.15);
//     oscB.frequency.setTargetAtTime(hz, t, 0.15);
//
//     master.gain.setTargetAtTime(climbing ? ACTIVE_GAIN : BASE_GAIN, t, 0.12);
//   } catch {
//     // ignore: audio must never interrupt the sync UI
//   }
// }
//
// // Fade out and release the context. Safe to call repeatedly.
// export function stopSyncSound() {
//   const v = voice;
//   voice = null;
//   if (!v) return;
//   try {
//     const t = v.ctx.currentTime;
//     v.master.gain.cancelScheduledValues(t);
//     v.master.gain.setTargetAtTime(0, t, 0.18);
//     if (v.started) {
//       v.oscA.stop(t + 0.6);
//       v.oscB.stop(t + 0.6);
//     }
//     setTimeout(() => {
//       try {
//         void v.ctx.close();
//       } catch {
//         // already closed
//       }
//     }, 800);
//   } catch {
//     // ignore
//   }
// }
//
// -----------------------------------------------------------------------------
// WIRING 1: src/routes/+page.svelte  (unlock on the GO gesture, stop when done)
// -----------------------------------------------------------------------------
//
//   import { primeSyncSound, stopSyncSound } from "$lib/audio";
//
//   // in doSync(), right after `syncingLive = true;`:
//   primeSyncSound(); // unlock Web Audio inside this click gesture
//
//   // in the "sync:done" and "sync:error" listeners:
//   stopSyncSound();
//
// -----------------------------------------------------------------------------
// WIRING 2: src/lib/components/LiveReveal.svelte  (drive pitch from the count-up)
// -----------------------------------------------------------------------------
//
//   import { untrack, onDestroy } from "svelte";
//   import { updateSyncSound, stopSyncSound } from "$lib/audio";
//
//   // inside the count-up $effect, once per new tick (after updating lastDone):
//   const progress = total ? done / total : 0;
//   updateSyncSound(progress, true);
//
//   // in the rAF `step`, when the count-up settles (the `else` of `if (k < 1)`):
//   else updateSyncSound(progress, false); // ease the swell back down
//
//   // and at top level so the sound stops if the view unmounts:
//   onDestroy(stopSyncSound);
//
// =============================================================================

export {};
