export type Category = {
  id: string;
  name: string;
  blurb: string;
  count: number;
};

export type Skill = {
  slug: string;
  name: string;
  description: string;
  category: string;
  path: string;
  enabled: boolean;
  files: number;
  updatedAt: number;
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
  installable: boolean;
  command: string | null;
};
