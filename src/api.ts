import { invoke } from "@tauri-apps/api/core";
import type { AppState, CatalogItem } from "./types";

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

  saveServer: (input: { name: string; command: string; args: string[]; env: string[] }) =>
    invoke<AppState>("save_server", input),

  deleteServer: (name: string) => invoke<AppState>("delete_server", { name }),

  searchMarket: (query: string, limit = 24) =>
    invoke<CatalogItem[]>("search_market", { query, limit }),

  previewMarket: (slug: string) => invoke<string>("preview_market", { slug }),

  installMarket: (slug: string, category: string | null) =>
    invoke<AppState>("install_market", { slug, category }),

  openUrl: (url: string) => invoke<void>("open_url", { url }),

  revealStore: () => invoke<void>("reveal_store")
};
