import { useState } from "react";
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
  const [project, setProject] = useState(Boolean(setup.projectRoot));
  const [root, setRoot] = useState(setup.projectRoot ?? "");
  const [agent, setAgent] = useState(setup.agent || "freebuff");
  const [profile, setProfile] = useState(setup.profile ?? "none");
  const [saving, setSaving] = useState(false);

  const chosen = AGENTS.find((entry) => entry.id === agent) ?? AGENTS[0];
  const target = project
    ? `${root.trim().replace(/\/$/, "")}/${chosen.folder}`
    : `~/${chosen.folder}`;

  const finish = async () => {
    setSaving(true);
    try {
      const next = await api.completeSetup({
        projectRoot: project ? root.trim() : null,
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

  const canContinue = step !== 0 || !project || root.trim().length > 0;

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
                Home keeps one library for every project. A project folder keeps its own skills and
                MCP config inside that folder.
              </p>
              <div className="segmented" role="group" aria-label="Scope">
                <button data-active={!project} onClick={() => setProject(false)}>
                  Home
                </button>
                <button data-active={project} onClick={() => setProject(true)}>
                  Project folder
                </button>
              </div>
              {project && (
                <label className="field">
                  <span>Project folder</span>
                  <input
                    className="input mono"
                    value={root}
                    onChange={(event) => setRoot(event.target.value)}
                    placeholder="/Users/you/code/app or ~/code/app"
                    spellCheck={false}
                    autoFocus
                  />
                  <span className="sheet-note">The folder must already exist.</span>
                </label>
              )}
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
                        {project ? `${root.trim().replace(/\/$/, "")}/${entry.folder}` : `~/${entry.folder}`}
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
            <button
              className="primary"
              disabled={!canContinue}
              onClick={() => setStep(step + 1)}
            >
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
