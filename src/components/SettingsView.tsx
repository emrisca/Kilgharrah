import { useState } from "react";
import { api } from "../api";
import type { AppState } from "../types";
import { Icon } from "./Icons";

type Theme = "dark" | "light";
type Panel = "general" | "credits";

const VERSION = "0.1.0";

type Props = {
  state: AppState;
  theme: Theme;
  onTheme: (theme: Theme) => void;
  notify: (message: string) => void;
  onError: (message: string) => void;
};

export function SettingsView({ state, theme, onTheme, notify, onError }: Props) {
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
      ) : (
        <>
          <section className="panel">
            <div className="panel-head">
              <h3>ClawHub</h3>
              <p>
                The public registry Kilgharrah searches for skills and plugin packages. Skill
                listings, versions and files come from the ClawHub API.
              </p>
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
              <p>Kilgharrah is a small local app. It sends nothing anywhere except ClawHub requests.</p>
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
