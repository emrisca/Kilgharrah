export type Category = {
  id: string;
  name: string;
  blurb: string;
  count: number;
  source: "store" | "hermes";
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
};

export type SkillFile = {
  path: string;
  bytes: number;
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
};

export type MarketSource = "clawhub" | "agentic" | "both";

export type InstallJob = {
  id: string;
  slug: string;
  name: string;
  phase: string;
  percent: number;
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
