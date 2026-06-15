// Mirrors the serde (camelCase) types in masterdiff-core.

export interface Summary {
  added: number;
  removed: number;
  net: number;
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
}

export interface SyncTick {
  done: number;
  total: number;
  repo: string;
  repoAdded: number;
  repoRemoved: number;
  repoTopLanguage: string | null;
  fromCache: boolean;
  added: number;
  removed: number;
  net: number;
  languages: LanguageStat[];
}
