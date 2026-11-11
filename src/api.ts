import { invoke } from "@tauri-apps/api/core";
import type { AppState, CatalogItem, SkillFile, SkillSource } from "./types";

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

  importHermes: (category: string, slug: string, target: string) =>
    invoke<AppState>("import_hermes", { category, slug, target }),

  saveServer: (input: { name: string; command: string; args: string[]; env: string[] }) =>
    invoke<AppState>("save_server", input),

  deleteServer: (name: string) => invoke<AppState>("delete_server", { name }),

  searchMarket: (query: string, limit = 24) =>
    invoke<CatalogItem[]>("search_market", { query, limit }),

  previewMarket: (slug: string, owner?: string) =>
    invoke<string>("preview_market", { slug, owner: owner ?? null }),

  installMarket: (slug: string, owner: string | null, category: string | null) =>
    invoke<AppState>("install_market", { slug, owner, category }),

  openUrl: (url: string) => invoke<void>("open_url", { url }),

  revealStore: () => invoke<void>("reveal_store")
};
