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

export type CatalogItem = {
  id: string;
  kind: "skill" | "plugin" | "bundle";
  owner: string;
  slug: string;
  name: string;
  summary: string;
  version: string;
  downloads: number;
  category: string;
  topics: string[];
  installable: boolean;
  command: string | null;
};
