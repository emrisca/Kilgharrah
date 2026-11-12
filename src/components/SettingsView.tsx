import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { api } from "../api";
import type { AppState, Diagnostics, SkillUpdate } from "../types";
import { Icon } from "./Icons";
import { DragonEditor } from "./DragonEditor";
import { BUNDLED_FRAMES, frameFits, splitFrames } from "../dragon";

import type { Palette } from "../App";
import type { UpdateView } from "../updater";

type Theme = "dark" | "light";
type Panel = "general" | "profiles" | "agents" | "diagnostics" | "updates" | "credits";

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
  onState: (state: AppState) => void;
  theme: Theme;
  onTheme: (theme: Theme) => void;
  palette: Palette;
  onPalette: (palette: Palette) => void;
  notify: (message: string) => void;
  onError: (message: string) => void;
  update: UpdateView;
  onCheckUpdates: () => void;
  onInstallUpdate: () => void;
  onOpenSetup: () => void;
};

export function SettingsView({
  state,
  onState,
  theme,
  onTheme,
  palette,
  onPalette,
  notify,
  onError,
  update,
  onCheckUpdates,
  onInstallUpdate,
  onOpenSetup
}: Props) {
  const [panel, setPanel] = useState<Panel>("general");
  const [version, setVersion] = useState("");
  const [targets, setTargets] = useState<string[]>(() =>
    state.agentTargets.filter((target) => target.enabled).map((target) => target.id)
  );
  const [savingTargets, setSavingTargets] = useState(false);
  const [checks, setChecks] = useState<Diagnostics | null>(null);
  const [checking, setChecking] = useState(false);
  const [skillUpdates, setSkillUpdates] = useState<SkillUpdate[] | null>(null);
  const [checkingSkills, setCheckingSkills] = useState(false);
  const [updatingSlug, setUpdatingSlug] = useState<string | null>(null);
  const [updatingAll, setUpdatingAll] = useState(false);
  const [profileName, setProfileName] = useState("");
  const [dragonOpen, setDragonOpen] = useState(false);
  const [dragon, setDragon] = useState<{ count: number; bad: number } | null>(null);
  const [exporting, setExporting] = useState(false);
  const [applying, setApplying] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);
  const [deleting, setDeleting] = useState<string | null>(null);

  useEffect(() => {
    getVersion()
      .then(setVersion)
      .catch(() => setVersion(""));
  }, []);

  useEffect(() => {
    let alive = true;
    const load = () => {
      api
        .readDragon()
        .then((saved) => {
          if (!alive) return;
          const frames = saved ? splitFrames(saved) : BUNDLED_FRAMES;
          setDragon({ count: frames.length, bad: frames.findIndex((frame) => !frameFits(frame)) });
        })
        .catch(() => {
          if (alive) setDragon(null);
        });
    };
    load();
    window.addEventListener("kilgharrah:dragon", load);
    return () => {
      alive = false;
      window.removeEventListener("kilgharrah:dragon", load);
    };
  }, []);

  const runDiagnostics = async () => {
    setChecking(true);
    try {
      setChecks(await api.diagnostics());
    } catch (reason) {
      onError(String(reason));
    } finally {
      setChecking(false);
    }
  };

  const runSkillUpdates = async () => {
    setCheckingSkills(true);
    try {
      setSkillUpdates(await api.checkUpdates());
    } catch (reason) {
      onError(String(reason));
    } finally {
      setCheckingSkills(false);
    }
  };

  const updateSkill = async (row: SkillUpdate) => {
    setUpdatingSlug(row.slug);
    try {
      onState(await api.updateSkill(row.category, row.slug));
      notify(`${row.name} updated`);
      setSkillUpdates(await api.checkUpdates());
    } catch (reason) {
      onError(String(reason));
    } finally {
      setUpdatingSlug(null);
    }
  };

  const updateAll = async () => {
    setUpdatingAll(true);
    try {
      const report = await api.updateAllSkills();
      onState(report.state);
      notify(
        report.updated === 0
          ? "No skills needed an update"
          : `${report.updated} skill${report.updated === 1 ? "" : "s"} updated`
      );
      if (report.failed.length > 0) onError(report.failed.join(" · "));
      setSkillUpdates(await api.checkUpdates());
    } catch (reason) {
      onError(String(reason));
    } finally {
      setUpdatingAll(false);
    }
  };

  const exportProfile = async () => {
    if (!profileName.trim() || exporting) return;
    setExporting(true);
    try {
      const path = await api.exportProfile(profileName.trim());
      notify(`Profile saved to ${path}`);
      setProfileName("");
      onState(await api.snapshot());
    } catch (reason) {
      onError(String(reason));
    } finally {
      setExporting(false);
    }
  };

  const applyProfile = async (name: string) => {
    setApplying(name);
    try {
      onState(await api.applyProfile(name));
      notify(`${name} applied`);
    } catch (reason) {
      onError(String(reason));
    } finally {
      setApplying(null);
    }
  };

  const deleteProfile = async (name: string) => {
    setDeleting(name);
    try {
      onState(await api.deleteProfile(name));
      notify(`${name} deleted`);
      setConfirmDelete(null);
    } catch (reason) {
      onError(String(reason));
    } finally {
      setDeleting(null);
    }
  };

  const saveTargets = async () => {
    setSavingTargets(true);
    try {
      onState(await api.setAgentTargets(targets));
      notify("Agent mirrors saved");
    } catch (reason) {
      onError(String(reason));
    } finally {
      setSavingTargets(false);
    }
  };

  useEffect(() => {
    if (panel === "diagnostics" && !checks && !checking) void runDiagnostics();
  }, [panel]);

  const savedTargets = state.agentTargets
    .filter((target) => target.enabled)
    .map((target) => target.id);
  const targetsDirty =
    [...targets].sort().join(",") !== [...savedTargets].sort().join(",");
  const pendingUpdates = (skillUpdates ?? []).filter((row) => row.status === "available");
  const updateErrors = (skillUpdates ?? []).filter((row) => row.status === "error");
  const unknownUpdates = (skillUpdates ?? []).filter((row) => row.status === "unknown");

  const open = (url: string) => api.openUrl(url).catch((reason) => onError(String(reason)));

  const copy = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      notify("Copied to the clipboard");
    } catch {
      onError("The clipboard is not available here.");
    }
  };

  const installed = state.skills;

  return (
    <div className="view settings">
      <div className="filters">
        <div className="segmented" role="tablist" aria-label="Settings section">
          <button data-active={panel === "general"} onClick={() => setPanel("general")}>
            General
          </button>
          <button data-active={panel === "profiles"} onClick={() => setPanel("profiles")}>
            Profiles
          </button>
          <button data-active={panel === "agents"} onClick={() => setPanel("agents")}>
            Agents
          </button>
          <button data-active={panel === "diagnostics"} onClick={() => setPanel("diagnostics")}>
            Diagnostics
          </button>
          <button data-active={panel === "updates"} onClick={() => setPanel("updates")}>
            Updates
          </button>
          <button data-active={panel === "credits"} onClick={() => setPanel("credits")}>
            Credits
          </button>
        </div>
        <span className="spacer" />
        <span className="filter-note">Kilgharrah {version || "…"}</span>
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
              <h3>Home animation</h3>
              <p>Step through the dragon frames and fix stray characters.</p>
            </div>
            <div className="filters">
              <button className="ghost-btn" onClick={() => setDragonOpen(true)}>
                {Icon.Edit} Edit dragon frames
              </button>
              <span className="spacer" />
              {dragon === null ? (
                <span className="sheet-note">Reading frames</span>
              ) : dragon.bad === -1 ? (
                <span className="chip">
                  {dragon.count} frames, 23 by 70
                </span>
              ) : (
                <span className="chip accent">Frame {dragon.bad + 1} needs a fix</span>
              )}
            </div>
          </section>

          <section className="panel">
            <div className="panel-head">
              <h3>Setup</h3>
              <p>Where the skills live and which agent loads them.</p>
            </div>
            <div className="facts">
              <Fact label="Agent" value={state.setup.agentName} />
              <Fact
                label="Scope"
                value={state.setup.scope === "project" ? "Project folder" : "Home"}
              />
              <Fact
                label="Project folder"
                value={state.setup.projectRoot ?? "not set"}
                mono
              />
              <Fact label="Profile" value={state.setup.profile ?? "blank"} />
              <Fact label="Library" value={state.store} mono />
              <Fact label="Skills folder" value={state.skillsRoot} mono />
              <Fact label="MCP config" value={state.mcpPath} mono />
            </div>
            <div className="filters">
              <span className="chip">{installed.length} installed</span>
              <span className="chip">{installed.filter((skill) => skill.enabled).length} enabled</span>
              <span className="chip">{state.servers.length} MCP servers</span>
              <span className="spacer" />
              <button className="ghost-btn" onClick={onOpenSetup}>
                {Icon.Settings} Run setup again
              </button>
              <button
                className="ghost-btn"
                onClick={() => api.revealStore().catch((reason) => onError(String(reason)))}
              >
                {Icon.Folder} Show library
              </button>
            </div>
          </section>
        </>
      ) : panel === "profiles" ? (
        <>
          <section className="panel">
            <div className="panel-head">
              <h3>Profiles</h3>
              <p>
                A profile is a skill set and its MCP servers. Applying one copies its skills into
                the library, where you can edit or remove them. Deleting a profile removes only its
                file, not the skills already in the library.
              </p>
            </div>
            <div className="diag-list">
              {state.profiles.map((entry) => {
                const active = state.setup.profile === entry.name;
                return (
                  <div className="diag-row" key={entry.name}>
                    <span className={`diag-dot ${active ? "ok" : "warn"}`} aria-hidden="true" />
                    <div className="target-copy">
                      <strong>{entry.name}</strong>
                      <span className="sheet-note">
                        {entry.skills} skill{entry.skills === 1 ? "" : "s"} · {entry.servers} MCP
                        server{entry.servers === 1 ? "" : "s"}
                        {entry.builtin ? " · built in" : ""}
                      </span>
                    </div>
                    <span className="spacer" />
                    {active ? (
                      <span className="chip accent">Active</span>
                    ) : (
                      <button
                        className="ghost-btn"
                        disabled={applying !== null || deleting !== null}
                        onClick={() => void applyProfile(entry.name)}
                      >
                        {applying === entry.name ? Icon.Spinner : Icon.Download} Apply
                      </button>
                    )}
                    {!entry.builtin &&
                      (confirmDelete === entry.name ? (
                        <>
                          <button
                            className="ghost-btn danger"
                            disabled={deleting !== null}
                            onClick={() => void deleteProfile(entry.name)}
                            title={`Delete the ${entry.name} profile file`}
                          >
                            {deleting === entry.name ? Icon.Spinner : "Delete"}
                          </button>
                          <button
                            className="ghost-btn"
                            onClick={() => setConfirmDelete(null)}
                          >
                            Keep
                          </button>
                        </>
                      ) : (
                        <button
                          className="ghost-btn"
                          aria-label={`Delete ${entry.name}`}
                          title="Delete this profile file"
                          onClick={() => setConfirmDelete(entry.name)}
                        >
                          {Icon.Trash}
                        </button>
                      ))}
                  </div>
                );
              })}
            </div>
          </section>

          <section className="panel">
            <div className="panel-head">
              <h3>Export the current setup</h3>
              <p>
                Saves the installed skills and MCP servers as a JSON profile in the profiles
                folder. Use it on another machine, or keep it as a backup.
              </p>
            </div>
            <div className="import-row">
              <input
                className="input"
                value={profileName}
                onChange={(event) => setProfileName(event.target.value)}
                placeholder="Profile name"
              />
              <button
                className="primary compact"
                disabled={!profileName.trim() || exporting}
                onClick={() => void exportProfile()}
              >
                {exporting ? Icon.Spinner : Icon.Download} Export
              </button>
            </div>
          </section>
        </>
      ) : panel === "agents" ? (
        <>
          <section className="panel">
            <div className="panel-head">
              <h3>Agent mirrors</h3>
              <p>
                Your main agent always gets every enabled skill. Turn on another agent here to link
                the same skills into its folder. Nothing is copied; the folders point at the
                library.
              </p>
            </div>
            <div className="target-list">
              <div className="target-row">
                <span className="diag-dot ok" aria-hidden="true" />
                <div className="target-copy">
                  <strong>{state.setup.agentName}</strong>
                  <span className="sheet-note mono">{state.skillsRoot}</span>
                </div>
                <span className="spacer" />
                <span className="chip">
                  {state.skills.filter((skill) => skill.enabled).length} linked
                </span>
              </div>
              {state.agentTargets
                .filter((target) => target.id !== state.setup.agent)
                .map((target) => {
                const on = targets.includes(target.id);
                return (
                  <label className="target-row" key={target.id}>
                    <input
                      type="checkbox"
                      checked={on}
                      onChange={() =>
                        setTargets((previous) =>
                          on ? previous.filter((value) => value !== target.id) : [...previous, target.id]
                        )
                      }
                      aria-label={`Mirror to ${target.name}`}
                    />
                    <div className="target-copy">
                      <strong>{target.name}</strong>
                      <span className="sheet-note mono">{target.path}</span>
                    </div>
                    <span className="spacer" />
                    <span className="chip">{on ? `${target.linked} linked` : "off"}</span>
                  </label>
                );
              })}
            </div>
            <p className="sheet-note">
              MCP servers are mirrored to the same agents: Claude uses `.claude.json` or `.mcp.json`,
              Gemini uses `settings.json`, and Codex uses a marked block in `config.toml`.
            </p>
            <div className="filters">
              <button
                className="primary compact"
                disabled={!targetsDirty || savingTargets}
                onClick={() => void saveTargets()}
              >
                {savingTargets ? Icon.Spinner : Icon.Check} Save mirrors
              </button>
              <span className="sheet-note">
                Saving also repairs missing links for skills that are already on.
              </span>
            </div>
          </section>

          <section className="panel">
            <div className="panel-head">
              <h3>What lands where</h3>
              <p>
                Each agent reads `SKILL.md` from its own skills folder. A skill that is off stays
                in the library and is removed from every mirror, so turning an agent on never
                enables a skill you disabled.
              </p>
            </div>
            <div className="filters">
              <span className="chip">the library stays the source</span>
              <span className="chip">mirrors are links</span>
              <span className="chip">Windows copies instead</span>
            </div>
          </section>
        </>
      ) : panel === "diagnostics" ? (
        <>
          <section className="panel">
            <div className="panel-head">
              <h3>Health checks</h3>
              <p>
                These run on this machine only. They look at the library, the agent folder, your
                MCP config, links, and install records.
              </p>
            </div>
            {checks ? (
              <div className="diag-list">
                {checks.checks.map((check) => (
                  <div className="diag-row" key={check.id}>
                    <span className={`diag-dot ${check.status}`} aria-hidden="true" />
                    <div className="target-copy">
                      <strong>{check.label}</strong>
                      <span className="sheet-note">{check.detail}</span>
                    </div>
                    <span className="spacer" />
                    <span className="chip">{check.status === "error" ? "fix" : check.status}</span>
                  </div>
                ))}
              </div>
            ) : (
              <div className="skel skel-line" />
            )}
            <div className="filters">
              <button className="ghost-btn" disabled={checking} onClick={() => void runDiagnostics()}>
                {Icon.Refresh} {checking ? "Checking" : "Run checks"}
              </button>
              {checks && <span className="chip">{checks.ok ? "All clear" : "Needs a look"}</span>}
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
              <Fact label="Installed" value={version || "Unknown"} />
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

          <section className="panel">
            <div className="panel-head">
              <h3>Skill updates</h3>
              <p>
                Kilgharrah asks ClawHub for the published version of each skill it installed. A
                skill with no install record, or one from another source, is reported rather than
                guessed.
              </p>
            </div>
            {skillUpdates === null ? (
              <p className="sheet-note">Run a check to compare installed skills with ClawHub.</p>
            ) : pendingUpdates.length === 0 && updateErrors.length === 0 ? (
              <p className="sheet-note">No published updates found for the checked skills.</p>
            ) : (
              <div className="diag-list">
                {pendingUpdates.map((row) => (
                  <div className="diag-row" key={`${row.category}:${row.slug}`}>
                    <span className="diag-dot warn" aria-hidden="true" />
                    <div className="target-copy">
                      <strong>{row.name}</strong>
                      <span className="sheet-note">
                        {row.current ? `v${row.current}` : "unversioned"} to v{row.latest}
                      </span>
                    </div>
                    <span className="spacer" />
                    <button
                      className="ghost-btn"
                      disabled={updatingSlug === row.slug}
                      onClick={() => void updateSkill(row)}
                    >
                      {updatingSlug === row.slug ? Icon.Spinner : Icon.Download} Update
                    </button>
                  </div>
                ))}
                {updateErrors.map((row) => (
                  <div className="diag-row" key={`${row.category}:${row.slug}`}>
                    <span className="diag-dot error" aria-hidden="true" />
                    <div className="target-copy">
                      <strong>{row.name}</strong>
                      <span className="sheet-note">{row.detail}</span>
                    </div>
                  </div>
                ))}
              </div>
            )}
            {unknownUpdates.length > 0 && (
              <p className="sheet-note">
                {unknownUpdates.length} skill{unknownUpdates.length === 1 ? "" : "s"} cannot be
                checked: no install record or no update feed for the source.
              </p>
            )}
            <div className="filters">
              <button
                className="ghost-btn"
                disabled={checkingSkills}
                onClick={() => void runSkillUpdates()}
              >
                {Icon.Refresh} {checkingSkills ? "Checking" : "Check skill updates"}
              </button>
              {skillUpdates && (
                <span className="chip">
                  {pendingUpdates.length} update{pendingUpdates.length === 1 ? "" : "s"}
                </span>
              )}
              <span className="spacer" />
              <button
                className="primary compact"
                disabled={updatingAll || pendingUpdates.length === 0}
                onClick={() => void updateAll()}
              >
                {updatingAll ? Icon.Spinner : Icon.Download} Update all
              </button>
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
              <span className="credit-mark">AG</span>
              <div className="credit-main">
                <strong>Main agent</strong>
                <span className="sheet-note">Loads the skills and MCP servers this app manages.</span>
              </div>
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

      <DragonEditor
        open={dragonOpen}
        onClose={() => setDragonOpen(false)}
        notify={notify}
        onError={onError}
      />
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
