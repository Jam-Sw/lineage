// Growth detection for the impact tree. Compares the snapshot the user last
// looked at (the "baseline") with a fresh one and returns the trails that grew.
// A trail is the same name chain the tree draws - language -> repo -> folder ->
// file - so each flagged key maps straight onto a node, and the whole chain
// from the centre avatar out to the new growth is flagged (the energy wave
// needs an unbroken path to travel along).
import type { PathNode, RepoStat, Snapshot } from "./api/types";

/** The tree's key for a trail: parts joined by "/". No part (language, repo
 *  label, folder or file name) can itself contain a slash. */
export function trailKey(parts: string[]): string {
  return parts.join("/");
}

/** The label the tree gives a repo: "owner/name" -> "name". */
export function repoShortName(fullName: string): string {
  return fullName.slice(fullName.indexOf("/") + 1);
}

export function findGrownTrails(baseline: Snapshot | null, current: Snapshot): Set<string> {
  const out = new Set<string>();

  const flag = (parts: string[]) => {
    for (let i = 1; i <= parts.length; i++) out.add(trailKey(parts.slice(0, i)));
  };

  // Everything under a node the baseline never had: all of it is new. The
  // zero-added twigs the tree would still draw are skipped.
  const flagSubtree = (kids: PathNode[], prefix: string[]) => {
    for (const c of kids) {
      if (c.added === 0) continue;
      flag([...prefix, c.name]);
      flagSubtree(c.children ?? [], [...prefix, c.name]);
    }
  };

  // A limb that is new in full (a repo that just appeared).
  const wholeLimb = (lang: string, r: RepoStat) => {
    const head = [lang, repoShortName(r.fullName)];
    flag(head);
    flagSubtree(r.tree?.children ?? [], head);
  };

  // A repo that existed before: recurse only where lines actually landed. A
  // parent's `added` includes everything below it, so an equal count means
  // nothing under it grew and the subtree can be skipped outright.
  const walkDiff = (kids: PathNode[], baseKids: PathNode[], prefix: string[]) => {
    const baseByName = new Map(baseKids.map((c) => [c.name, c]));
    for (const c of kids) {
      const b = baseByName.get(c.name);
      if (!b) {
        if (c.added === 0) continue;
        flag([...prefix, c.name]);
        flagSubtree(c.children ?? [], [...prefix, c.name]);
        continue;
      }
      if (c.added <= b.added) continue;
      flag([...prefix, c.name]);
      walkDiff(c.children ?? [], b.children ?? [], [...prefix, c.name]);
    }
  };

  const baseByRepo = baseline ? new Map(baseline.repos.map((r) => [r.fullName, r])) : null;
  for (const r of current.repos) {
    const lang = r.topLanguage ?? "Other";
    if (r.added === 0) continue;
    const b = baseByRepo?.get(r.fullName);
    if (!baseByRepo || !b) {
      // First look ever, or a repo we have never counted before.
      wholeLimb(lang, r);
      continue;
    }
    if (r.added <= b.added) continue;
    const head = [lang, repoShortName(r.fullName)];
    flag(head);
    if (r.tree && b.tree) walkDiff(r.tree.children ?? [], b.tree.children ?? [], head);
  }
  return out;
}
