import { useState } from "react";
import { api } from "../api";
import type { AppState } from "../types";
import { Icon } from "./Icons";

import type { Palette } from "../App";
import type { UpdateView } from "../updater";

type Theme = "dark" | "light";
type Panel = "general" | "updates" | "credits";

const VERSION = "0.1.0";

const PALETTES: { id: Palette; name: string; note: string }[] = [
  { id: "aurora", name: "Aurora", note: "Green" },
  { id: "ember", name: "Ember", note: "Amber" },
  { id: "tide", name: "Tide", note: "Blue" },
  { id: "orchid", name: "Orchid", note: "Violet" },
  { id: "rose", name: "Rose", note: "Pink" },
  { id: "merlin", name: "Merlin", note: "Gold" }
];

type Props = {
  state: AppState;
  theme: Theme;
  onTheme: (theme: Theme) => void;
  palette: Palette;
  onPalette: (palette: Palette) => void;
  notify: (message: string) => void;
  onError: (message: string) => void;
  update: UpdateView;
  onCheckUpdates: () => void;
  onInstallUpdate: () => void;
};

export function SettingsView({
  state,
  theme,
  onTheme,
  palette,
  onPalette,
  notify,
  onError,
  update,
  onCheckUpdates,
  onInstallUpdate
}: Props) {
  const [panel, setPanel] = useState<Panel>("general");

  const open = (url: string) => api.openUrl(url).catch((reason) => onError(String(reason)));

  const copy = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      notify("Copied to the clipboard");
    } catch {
      onError("The clipboard is not available here.");
    }
  };

  const storeSkills = state.skills.filter((skill) => skill.source === "store");
  const hermesSkills = state.skills.filter((skill) => skill.source === "hermes");

  return (
    <div className="view settings">
      <div className="filters">
        <div className="segmented" role="tablist" aria-label="Settings section">
          <button data-active={panel === "general"} onClick={() => setPanel("general")}>
            General
          </button>
          <button data-active={panel === "updates"} onClick={() => setPanel("updates")}>
            Updates
          </button>
          <button data-active={panel === "credits"} onClick={() => setPanel("credits")}>
            Credits
          </button>
        </div>
        <span className="spacer" />
        <span className="filter-note">Kilgharrah {VERSION}</span>
      </div>

      {panel === "general" ? (
        <>
          <section className="panel">
            <div className="panel-head">
              <h3>Appearance</h3>
              <p>The window follows your choice, not the system.</p>
            </div>
            <div className="segmented" role="group" aria-label="Appearance">
              <button data-active={theme === "dark"} onClick={() => onTheme("dark")}>
                Dark
              </button>
              <button data-active={theme === "light"} onClick={() => onTheme("light")}>
                Light
              </button>
            </div>
            <div className="palette-head">
              <span className="sheet-label">Colour</span>
              <span className="sheet-note">The accent and the surfaces shift together.</span>
            </div>
            <div className="palette-row" role="group" aria-label="Colour palette">
              {PALETTES.map((entry) => (
                <button
                  key={entry.id}
                  className="palette"
                  data-active={palette === entry.id}
                  data-palette-swatch={entry.id}
                  onClick={() => onPalette(entry.id)}
                  aria-pressed={palette === entry.id}
                >
                  <span className="palette-swatch" aria-hidden="true" />
                  <span className="palette-name">{entry.name}</span>
                  <span className="palette-note">{entry.note}</span>
                </button>
              ))}
            </div>
          </section>

          <section className="panel">
            <div className="panel-head">
              <h3>Library</h3>
              <p>Where Kilgharrah reads skills from and how the CLI sees them.</p>
            </div>
            <div className="facts">
              <Fact label="Store" value={state.store} mono />
              <Fact label="CLI links" value={state.skillsRoot} mono />
              <Fact label="Hermes" value={state.hermesRoot} mono />
              <Fact label="MCP config" value={state.mcpPath} mono />
            </div>
            <div className="filters">
              <span className="chip">{storeSkills.length} in the store</span>
              <span className="chip">{storeSkills.filter((skill) => skill.enabled).length} enabled</span>
              <span className="chip">{hermesSkills.length} from Hermes</span>
              <span className="chip">{state.servers.length} MCP servers</span>
              <span className="spacer" />
              <button
                className="ghost-btn"
                onClick={() => api.revealStore().catch((reason) => onError(String(reason)))}
              >
                {Icon.Folder} Show store
              </button>
            </div>
          </section>

          <section className="panel">
            <div className="panel-head">
              <h3>Sources</h3>
              <p>
                The store is the copy Kilgharrah edits and links into the CLI. Hermes keeps its own
                library, which Kilgharrah reads and can import from.
              </p>
            </div>
            <div className="filters">
              <button className="ghost-btn" onClick={() => void copy(state.store)}>
                {Icon.Copy} Copy store path
              </button>
              <button className="ghost-btn" onClick={() => void copy(state.hermesRoot)}>
                {Icon.Copy} Copy Hermes path
              </button>
            </div>
          </section>
        </>
      ) : panel === "updates" ? (
        <>
          <section className="panel">
            <div className="panel-head">
              <h3>App updates</h3>
              <p>
                Kilgharrah checks GitHub for a signed build and installs it over the current one.
                The check runs in the background every few hours.
              </p>
            </div>
            <div className="facts">
              <Fact label="Installed" value={VERSION} />
              <Fact label="Channel" value="Stable" />
              <Fact
                label="Available"
                value={
                  update.status === "available" || update.status === "downloading"
                    ? update.version ?? "Unknown"
                    : "None found"
                }
              />
            </div>
            <div className="update-status">
              <span>{updateLabel(update)}</span>
            </div>
            {(update.status === "downloading" || update.status === "ready") && (
              <div
                className="install-bar"
                role="progressbar"
                aria-label="Downloading update"
                aria-valuemin={0}
                aria-valuemax={100}
                aria-valuenow={update.percent}
              >
                <span style={{ width: `${update.percent}%` }} />
              </div>
            )}
            <div className="filters">
              <button className="ghost-btn" disabled={update.status === "checking"} onClick={onCheckUpdates}>
                {Icon.Refresh} {update.status === "checking" ? "Checking" : "Check for updates"}
              </button>
              {update.status === "available" && (
                <button className="primary compact" onClick={onInstallUpdate}>
                  {Icon.Download} Install {update.version}
                </button>
              )}
            </div>
          </section>
        </>
      ) : (
        <>
          <section className="panel">
            <div className="panel-head">
              <h3>Marketplace sources</h3>
              <p>
                Kilgharrah reads ClawHub for published skills and plugin packages, and
                agenticskills.io for its curated, ranked directory. A skill that ships in both
                shows once.
              </p>
            </div>
            <div className="credit">
              <span className="credit-mark">AS</span>
              <div className="credit-main">
                <strong>AgenticSkills</strong>
                <span className="sheet-note">agenticskills.io · ranked directory</span>
              </div>
              <span className="spacer" />
              <button className="ghost-btn" onClick={() => open("https://agenticskills.io/skills")}>
                {Icon.External} Open
              </button>
            </div>
            <div className="credit">
              <span className="credit-mark">CH</span>
              <div className="credit-main">
                <strong>ClawHub</strong>
                <span className="sheet-note">clawhub.ai · registry</span>
              </div>
              <span className="spacer" />
              <button className="ghost-btn" onClick={() => open("https://clawhub.ai")}>
                {Icon.External} Open
              </button>
            </div>
            <p className="sheet-note">
              Every skill belongs to the author who published it. Open a skill on ClawHub to read its
              licence and changelog.
            </p>
          </section>

          <section className="panel">
            <div className="panel-head">
              <h3>Built with</h3>
              <p>
                Kilgharrah is a small local app. It sends nothing anywhere except marketplace
                requests.
              </p>
            </div>
            <div className="credit">
              <span className="credit-mark">FF</span>
              <div className="credit-main">
                <strong>Freebuff CLI</strong>
                <span className="sheet-note">Loads the skills and MCP servers this app manages.</span>
              </div>
              <span className="spacer" />
              <button className="ghost-btn" onClick={() => open("https://freebuff.com")}>
                {Icon.External} Open
              </button>
            </div>
            <div className="filters">
              <span className="chip">Tauri 2</span>
              <span className="chip">React 18</span>
              <span className="chip">Vite</span>
            </div>
          </section>

          <section className="panel">
            <div className="panel-head">
              <h3>Credits and licences</h3>
              <p>
                Placeholder for the full credit list. It will name the skills, icons and libraries
                that ship with the app.
              </p>
            </div>
          </section>
        </>
      )}
    </div>
  );
}

function updateLabel(update: UpdateView): string {
  switch (update.status) {
    case "checking":
      return "Checking GitHub for a new build.";
    case "available":
      return `Version ${update.version} is ready to install.`;
    case "downloading":
      return `Downloading, ${update.percent}%.`;
    case "ready":
      return "Downloaded. Restarting to finish.";
    case "error":
      return update.message ?? "The update check failed.";
    default:
      return update.message ?? "No update check has run yet.";
  }
}

function Fact({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="fact">
      <span className="fact-label">{label}</span>
      <span className={mono ? "fact-value mono" : "fact-value"} title={value}>
        {value}
      </span>
    </div>
  );
}
