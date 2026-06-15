// Mirrors the serde (camelCase) types in lineage-core.

export interface Summary {
  added: number;
  removed: number;
  net: number;
  commits: number;
  repoCount: number;
  languageCount: number;
}

export interface LanguageStat {
  language: string;
  color: string;
  added: number;
  removed: number;
  net: number;
  share: number;
}

export interface RepoStat {
  fullName: string;
  added: number;
  removed: number;
  net: number;
  commits: number;
  topLanguage: string | null;
}

export interface Snapshot {
  summary: Summary;
  languages: LanguageStat[];
  repos: RepoStat[];
  filtered: boolean;
  lastSyncedAt: string | null;
}

export interface AuthStatus {
  connected: boolean;
  source: string | null;
  login: string | null;
}

export interface SyncStatus {
  syncing: boolean;
  phase: string;
  reposDone: number;
  reposTotal: number;
  currentRepo: string | null;
  lastFinishedAt: string | null;
  lastError: string | null;
}

export interface AppSettings {
  includeForks: boolean;
  ownerOnly: boolean;
  includeArchived: boolean;
  excludeGenerated: boolean;
  extraEmails: string[];
  seenTour: boolean;
  // Closing the window: "ask" (prompt on first close), "menuBar" (idle to the
  // tray), or "quit" (terminate the app).
  closeBehavior: string;
}

export interface SyncTick {
  done: number;
  total: number;
  repo: string;
  repoAdded: number;
  repoRemoved: number;
  repoTopLanguage: string | null;
  repoCommits: number;
  fromCache: boolean;
  added: number;
  removed: number;
  net: number;
  commits: number;
  languages: LanguageStat[];
}

export interface AppearanceSettings {
  trayIcon: string;
  trayShowNumber: boolean;
  trayMetric: string;
  barStyle: string;
}

export interface SyncPhase {
  // "preparing" | "discovering" | "scanning" | "saving"
  phase: string;
  message: string;
}

export interface ContributionDay {
  date: string;
  count: number;
  color: string;
}

export interface ProfileStats {
  login: string;
  name: string | null;
  avatarDataUri: string | null;
  createdYear: number;
  totalContributions: number;
  lastYearContributions: number;
  calendar: ContributionDay[];
  fetchedAt: string | null;
}
