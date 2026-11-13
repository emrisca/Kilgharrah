export type Category = {
  id: string;
  name: string;
  blurb: string;
  count: number;
  source: string;
};

export type SkillSource = "store" | "hermes";

export type Skill = {
  id: string;
  source: SkillSource;
  slug: string;
  name: string;
  description: string;
  category: string;
  path: string;
  enabled: boolean;
  files: number;
  updatedAt: number;
  version: string | null;
  origin: string;
  installedAt: number;
  modified: boolean;
  scripts: number;
  profile: string | null;
};

export type AgentTarget = {
  id: string;
  name: string;
  path: string;
  enabled: boolean;
  linked: number;
};

export type SkillUpdate = {
  category: string;
  slug: string;
  name: string;
  origin: string;
  current: string | null;
  latest: string | null;
  status: "available" | "current" | "unknown" | "error";
  detail: string;
};

export type UpdateReport = {
  state: AppState;
  updated: number;
  failed: string[];
};

export type DiagnosticCheck = {
  id: string;
  label: string;
  status: "ok" | "warn" | "error";
  detail: string;
};

export type Diagnostics = {
  ok: boolean;
  checks: DiagnosticCheck[];
  checkedAt: number;
};

export type SkillFile = {
  path: string;
  bytes: number;
};

export type SetupView = {
  onboarded: boolean;
  scope: "home" | "project";
  projectRoot: string | null;
  agent: string;
  agentName: string;
  profile: string | null;
};

export type ProfileInfo = {
  name: string;
  path: string | null;
  skills: number;
  servers: number;
  serverNames: string[];
  skillNames: string[];
  excludedNames: string[];
  location: string;
  builtin: boolean;
};

export type DiscoveredSkill = {
  id: string;
  name: string;
  slug: string;
  description: string;
  category: string;
  source: string;
  path: string;
  files: number;
  platforms: string[];
  portable: boolean;
  inLibrary: boolean;
};

export type McpServer = {
  name: string;
  command: string;
  args: string[];
  env: Record<string, string>;
  enabled: boolean;
};

export type AppState = {
  store: string;
  skillsRoot: string;
  hermesRoot: string;
  mcpPath: string;
  categories: Category[];
  skills: Skill[];
  servers: McpServer[];
  agentTargets: AgentTarget[];
  setup: SetupView;
  profiles: ProfileInfo[];
};

export type MarketSource = "clawhub" | "agentic" | "both";

export type InstallJob = {
  id: string;
  slug: string;
  name: string;
  phase: string;
  percent: number;
};

export type AgenticChoice = {
  name: string;
  path: string;
};

export type CatalogItem = {
  id: string;
  kind: "skill" | "plugin" | "bundle";
  owner: string;
  slug: string;
  name: string;
  summary: string;
  version: string;
  downloads: number;
  stars: number;
  rank: number;
  grade: string;
  market: MarketSource;
  category: string;
  topics: string[];
  installable: boolean;
  command: string | null;
};
