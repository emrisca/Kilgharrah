import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api } from "../api";
import type { AppState } from "../types";
import { Icon } from "./Icons";

const AGENTS: { id: string; name: string; folder: string; note: string }[] = [
  { id: "freebuff", name: "Freebuff CLI", folder: ".agents/skills", note: "Default" },
  { id: "claude", name: "Claude Code", folder: ".claude/skills", note: "" },
  { id: "codex", name: "Codex CLI", folder: ".codex/skills", note: "" },
  { id: "gemini", name: "Gemini CLI", folder: ".gemini/skills", note: "" }
];

type Props = {
  state: AppState;
  onState: (state: AppState) => void;
  onClose: () => void;
  onError: (message: string) => void;
};

export function SetupOverlay({ state, onState, onClose, onError }: Props) {
  const setup = state.setup;
  const [step, setStep] = useState(0);
  const [root, setRoot] = useState(setup.projectRoot ?? "");
  const [agent, setAgent] = useState(setup.agent || "freebuff");
  const [profile, setProfile] = useState(setup.profile ?? "none");
  const [saving, setSaving] = useState(false);

  const chosen = AGENTS.find((entry) => entry.id === agent) ?? AGENTS[0];
  const projectPath = root.trim().replace(/\/$/, "");
  const target = projectPath ? `${projectPath}/${chosen.folder}` : `~/${chosen.folder}`;

  const browse = async () => {
    try {
      const picked = await open({
        directory: true,
        multiple: false,
        title: "Choose a project folder"
      });
      if (typeof picked === "string") setRoot(picked);
    } catch (reason) {
      onError(String(reason));
    }
  };

  const finish = async () => {
    setSaving(true);
    try {
      const next = await api.completeSetup({
        projectRoot: projectPath || null,
        agent,
        profile: profile === "none" ? null : profile
      });
      onState(next);
      onClose();
    } catch (reason) {
      onError(String(reason));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="setup" role="dialog" aria-label="Set up Kilgharrah">
      <div className="setup-card">
        <header className="setup-head">
          <div>
            <span className="eyebrow">First run</span>
            <h1>Set up Kilgharrah</h1>
          </div>
        </header>

        <ol className="setup-steps" aria-label="Setup steps">
          {["Where", "Agent", "Profile"].map((label, index) => (
            <li key={label} data-active={step === index} data-done={step > index}>
              <span>{index + 1}</span>
              {label}
            </li>
          ))}
        </ol>

        <div className="setup-body">
          {step === 0 && (
            <>
              <h2>Where do you work?</h2>
              <p className="sheet-note">
                Leave the field empty to keep one library in your home folder. Pick a project folder
                to keep its own skills and MCP config inside that folder.
              </p>
              <div className="field">
                <span id="setup-root-label">Project folder</span>
                <div className="path-row">
                  <input
                    className="input mono"
                    aria-labelledby="setup-root-label"
                    value={root}
                    onChange={(event) => setRoot(event.target.value)}
                    placeholder="Leave empty for Home, or /Users/you/code/app"
                    spellCheck={false}
                    autoFocus
                  />
                  <button type="button" className="ghost-btn" onClick={() => void browse()}>
                    Browse
                  </button>
                </div>
                <span className="sheet-note">
                  Leave empty to use Home. The folder must already exist.
                </span>
              </div>
            </>
          )}

          {step === 1 && (
            <>
              <h2>Which agent loads the skills?</h2>
              <p className="sheet-note">
                Kilgharrah links each enabled skill into this agent's folder. You can add more
                agents later in Settings.
              </p>
              <div className="setup-agents">
                {AGENTS.map((entry) => (
                  <button
                    key={entry.id}
                    className="setup-agent"
                    data-active={agent === entry.id}
                    onClick={() => setAgent(entry.id)}
                  >
                    <span className="target-copy">
                      <strong>{entry.name}</strong>
                      <span className="sheet-note mono">
                        {projectPath ? `${projectPath}/${entry.folder}` : `~/${entry.folder}`}
                      </span>
                    </span>
                    {entry.note && <span className="chip">{entry.note}</span>}
                    {agent === entry.id && <span className="chip accent">Selected</span>}
                  </button>
                ))}
              </div>
            </>
          )}

          {step === 2 && (
            <>
              <h2>Start with a profile?</h2>
              <p className="sheet-note">
                A profile copies a skill set and its MCP servers into your library. Every file
                stays editable after that. Choose Blank to start with nothing.
              </p>
              <div className="setup-agents">
                <button
                  className="setup-agent"
                  data-active={profile === "none"}
                  onClick={() => setProfile("none")}
                >
                  <span className="target-copy">
                    <strong>Blank</strong>
                    <span className="sheet-note">Add skills from the marketplace or write your own.</span>
                  </span>
                  {profile === "none" && <span className="chip accent">Selected</span>}
                </button>
                {state.profiles.map((entry) => (
                  <button
                    key={entry.name}
                    className="setup-agent"
                    data-active={profile === entry.name}
                    onClick={() => setProfile(entry.name)}
                  >
                    <span className="target-copy">
                      <strong>{entry.name}</strong>
                      <span className="sheet-note">
                        {entry.skills} skill{entry.skills === 1 ? "" : "s"} and {entry.servers} MCP
                        server{entry.servers === 1 ? "" : "s"}
                      </span>
                      {entry.skillNames.length > 0 && (
                        <span className="sheet-note">
                          {entry.skillNames.slice(0, 8).join(", ")}
                          {entry.skillNames.length > 8
                            ? ` and ${entry.skillNames.length - 8} more`
                            : ""}
                        </span>
                      )}
                      {entry.serverNames.length > 0 && (
                        <span className="sheet-note mono">{entry.serverNames.join(", ")}</span>
                      )}
                    </span>
                    {entry.builtin && <span className="chip">Built in</span>}
                    {profile === entry.name && <span className="chip accent">Selected</span>}
                  </button>
                ))}
              </div>
              <div className="facts">
                <div className="fact">
                  <span className="fact-label">Skills folder</span>
                  <span className="fact-value mono" title={target}>
                    {target}
                  </span>
                </div>
                <div className="fact">
                  <span className="fact-label">Stored in</span>
                  <span className="fact-value mono" title={state.store}>
                    {state.store}
                  </span>
                </div>
              </div>
            </>
          )}
        </div>

        <footer className="setup-actions">
          <button
            className="ghost-btn"
            onClick={
              step === 0
                ? () => (setup.onboarded ? onClose() : void finish())
                : () => setStep(step - 1)
            }
          >
            {step === 0 ? (setup.onboarded ? "Close" : "Skip") : "Back"}
          </button>
          <span className="spacer" />
          {step < 2 ? (
            <button className="primary" onClick={() => setStep(step + 1)}>
              Continue {Icon.ChevronRight}
            </button>
          ) : (
            <button className="primary" disabled={saving} onClick={() => void finish()}>
              {saving ? Icon.Spinner : Icon.Check} Finish setup
            </button>
          )}
        </footer>
      </div>
    </div>
  );
}
