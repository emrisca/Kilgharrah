import { useEffect, useMemo, useRef, useState } from "react";
import { api } from "../api";
import type { AppState, CatalogItem, Skill } from "../types";
import { Icon } from "./Icons";
import { Sheet } from "./Sheet";

type Scope = "all" | "installed" | "market";

type Props = {
  state: AppState;
  onState: (state: AppState) => void;
  notify: (message: string) => void;
  onError: (message: string) => void;
};

export function SkillsView({ state, onState, notify, onError }: Props) {
  const [scope, setScope] = useState<Scope>("all");
  const [query, setQuery] = useState("");
  const [category, setCategory] = useState("all");
  const [market, setMarket] = useState<CatalogItem[]>([]);
  const [marketLoading, setMarketLoading] = useState(true);
  const [nonce, setNonce] = useState(0);
  const [busy, setBusy] = useState<string | null>(null);
  const [selected, setSelected] = useState<Skill | null>(null);
  const [selectedItem, setSelectedItem] = useState<CatalogItem | null>(null);
  const [compose, setCompose] = useState(false);
  const searchRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      const typing =
        target?.tagName === "INPUT" || target?.tagName === "TEXTAREA" || target?.tagName === "SELECT";
      if (event.key === "/" && !typing && !event.metaKey && !event.ctrlKey) {
        event.preventDefault();
        searchRef.current?.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  useEffect(() => {
    if (scope === "installed") return;
    const trimmed = query.trim();
    if (trimmed.length === 1) return;
    let alive = true;
    const timer = setTimeout(
      async () => {
        setMarketLoading(true);
        try {
          const items = await api.searchMarket(trimmed, 24);
          if (alive) setMarket(items);
        } catch (reason) {
          onError(String(reason));
        } finally {
          if (alive) setMarketLoading(false);
        }
      },
      trimmed ? 320 : 0
    );
    return () => {
      alive = false;
      clearTimeout(timer);
    };
  }, [query, scope, nonce]);

  const skills = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return state.skills.filter((skill) => {
      if (category !== "all" && skill.category !== category) return false;
      if (!needle) return true;
      return `${skill.name} ${skill.slug} ${skill.description}`.toLowerCase().includes(needle);
    });
  }, [state.skills, query, category]);

  const groups = state.categories
    .map((entry) => ({ entry, skills: skills.filter((skill) => skill.category === entry.id) }))
    .filter((group) => group.skills.length > 0);

  const installedSlugs = new Set(state.skills.map((skill) => skill.slug));
  const visibleMarket = scope === "installed" ? [] : market;
  const showInstalled = scope !== "market";
  const showMarket = scope !== "installed";

  const toggle = async (skill: Skill) => {
    try {
      onState(await api.setSkillEnabled(skill.category, skill.slug, !skill.enabled));
    } catch (reason) {
      onError(String(reason));
    }
  };

  const install = async (item: CatalogItem, target?: string) => {
    setBusy(item.id);
    try {
      const next = await api.installMarket(item.slug, target ?? null);
      onState(next);
      setSelectedItem(null);
      notify(`${item.name} installed and enabled`);
    } catch (reason) {
      onError(String(reason));
    } finally {
      setBusy(null);
    }
  };

  const copy = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      notify("Copied to the clipboard");
    } catch {
      onError("The clipboard is not available here.");
    }
  };

  const sheetOpen = Boolean(selected || selectedItem);
  const closeSheet = () => {
    setSelected(null);
    setSelectedItem(null);
  };

  const eyebrow = selected
    ? `${state.categories.find((entry) => entry.id === selected.category)?.name ?? selected.category} · local`
    : selectedItem
      ? `ClawHub · ${selectedItem.kind}`
      : "";
  const title = selected?.name ?? selectedItem?.name ?? "";

  return (
    <div className="view">
      <div className="searchrow">
        <div className="searchbar">
          <span className="search-icon">{Icon.Search}</span>
          <span className="scope-chip">
            Skills {Icon.ChevronDown}
          </span>
          <input
            ref={searchRef}
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Search installed skills and the ClawHub registry"
            spellCheck={false}
            aria-label="Search skills"
          />
          {query ? (
            <button className="icon-btn small" onClick={() => setQuery("")} aria-label="Clear search">
              {Icon.Close}
            </button>
          ) : (
            <kbd>/</kbd>
          )}
        </div>
        <button className="primary" onClick={() => setCompose(true)}>
          {Icon.Plus} New skill
        </button>
      </div>

      <div className="filters">
        <select
          className="pill select"
          value={category}
          onChange={(event) => setCategory(event.target.value)}
          aria-label="Filter by category"
        >
          <option value="all">All categories</option>
          {state.categories.map((entry) => (
            <option key={entry.id} value={entry.id}>
              {entry.name}
            </option>
          ))}
        </select>

        <div className="segmented" role="group" aria-label="Source">
          {(["all", "installed", "market"] as Scope[]).map((value) => (
            <button key={value} data-active={scope === value} onClick={() => setScope(value)}>
              {value === "all" ? "All" : value === "market" ? "ClawHub" : "Installed"}
            </button>
          ))}
        </div>

        <span className="filter-note">
          {state.skills.filter((skill) => skill.enabled).length} enabled of {state.skills.length}
        </span>
        <span className="spacer" />
        <button
          className="ghost-btn"
          onClick={() => setNonce((value) => value + 1)}
          title="Refresh ClawHub"
        >
          {Icon.Refresh}
        </button>
      </div>

      {showInstalled && groups.length === 0 && (
        <div className="empty">
          <h3>No skills here yet</h3>
          <p>
            {query
              ? `Nothing installed matches “${query}”.`
              : "Create one, or install from the ClawHub shelf below."}
          </p>
          <button className="primary" onClick={() => setCompose(true)}>
            {Icon.Plus} New skill
          </button>
        </div>
      )}

      {showInstalled &&
        groups.map(({ entry, skills: items }) => (
          <section className="group" key={entry.id}>
            <header className="group-head">
              <h2>{entry.name}</h2>
              <p>{entry.blurb || "Your own folder."}</p>
              <span className="count">{items.length}</span>
            </header>
            <div className="grid">
              {items.map((skill) => (
                <article
                  className="card"
                  key={skill.path}
                  onClick={() => setSelected(skill)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" || event.key === " ") {
                      event.preventDefault();
                      setSelected(skill);
                    }
                  }}
                  role="button"
                  tabIndex={0}
                >
                  <div className="card-top">
                    <span className="eyebrow">
                      {entry.name} · {skill.enabled ? "on" : "off"}
                    </span>
                    <button
                      className="icon-btn small"
                      onClick={(event) => {
                        event.stopPropagation();
                        void toggle(skill);
                      }}
                      aria-label={skill.enabled ? `Disable ${skill.name}` : `Enable ${skill.name}`}
                      title={skill.enabled ? "Disable" : "Enable"}
                    >
                      {skill.enabled ? Icon.Check : Icon.Plus}
                    </button>
                  </div>
                  <h3>{skill.name}</h3>
                  <p>{skill.description || "No description yet."}</p>
                  <div className="card-foot">
                    <span className="chip">
                      {skill.files} file{skill.files === 1 ? "" : "s"}
                    </span>
                    <span className="chip ghost">{relativeTime(skill.updatedAt)}</span>
                    <span className="spacer" />
                    <span className="switch" data-on={skill.enabled} onClick={(event) => event.stopPropagation()}>
                      <input
                        type="checkbox"
                        checked={skill.enabled}
                        onChange={() => void toggle(skill)}
                        aria-label={`${skill.name} enabled`}
                      />
                      <span />
                    </span>
                  </div>
                </article>
              ))}
            </div>
          </section>
        ))}

      {showMarket && (
        <section className="group">
          <header className="group-head">
            <h2>ClawHub</h2>
            <p>
              {query.trim()
                ? `Skills and plugins matching “${query.trim()}”`
                : "Most downloaded skills, ready to install"}
            </p>
            <span className="count">{visibleMarket.length}</span>
          </header>

          {marketLoading && visibleMarket.length === 0 ? (
            <div className="grid">
              {[0, 1, 2, 3, 4, 5].map((index) => (
                <div className="skel skel-card" key={index} />
              ))}
            </div>
          ) : visibleMarket.length === 0 ? (
            <div className="empty">
              <h3>No results</h3>
              <p>Try a different word, or search by a skill name.</p>
            </div>
          ) : (
            <div className="grid">
              {visibleMarket.map((item) => (
                <article
                  className="card market"
                  key={item.id}
                  onClick={() => setSelectedItem(item)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" || event.key === " ") {
                      event.preventDefault();
                      setSelectedItem(item);
                    }
                  }}
                  role="button"
                  tabIndex={0}
                >
                  <div className="card-top">
                    <span className="eyebrow">
                      ClawHub · {item.kind === "bundle" ? "bundle plugin" : item.kind}
                    </span>
                    <button
                      className="icon-btn small"
                      onClick={(event) => {
                        event.stopPropagation();
                        if (item.installable) void install(item);
                        else void copy(item.command ?? "");
                      }}
                      aria-label={item.installable ? `Install ${item.name}` : `Copy the command for ${item.name}`}
                      title={item.installable ? "Install" : item.command ?? "Copy"}
                    >
                      {busy === item.id ? (
                        Icon.Spinner
                      ) : item.installable ? (
                        installedSlugs.has(item.slug) ? Icon.Check : Icon.Plus
                      ) : (
                        Icon.Copy
                      )}
                    </button>
                  </div>
                  <h3>{item.name}</h3>
                  <p>{item.summary || "No summary provided."}</p>
                  <div className="card-foot">
                    <span className="chip">{item.owner}</span>
                    {item.version && <span className="chip ghost">v{item.version}</span>}
                    <span className="spacer" />
                    {item.installable && installedSlugs.has(item.slug) && (
                      <span className="chip accent">Installed</span>
                    )}
                  </div>
                </article>
              ))}
            </div>
          )}
        </section>
      )}

      <Sheet open={sheetOpen} onClose={closeSheet} eyebrow={eyebrow} title={title}>
        {selected && (
          <LocalDetail
            skill={selected}
            state={state}
            onState={onState}
            onClose={closeSheet}
            notify={notify}
            onError={onError}
          />
        )}
        {selectedItem && (
          <MarketDetail
            item={selectedItem}
            state={state}
            busy={busy === selectedItem.id}
            onInstall={install}
            onError={onError}
          />
        )}
      </Sheet>

      {compose && (
        <ComposeDialog
          state={state}
          onClose={() => setCompose(false)}
          onState={onState}
          notify={notify}
          onError={onError}
        />
      )}
    </div>
  );
}

function LocalDetail({
  skill,
  state,
  onState,
  onClose,
  notify,
  onError
}: {
  skill: Skill;
  state: AppState;
  onState: (state: AppState) => void;
  onClose: () => void;
  notify: (message: string) => void;
  onError: (message: string) => void;
}) {
  const [confirming, setConfirming] = useState(false);

  const move = async (target: string) => {
    try {
      onState(await api.moveSkill(skill.category, skill.slug, target));
      notify(`Moved to ${target}`);
    } catch (reason) {
      onError(String(reason));
    }
  };

  const remove = async () => {
    try {
      onState(await api.deleteSkill(skill.category, skill.slug));
      notify(`${skill.name} removed`);
      onClose();
    } catch (reason) {
      onError(String(reason));
    }
  };

  return (
    <>
      <p className="sheet-lead">{skill.description || "No description yet."}</p>

      <section className="sheet-section">
        <span className="sheet-label">Enabled for the CLI</span>
        <div className="sheet-line">
          <span className="sheet-value">
            {skill.enabled ? "Linked into ~/.agents/skills" : "Off, the link is removed"}
          </span>
          <span className="switch" data-on={skill.enabled}>
            <input
              type="checkbox"
              checked={skill.enabled}
              onChange={async () => {
                try {
                  onState(await api.setSkillEnabled(skill.category, skill.slug, !skill.enabled));
                } catch (reason) {
                  onError(String(reason));
                }
              }}
              aria-label="Enabled"
            />
            <span />
          </span>
        </div>
      </section>

      <section className="sheet-section">
        <span className="sheet-label">Category</span>
        <select
          className="input"
          value={skill.category}
          onChange={(event) => void move(event.target.value)}
        >
          {state.categories.map((entry) => (
            <option key={entry.id} value={entry.id}>
              {entry.name}
            </option>
          ))}
        </select>
      </section>

      <section className="sheet-section">
        <span className="sheet-label">Files</span>
        <div className="meta">
          <MetaRow label="Folder" value={skill.path} mono />
          <MetaRow label="Contents" value={`${skill.files} file${skill.files === 1 ? "" : "s"}`} />
          <MetaRow label="Updated" value={relativeTime(skill.updatedAt)} />
        </div>
      </section>

      <div className="sheet-actions">
        <button
          className="ghost-btn"
          onClick={() => api.revealStore().catch((reason) => onError(String(reason)))}
        >
          {Icon.Folder} Show in Finder
        </button>
        <span className="spacer" />
        <button className="ghost-btn danger" onClick={() => (confirming ? void remove() : setConfirming(true))}>
          {Icon.Trash} {confirming ? "Confirm remove" : "Remove"}
        </button>
      </div>
    </>
  );
}

function MarketDetail({
  item,
  state,
  busy,
  onInstall,
  onError
}: {
  item: CatalogItem;
  state: AppState;
  busy: boolean;
  onInstall: (item: CatalogItem, category?: string) => void;
  onError: (message: string) => void;
}) {
  const [category, setCategory] = useState("other");
  const [preview, setPreview] = useState<string | null>(null);
  const [previewing, setPreviewing] = useState(true);

  useEffect(() => {
    let alive = true;
    if (!item.installable) {
      setPreviewing(false);
      return;
    }
    setPreviewing(true);
    api
      .previewMarket(item.slug)
      .then((text) => {
        if (alive) setPreview(text);
      })
      .catch((reason) => {
        if (alive) onError(String(reason));
      })
      .finally(() => {
        if (alive) setPreviewing(false);
      });
    return () => {
      alive = false;
    };
  }, [item.id]);

  return (
    <>
      <p className="sheet-lead">{item.summary || "No summary provided."}</p>

      <section className="sheet-section">
        <span className="sheet-label">Details</span>
        <div className="meta">
          <MetaRow label="Owner" value={item.owner} />
          <MetaRow label="Version" value={item.version || "unversioned"} />
          <MetaRow label="Downloads" value={item.downloads.toLocaleString()} />
          <MetaRow label="Kind" value={item.kind} />
        </div>
      </section>

      {item.installable ? (
        <>
          <section className="sheet-section">
            <span className="sheet-label">Install into</span>
            <select className="input" value={category} onChange={(event) => setCategory(event.target.value)}>
              {state.categories.map((entry) => (
                <option key={entry.id} value={entry.id}>
                  {entry.name}
                </option>
              ))}
            </select>
          </section>

          <section className="sheet-section">
            <span className="sheet-label">SKILL.md</span>
            <div className="preview" aria-busy={previewing}>
              {previewing ? <div className="skel skel-preview" /> : preview || "No preview available."}
            </div>
            <p className="sheet-note">
              Skills are instructions, not sandboxes. Read the file before you install.
            </p>
          </section>

          <div className="sheet-actions">
            <button className="primary" disabled={busy} onClick={() => onInstall(item, category)}>
              {busy ? Icon.Spinner : Icon.Plus} Install skill
            </button>
          </div>
        </>
      ) : (
        <div className="sheet-actions">
          <button
            className="primary"
            onClick={() => navigator.clipboard.writeText(item.command ?? "").catch(() => undefined)}
          >
            {Icon.Copy} Copy install command
          </button>
          <p className="sheet-note">
            Plugins ship their own runtime. Kilgharrah copies the package command instead of editing your
            MCP config.
          </p>
        </div>
      )}
    </>
  );
}

function ComposeDialog({
  state,
  onClose,
  onState,
  notify,
  onError
}: {
  state: AppState;
  onClose: () => void;
  onState: (state: AppState) => void;
  notify: (message: string) => void;
  onError: (message: string) => void;
}) {
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [category, setCategory] = useState("design");
  const [body, setBody] = useState("");
  const [saving, setSaving] = useState(false);

  const submit = async () => {
    if (!name.trim() || saving) return;
    setSaving(true);
    try {
      onState(await api.addSkill({ name, description, category, body }));
      notify(`${name.trim()} created and enabled`);
      onClose();
    } catch (reason) {
      onError(String(reason));
    } finally {
      setSaving(false);
    }
  };

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
      if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) void submit();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  return (
    <>
      <div className="scrim" data-open onClick={onClose} />
      <div className="dialog" role="dialog" aria-label="New skill">
        <header className="sheet-head">
          <div className="sheet-heading">
            <span className="eyebrow">Local</span>
            <h2>New skill</h2>
          </div>
          <button className="icon-btn" onClick={onClose} aria-label="Close">
            {Icon.Close}
          </button>
        </header>

        <div className="dialog-body">
          <label className="field">
            <span>Name</span>
            <input
              className="input"
              value={name}
              onChange={(event) => setName(event.target.value)}
              placeholder="Release notes"
              autoFocus
            />
          </label>

          <label className="field">
            <span>Description</span>
            <input
              className="input"
              value={description}
              onChange={(event) => setDescription(event.target.value)}
              placeholder="Use when the user asks for a changelog"
            />
          </label>

          <label className="field">
            <span>Category</span>
            <select className="input" value={category} onChange={(event) => setCategory(event.target.value)}>
              {state.categories.map((entry) => (
                <option key={entry.id} value={entry.id}>
                  {entry.name}
                </option>
              ))}
            </select>
          </label>

          <label className="field">
            <span>Instructions</span>
            <textarea
              className="input textarea"
              value={body}
              onChange={(event) => setBody(event.target.value)}
              placeholder={"# Release notes\n\n1. Read the commits since the last tag.\n2. Group them by user impact."}
              rows={6}
            />
          </label>
        </div>

        <div className="dialog-actions">
          <span className="sheet-note">Saved to the store and linked for the CLI.</span>
          <span className="spacer" />
          <button className="ghost-btn" onClick={onClose}>
            Cancel
          </button>
          <button className="primary" disabled={!name.trim() || saving} onClick={() => void submit()}>
            {saving ? Icon.Spinner : Icon.Plus} Create
          </button>
        </div>
      </div>
    </>
  );
}

function MetaRow({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className="meta-row">
      <span>{label}</span>
      <span className={mono ? "mono" : undefined} title={value}>
        {value}
      </span>
    </div>
  );
}

function relativeTime(seconds: number) {
  if (!seconds) return "unknown";
  const delta = Math.max(0, Math.floor(Date.now() / 1000) - seconds);
  if (delta < 60) return "just now";
  if (delta < 3600) return `${Math.floor(delta / 60)}m ago`;
  if (delta < 86400) return `${Math.floor(delta / 3600)}h ago`;
  if (delta < 2592000) return `${Math.floor(delta / 86400)}d ago`;
  return new Date(seconds * 1000).toLocaleDateString();
}
