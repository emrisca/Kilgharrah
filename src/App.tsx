import { useCallback, useEffect, useState } from "react";
import { api } from "./api";
import type { AppState } from "./types";
import { SkillsView } from "./components/SkillsView";
import { McpView } from "./components/McpView";
import { Icon } from "./components/Icons";

type Theme = "dark" | "light";
const THEME_KEY = "kilgharrah.theme";

export default function App() {
  const [state, setState] = useState<AppState | null>(null);
  const [tab, setTab] = useState<"skills" | "mcp">("skills");
  const [toast, setToast] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [theme, setTheme] = useState<Theme>(() => {
    const saved = localStorage.getItem(THEME_KEY);
    if (saved === "dark" || saved === "light") return saved;
    return window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
  });

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    localStorage.setItem(THEME_KEY, theme);
  }, [theme]);

  useEffect(() => {
    if (!toast) return;
    const timer = setTimeout(() => setToast(null), 3600);
    return () => clearTimeout(timer);
  }, [toast]);

  useEffect(() => {
    if (!error) return;
    const timer = setTimeout(() => setError(null), 8000);
    return () => clearTimeout(timer);
  }, [error]);

  const load = useCallback(async () => {
    try {
      setState(await api.snapshot());
    } catch (reason) {
      setError(String(reason));
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const enabled = state?.skills.filter((skill) => skill.enabled).length ?? 0;

  return (
    <div className="app">
      <header className="topbar">
        <div className="brand">
          <span className="brand-mark">{Icon.Kilgharrah}</span>
          <span className="brand-name">Kilgharrah</span>
        </div>

        <nav className="tabs" aria-label="Sections">
          <button className="tab" data-active={tab === "skills"} onClick={() => setTab("skills")}>
            Skills <span className="tab-count">{enabled}</span>
          </button>
          <button className="tab" data-active={tab === "mcp"} onClick={() => setTab("mcp")}>
            MCP servers <span className="tab-count">{state?.servers.length ?? 0}</span>
          </button>
        </nav>

        <div className="top-actions">
          <button
            className="ghost-btn"
            title={state?.store ?? "Open the store"}
            onClick={() => api.revealStore().catch((reason) => setError(String(reason)))}
          >
            {Icon.Folder}
            <span>Store</span>
          </button>
          <button
            className="icon-btn"
            title={theme === "dark" ? "Switch to light" : "Switch to dark"}
            aria-label="Toggle appearance"
            onClick={() => setTheme(theme === "dark" ? "light" : "dark")}
          >
            {theme === "dark" ? Icon.Sun : Icon.Moon}
          </button>
        </div>
      </header>

      <main className="page">
        {state === null ? (
          <Loading />
        ) : tab === "skills" ? (
          <SkillsView
            state={state}
            onState={setState}
            notify={setToast}
            onError={(message) => setError(message)}
          />
        ) : (
          <McpView
            state={state}
            onState={setState}
            notify={setToast}
            onError={(message) => setError(message)}
          />
        )}
      </main>

      {toast && <div className="toast">{toast}</div>}
      {error && (
        <div className="toast error" role="alert">
          {error}
        </div>
      )}
    </div>
  );
}

function Loading() {
  return (
    <div className="loading" aria-busy="true" aria-label="Loading skills">
      <div className="skel skel-search" />
      <div className="skel-group">
        <div className="skel skel-title" />
        <div className="grid">
          {[0, 1, 2, 3, 4, 5].map((index) => (
            <div className="skel skel-card" key={index} />
          ))}
        </div>
      </div>
    </div>
  );
}
