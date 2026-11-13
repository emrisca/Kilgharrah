import { invoke } from "@tauri-apps/api/core";
import type {
  AppState,
  CatalogItem,
  Diagnostics,
  DiscoveredSkill,
  MarketSource,
  SkillFile,
  SkillSource,
  SkillUpdate,
  UpdateReport
} from "./types";

export const api = {
  snapshot: () => invoke<AppState>("snapshot"),

  addSkill: (input: { name: string; description: string; body: string; category: string }) =>
    invoke<AppState>("add_skill", input),

  deleteSkill: (category: string, slug: string) =>
    invoke<AppState>("delete_skill", { category, slug }),

  setSkillEnabled: (category: string, slug: string, enabled: boolean) =>
    invoke<AppState>("set_skill_enabled", { category, slug, enabled }),

  moveSkill: (category: string, slug: string, target: string) =>
    invoke<AppState>("move_skill", { category, slug, target }),

  listSkillFiles: (source: SkillSource, category: string, slug: string) =>
    invoke<SkillFile[]>("list_skill_files", { source, category, slug }),

  readSkillFile: (source: SkillSource, category: string, slug: string, path: string) =>
    invoke<string>("read_skill_file", { source, category, slug, path }),

  writeSkillFile: (category: string, slug: string, path: string, content: string) =>
    invoke<AppState>("write_skill_file", { category, slug, path, content }),

  setAgentTargets: (targets: string[]) =>
    invoke<AppState>("set_agent_targets", { targets }),

  checkUpdates: () => invoke<SkillUpdate[]>("check_updates"),

  updateSkill: (category: string, slug: string) =>
    invoke<AppState>("update_skill", { category, slug }),

  updateAllSkills: () => invoke<UpdateReport>("update_all_skills"),

  setSkillSource: (category: string, slug: string, owner: string) =>
    invoke<AppState>("set_skill_source", { category, slug, owner }),

  readDragon: () => invoke<string | null>("read_dragon"),

  saveDragon: (frames: string[]) => invoke<void>("save_dragon", { frames }),

  completeSetup: (input: {
    projectRoot: string | null;
    agent: string;
    profile: string | null;
  }) =>
    invoke<AppState>("complete_setup", {
      projectRoot: input.projectRoot,
      agent: input.agent,
      profile: input.profile
    }),

  exportProfile: (name: string, path: string | null = null) =>
    invoke<string>("export_profile", { name, path }),

  importProfile: (path: string) => invoke<AppState>("import_profile", { path }),

  applyProfile: (name: string) => invoke<AppState>("apply_profile", { name }),

  deleteProfile: (name: string) => invoke<AppState>("delete_profile", { name }),

  scanInstalledSkills: () => invoke<DiscoveredSkill[]>("scan_installed_skills"),

  importInstalledSkills: (paths: string[]) =>
    invoke<AppState>("import_installed_skills", { paths }),

  diagnostics: () => invoke<Diagnostics>("diagnostics"),

  saveServer: (input: { name: string; command: string; args: string[]; env: string[] }) =>
    invoke<AppState>("save_server", input),

  deleteServer: (name: string) => invoke<AppState>("delete_server", { name }),

  searchMarket: (query: string, limit = 24) =>
    invoke<CatalogItem[]>("search_market", { query, limit }),

  previewMarket: (slug: string, owner: string | null, market: MarketSource) =>
    invoke<string>("preview_market", { slug, owner, market }),

  startInstall: (
    slug: string,
    owner: string | null,
    category: string | null,
    market: MarketSource
  ) => invoke<string>("install_market", { slug, owner, category, market }),

  openUrl: (url: string) => invoke<void>("open_url", { url }),

  revealStore: () => invoke<void>("reveal_store")
};
