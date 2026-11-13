import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { api } from "./api";
import type { AppState, CatalogItem, InstallJob } from "./types";
import { HomeView } from "./components/HomeView";
import { SkillsView } from "./components/SkillsView";
import { McpView } from "./components/McpView";
import { SettingsView } from "./components/SettingsView";
import { SetupOverlay } from "./components/SetupOverlay";
import { Icon } from "./components/Icons";
import { useUpdater } from "./updater";

type Theme = "dark" | "light";
export type Palette = "aurora" | "ember" | "tide" | "orchid" | "rose" | "merlin";
export type Tab = "home" | "skills" | "mcp" | "settings";
const THEME_KEY = "kilgharrah.theme";
const PALETTE_KEY = "kilgharrah.palette";
const PALETTES: Palette[] = ["aurora", "ember", "tide", "orchid", "rose", "merlin"];

export default function App() {
  const [state, setState] = useState<AppState | null>(null);
  const [tab, setTab] = useState<Tab>("home");
  const [toast, setToast] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [installs, setInstalls] = useState<InstallJob[]>([]);
  const [setupOpen, setSetupOpen] = useState(false);
  const installNames = useRef(new Map<string, string>());
  const update = useUpdater();
  const [theme, setTheme] = useState<Theme>(() => {
    const saved = localStorage.getItem(THEME_KEY);
    if (saved === "dark" || saved === "light") return saved;
    return window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
  });
  const [palette, setPalette] = useState<Palette>(() => {
    const saved = localStorage.getItem(PALETTE_KEY) as Palette | null;
    return saved && PALETTES.includes(saved) ? saved : "aurora";
  });

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    localStorage.setItem(THEME_KEY, theme);
    getCurrentWindow().setTheme(theme).catch(() => {});
  }, [theme]);

  useEffect(() => {
    document.documentElement.dataset.palette = palette;
    localStorage.setItem(PALETTE_KEY, palette);
  }, [palette]);

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

  // Installs run on a Rust worker and report back by job id, so switching
  // sections or closing the sheet never stops them.
  useEffect(() => {
    const stops: Array<() => void> = [];
    const keep = (stop: () => void) => stops.push(stop);

    void listen<{ id: string; phase: string; percent: number }>("install-progress", (event) => {
      const { id, phase, percent } = event.payload;
      setInstalls((rows) =>
        rows.map((row) => (row.id === id ? { ...row, phase, percent } : row))
      );
    }).then(keep);

    void listen<{ id: string; state: AppState }>("install-finished", (event) => {
      const { id, state: next } = event.payload;
      const name = installNames.current.get(id) ?? "Skill";
      installNames.current.delete(id);
      setState(next);
      setToast(`${name} installed and enabled`);
      setInstalls((rows) =>
        rows.map((row) => (row.id === id ? { ...row, phase: "Installed", percent: 100 } : row))
      );
      window.setTimeout(() => {
        setInstalls((rows) => rows.filter((row) => row.id !== id));
      }, 1600);
    }).then(keep);

    void listen<{ id: string; error: string }>("install-failed", (event) => {
      installNames.current.delete(event.payload.id);
      setError(event.payload.error);
      setInstalls((rows) => rows.filter((row) => row.id !== event.payload.id));
    }).then(keep);

    return () => stops.forEach((stop) => stop());
  }, []);

  const startInstall = useCallback(async (item: CatalogItem, target: string | null, sub: string | null = null) => {
    const id = `install:${item.slug}`;
    installNames.current.set(id, item.name);
    setInstalls((rows) =>
      rows.some((row) => row.id === id)
        ? rows
        : [
            ...rows,
            { id, slug: item.slug, name: item.name, phase: "Starting", percent: 2 }
          ]
    );
    try {
      await api.startInstall(item.slug, item.owner, target, item.market, sub);
    } catch (reason) {
      installNames.current.delete(id);
      setInstalls((rows) => rows.filter((row) => row.id !== id));
      setError(String(reason));
    }
  }, []);

  const enabled = state?.skills.filter((skill) => skill.enabled).length ?? 0;
  const showSetup = state !== null && (!state.setup.onboarded || setupOpen);

  return (
    <div className="app">
      <header className="topbar">
        <div className="brand">
          <span className="brand-mark" aria-hidden="true" />
        </div>

        <nav className="tabs" aria-label="Sections">
          <button className="tab" data-active={tab === "home"} onClick={() => setTab("home")}>
            Home
          </button>
          <button className="tab" data-active={tab === "skills"} onClick={() => setTab("skills")}>
            Skills <span className="tab-count">{enabled}</span>
          </button>
          <button className="tab" data-active={tab === "mcp"} onClick={() => setTab("mcp")}>
            MCP servers <span className="tab-count">{state?.servers.length ?? 0}</span>
          </button>
          <button className="tab" data-active={tab === "settings"} onClick={() => setTab("settings")}>
            {Icon.Settings} Settings
          </button>
        </nav>

        <div className="top-actions">
          <button
            className="ghost-btn"
            title={state?.store ?? "Open the library folder"}
            aria-label="Library folder"
            onClick={() => api.revealStore().catch((reason) => setError(String(reason)))}
          >
            {Icon.Folder}
            <span>Library</span>
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

      {update.view.status === "available" || update.view.status === "downloading" ? (
        <div className="update-banner" role="status">
          <div className="update-copy">
            <strong>Kilgharrah {update.view.version} is ready</strong>
            <span className="update-note">
              {update.view.status === "downloading"
                ? `Downloading the update, ${update.view.percent}%`
                : "A new signed build is available. Install it now or keep working."}
            </span>
          </div>
          {update.view.status === "downloading" && (
            <div
              className="update-bar"
              role="progressbar"
              aria-label="Downloading update"
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={update.view.percent}
            >
              <span style={{ width: `${update.view.percent}%` }} />
            </div>
          )}
          <div className="update-actions">
            {update.view.status === "available" && (
              <button className="primary compact" onClick={() => void update.install()}>
                {Icon.Download} Install and restart
              </button>
            )}
            <button className="ghost-btn" onClick={update.dismiss}>
              Later
            </button>
          </div>
        </div>
      ) : null}

      <main className="page">
        {state === null ? (
          <Loading />
        ) : tab === "home" ? (
          <HomeView state={state} onOpen={setTab} />
        ) : tab === "skills" ? (
          <SkillsView
            state={state}
            onState={setState}
            notify={setToast}
            onError={(message) => setError(message)}
            installs={installs}
            onInstall={startInstall}
          />
        ) : tab === "mcp" ? (
          <McpView
            state={state}
            onState={setState}
            notify={setToast}
            onError={(message) => setError(message)}
          />
        ) : (
          <SettingsView
            state={state}
            onState={setState}
            theme={theme}
            onTheme={setTheme}
            palette={palette}
            onPalette={setPalette}
            notify={setToast}
            onError={(message) => setError(message)}
            update={update.view}
            onCheckUpdates={() => void update.checkNow(false)}
            onInstallUpdate={() => void update.install()}
            onOpenSetup={() => setSetupOpen(true)}
          />
        )}
      </main>

      {showSetup && state && (
        <SetupOverlay
          state={state}
          onState={setState}
          onClose={() => setSetupOpen(false)}
          onError={(message) => setError(message)}
        />
      )}

      {installs.length > 0 && (
        <div className="install-dock" aria-live="polite">
          {installs.map((job) => (
            <div className="install-job" key={job.id}>
              <div className="install-job-head">
                <span className="install-job-name">{job.name}</span>
                <span className="install-job-pct">{job.percent}%</span>
              </div>
              <div
                className="install-bar"
                role="progressbar"
                aria-label={`Installing ${job.name}`}
                aria-valuemin={0}
                aria-valuemax={100}
                aria-valuenow={job.percent}
              >
                <span style={{ width: `${job.percent}%` }} />
              </div>
              <span className="install-phase">{job.phase}</span>
            </div>
          ))}
        </div>
      )}

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
