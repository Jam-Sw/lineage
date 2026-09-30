// Growth detection for the impact tree: which limbs grew since the last time
// the user looked at the graph. Trails are the same name chains the tree builds
// (language -> repo -> folder -> file), so a flagged key maps straight onto a
// drawn node.
import { describe, expect, it } from "vitest";
import type { PathNode, RepoStat, Snapshot } from "./api/types";
import { findGrownTrails, repoShortName, trailKey } from "./tree-diff";

function path(name: string, added: number, children: PathNode[] = [], language = "TypeScript"): PathNode {
  return { name, added, removed: 0, language, children };
}

function repo(
  fullName: string,
  added: number,
  opts: { topLanguage?: string | null; tree?: PathNode | null } = {},
): RepoStat {
  return {
    fullName,
    added,
    removed: 0,
    net: added,
    commits: 1,
    topLanguage: opts.topLanguage === undefined ? "TypeScript" : opts.topLanguage,
    tree: opts.tree ?? null,
  };
}

function snap(repos: RepoStat[]): Snapshot {
  return {
    summary: {
      added: repos.reduce((t, r) => t + r.added, 0),
      removed: 0,
      net: 0,
      commits: repos.length,
      repoCount: repos.length,
      languageCount: 1,
    },
    languages: [],
    repos,
    filtered: true,
    lastSyncedAt: null,
  };
}

function grownKeys(baseline: Snapshot | null, current: Snapshot): string[] {
  return [...findGrownTrails(baseline, current)].sort();
}

describe("trailKey / repoShortName", () => {
  it("joins trail parts into the tree's key", () => {
    expect(trailKey(["Rust", "lineage", "src"])).toBe("Rust/lineage/src");
  });

  it("shortens owner/repo to the tree's label", () => {
    expect(repoShortName("me/lineage")).toBe("lineage");
    expect(repoShortName("bare")).toBe("bare");
  });
});

describe("findGrownTrails", () => {
  it("returns nothing for an identical snapshot", () => {
    const tree = path("app", 10, [path("src", 10, [path("a.ts", 10)])]);
    const s = snap([repo("me/app", 10, { tree })]);
    expect(grownKeys(s, structuredClone(s))).toEqual([]);
  });

  it("returns nothing when repos only lost lines or disappeared", () => {
    const before = snap([repo("me/app", 10), repo("me/bye", 5)]);
    const after = snap([repo("me/app", 7)]);
    expect(grownKeys(before, after)).toEqual([]);
  });

  it("flags the whole limb of a repository that did not exist", () => {
    const tree = path("fresh", 12, [path("src", 12, [path("a.ts", 8), path("b.ts", 4)]), path("README.md", 2)]);
    const current = snap([repo("me/fresh", 12, { topLanguage: "Rust", tree })]);
    expect(grownKeys(snap([]), current)).toEqual(
      [
        ["Rust"],
        ["Rust", "fresh"],
        ["Rust", "fresh", "src"],
        ["Rust", "fresh", "src", "a.ts"],
        ["Rust", "fresh", "src", "b.ts"],
        ["Rust", "fresh", "README.md"],
      ].map(trailKey).sort(),
    );
  });

  it("flags exactly the files that grew, and their lineage", () => {
    const before = snap([
      repo("me/app", 12, {
        tree: path("app", 12, [path("src", 10, [path("a.ts", 6), path("b.ts", 4)]), path("README.md", 2)]),
      }),
    ]);
    const after = snap([
      repo("me/app", 14, {
        tree: path("app", 14, [path("src", 12, [path("a.ts", 8), path("b.ts", 4)]), path("README.md", 2)]),
      }),
    ]);
    expect(grownKeys(before, after)).toEqual(
      [
        ["TypeScript"],
        ["TypeScript", "app"],
        ["TypeScript", "app", "src"],
        ["TypeScript", "app", "src", "a.ts"],
      ]
        .map(trailKey)
        .sort(),
    );
  });

  it("flags a folder that just appeared and everything under it", () => {
    const before = snap([
      repo("me/app", 6, { tree: path("app", 6, [path("src", 6, [path("a.ts", 6)])]) }),
    ]);
    const after = snap([
      repo("me/app", 11, {
        tree: path("app", 11, [path("src", 11, [path("a.ts", 6), path("docs", 5, [path("guide.md", 5)])])]),
      }),
    ]);
    expect(grownKeys(before, after)).toEqual(
      [
        ["TypeScript"],
        ["TypeScript", "app"],
        ["TypeScript", "app", "src"],
        ["TypeScript", "app", "src", "docs"],
        ["TypeScript", "app", "src", "docs", "guide.md"],
      ]
        .map(trailKey)
        .sort(),
    );
  });

  it("does not flag a new folder that holds no added lines", () => {
    const before = snap([repo("me/app", 6, { tree: path("app", 6, [path("src", 6, [path("a.ts", 6)])]) })]);
    const after = snap([
      repo("me/app", 6, {
        tree: path("app", 6, [path("src", 6, [path("a.ts", 6), path("trash", 0)])]),
      }),
    ]);
    expect(grownKeys(before, after)).toEqual([]);
  });

  it("flags just the repo branch when path trees are unavailable", () => {
    const before = snap([repo("me/app", 10)]);
    const after = snap([repo("me/app", 15)]);
    expect(grownKeys(before, after)).toEqual([trailKey(["TypeScript"]), trailKey(["TypeScript", "app"])].sort());
  });

  it("groups a repo without a top language under Other", () => {
    const current = snap([repo("me/misc", 3, { topLanguage: null, tree: path("misc", 3, [path("x.txt", 3)]) })]);
    expect(grownKeys(null, current)).toEqual(
      [["Other"], ["Other", "misc"], ["Other", "misc", "x.txt"]].map(trailKey).sort(),
    );
  });

  it("treats a first look (no baseline) as everything new", () => {
    const current = snap([
      repo("me/app", 6, { tree: path("app", 6, [path("a.ts", 6)]) }),
      repo("me/lib", 4, { topLanguage: "Rust" }),
    ]);
    expect(grownKeys(null, current)).toEqual(
      [
        ["TypeScript"],
        ["TypeScript", "app"],
        ["TypeScript", "app", "a.ts"],
        ["Rust"],
        ["Rust", "lib"],
      ]
        .map(trailKey)
        .sort(),
    );
  });
});
