import { useEffect, useMemo, useRef, useState } from "react";
import { api } from "../api";
import type { AppState, CatalogItem, Skill, SkillFile } from "../types";
import { Icon } from "./Icons";
import { Markdown } from "./Markdown";
import { Sheet } from "./Sheet";

type Scope = "all" | "store" | "hermes" | "market";

const MARKET_CATEGORIES: { id: string; label: string }[] = [
  { id: "all", label: "All" },
  { id: "design", label: "Design" },
  { id: "development", label: "Development" },
  { id: "writing", label: "Writing" },
  { id: "research", label: "Research" },
  { id: "data", label: "Data" },
  { id: "productivity", label: "Productivity" },
  { id: "media", label: "Media" },
  { id: "marketing", label: "Marketing" },
  { id: "automation", label: "Automation" },
  { id: "security", label: "Security" },
  { id: "ai", label: "AI and agents" },
  { id: "other", label: "Other" }
];

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
  const [marketCategory, setMarketCategory] = useState("all");
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
    if (scope === "store" || scope === "hermes") return;
    const trimmed = query.trim();
    if (trimmed.length === 1) return;
    let alive = true;
    const timer = setTimeout(
      async () => {
        setMarketLoading(true);
        try {
          const items = await api.searchMarket(trimmed, trimmed ? 60 : 200);
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

  const storeSkills = state.skills.filter((skill) => skill.source === "store");
  const hermesCount = state.skills.length - storeSkills.length;
  const categoryOptions = state.categories.filter((entry) =>
    scope === "store" ? entry.source === "store" : scope === "hermes" ? entry.source === "hermes" : true
  );

  useEffect(() => {
    setCategory("all");
  }, [scope]);

  const skills = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return state.skills.filter((skill) => {
      if (scope === "store" && skill.source !== "store") return false;
      if (scope === "hermes" && skill.source !== "hermes") return false;
      if (category !== "all" && skill.category !== category) return false;
      if (!needle) return true;
      return `${skill.name} ${skill.slug} ${skill.description}`.toLowerCase().includes(needle);
    });
  }, [state.skills, query, category, scope]);

  const groups = state.categories
    .map((entry) => ({ entry, skills: skills.filter((skill) => skill.category === entry.id) }))
    .filter((group) => group.skills.length > 0);

  const installedSlugs = new Set(storeSkills.map((skill) => skill.slug));

  const marketCounts = useMemo(() => {
    const counts: Record<string, number> = {};
    for (const item of market) {
      counts[item.category] = (counts[item.category] ?? 0) + 1;
    }
    return counts;
  }, [market]);

  const visibleMarket =
    scope === "store" || scope === "hermes"
      ? []
      : market.filter((item) => marketCategory === "all" || item.category === marketCategory);
  const showInstalled = scope !== "market";
  const showMarket = scope === "all" || scope === "market";
  const marketLabel =
    MARKET_CATEGORIES.find((entry) => entry.id === marketCategory)?.label ?? marketCategory;

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
      const next = await api.installMarket(item.slug, item.owner, target ?? null);
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
    ? `${state.categories.find((entry) => entry.id === selected.category)?.name ?? selected.category} · ${
        selected.source === "hermes" ? "Hermes" : "local"
      }`
    : selectedItem
      ? `ClawHub · ${selectedItem.kind}`
      : "";
  const title = selected?.name ?? selectedItem?.name ?? "";

  return (
    <div className="view">
      <div className="searchrow">
        <div className="searchbar">
          <span className="search-icon">{Icon.Search}</span>
          <span className="scope-chip">Skills {Icon.ChevronDown}</span>
          <input
            ref={searchRef}
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Search the store, Hermes and the ClawHub registry"
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
          {categoryOptions.map((entry) => (
            <option key={entry.id} value={entry.id}>
              {entry.name} ({entry.count})
            </option>
          ))}
        </select>

        <div className="segmented" role="group" aria-label="Source">
          {(["all", "store", "hermes", "market"] as Scope[]).map((value) => (
            <button key={value} data-active={scope === value} onClick={() => setScope(value)}>
              {value === "all"
                ? "All"
                : value === "market"
                  ? "ClawHub"
                  : value === "store"
                    ? "Store"
                    : "Hermes"}
            </button>
          ))}
        </div>

        <span className="filter-note">
          {storeSkills.filter((skill) => skill.enabled).length} enabled of {storeSkills.length} in the
          store · {hermesCount} in Hermes
        </span>
        <span className="spacer" />
        {showMarket && (
          <button
            className="ghost-btn"
            onClick={() => setNonce((value) => value + 1)}
            title="Refresh ClawHub"
          >
            {Icon.Refresh}
          </button>
        )}
      </div>

      {showInstalled && groups.length === 0 && (
        <div className="empty">
          <h3>Nothing matches</h3>
          <p>
            {query
              ? `No installed skill matches "${query.trim()}".`
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
                  key={skill.id}
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
                      {entry.name} ·{" "}
                      {skill.source === "hermes" ? "Hermes" : skill.enabled ? "on" : "off"}
                    </span>
                    {skill.source === "store" && (
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
                    )}
                  </div>
                  <h3>{skill.name}</h3>
                  <p>{skill.description || "No description yet."}</p>
                  <div className="card-foot">
                    <span className="chip">
                      {skill.files} file{skill.files === 1 ? "" : "s"}
                    </span>
                    <span className="chip ghost">{relativeTime(skill.updatedAt)}</span>
                    <span className="spacer" />
                    {skill.source === "store" ? (
                      <span
                        className="switch"
                        data-on={skill.enabled}
                        onClick={(event) => event.stopPropagation()}
                      >
                        <input
                          type="checkbox"
                          checked={skill.enabled}
                          onChange={() => void toggle(skill)}
                          aria-label={`${skill.name} enabled`}
                        />
                        <span />
                      </span>
                    ) : (
                      <span className="chip">Read-only</span>
                    )}
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
                ? `Skills and plugins matching "${query.trim()}"`
                : marketCategory === "all"
                  ? "Most downloaded skills, ready to install"
                  : `Top skills filed under ${marketLabel}`}
            </p>
            <span className="count">{visibleMarket.length}</span>
          </header>

          <div className="chips-row" role="tablist" aria-label="ClawHub category">
            {MARKET_CATEGORIES.filter(
              (entry) => entry.id === "all" || (marketCounts[entry.id] ?? 0) > 0
            ).map((entry) => (
              <button
                key={entry.id}
                role="tab"
                aria-selected={marketCategory === entry.id}
                className="pill"
                data-active={marketCategory === entry.id}
                onClick={() => setMarketCategory(entry.id)}
              >
                {entry.label}
                <span className="pill-count">
                  {entry.id === "all" ? market.length : marketCounts[entry.id] ?? 0}
                </span>
              </button>
            ))}
          </div>

          {marketLoading && market.length === 0 ? (
            <div className="grid">
              {[0, 1, 2, 3, 4, 5].map((index) => (
                <div className="skel skel-card" key={index} />
              ))}
            </div>
          ) : visibleMarket.length === 0 ? (
            <div className="empty">
              <h3>{market.length === 0 ? "No results" : `Nothing under ${marketLabel} yet`}</h3>
              <p>
                {market.length === 0
                  ? "Try a different word, or search by a skill name."
                  : "Pick another category, or search ClawHub by name."}
              </p>
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
                      ClawHub ·{" "}
                      {MARKET_CATEGORIES.find((entry) => entry.id === item.category)?.label ??
                        item.kind}
                    </span>
                    <button
                      className="icon-btn small"
                      onClick={(event) => {
                        event.stopPropagation();
                        if (item.installable) void install(item);
                        else void copy(item.command ?? "");
                      }}
                      aria-label={
                        item.installable ? `Install ${item.name}` : `Copy the command for ${item.name}`
                      }
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
  const [files, setFiles] = useState<SkillFile[]>([]);
  const [active, setActive] = useState("SKILL.md");
  const [content, setContent] = useState("");
  const [draft, setDraft] = useState("");
  const [mode, setMode] = useState<"view" | "edit">("view");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const [target, setTarget] = useState("other");
  const [importing, setImporting] = useState(false);
  const readOnly = skill.source === "hermes";
  const storeCategories = state.categories.filter((entry) => entry.source === "store");
  const categoryName =
    state.categories.find((entry) => entry.id === skill.category)?.name ?? skill.category;

  useEffect(() => {
    let alive = true;
    setLoading(true);
    api
      .listSkillFiles(skill.source, skill.category, skill.slug)
      .then((list) => {
        if (!alive) return;
        setFiles(list);
        const first = list.find((file) => file.path === "SKILL.md")?.path ?? list[0]?.path;
        if (first) setActive(first);
      })
      .catch((reason) => {
        if (alive) onError(String(reason));
      })
      .finally(() => {
        if (alive) setLoading(false);
      });
    return () => {
      alive = false;
    };
  }, [skill.id]);

  useEffect(() => {
    let alive = true;
    setMode("view");
    setLoading(true);
    api
      .readSkillFile(skill.source, skill.category, skill.slug, active)
      .then((text) => {
        if (!alive) return;
        setContent(text);
        setDraft(text);
      })
      .catch((reason) => {
        if (!alive) return;
        setContent("");
        setDraft("");
        onError(String(reason));
      })
      .finally(() => {
        if (alive) setLoading(false);
      });
    return () => {
      alive = false;
    };
  }, [active, skill.id]);

  const move = async (next: string) => {
    try {
      onState(await api.moveSkill(skill.category, skill.slug, next));
      notify(`Moved to ${next}`);
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

  const save = async () => {
    if (saving || draft === content) return;
    setSaving(true);
    try {
      onState(await api.writeSkillFile(skill.category, skill.slug, active, draft));
      setContent(draft);
      setMode("view");
      notify(`${active} saved`);
      setFiles(await api.listSkillFiles(skill.source, skill.category, skill.slug));
    } catch (reason) {
      onError(String(reason));
    } finally {
      setSaving(false);
    }
  };

  const importCopy = async () => {
    setImporting(true);
    try {
      onState(await api.importHermes(skill.category, skill.slug, target));
      notify(`${skill.name} imported into ${target}`);
      onClose();
    } catch (reason) {
      onError(String(reason));
    } finally {
      setImporting(false);
    }
  };

  return (
    <>
      <p className="sheet-lead">{skill.description || "No description yet."}</p>

      <div className="facts">
        <Fact label="Source" value={readOnly ? "Hermes library" : "Kilgharrah store"} />
        <Fact
          label="Status"
          value={readOnly ? "Read-only" : skill.enabled ? "Linked for the CLI" : "Not linked"}
        />
        <Fact label="Category" value={categoryName} />
        <Fact label="Slug" value={skill.slug} mono />
        <Fact label="Files" value={`${skill.files} file${skill.files === 1 ? "" : "s"}`} />
        <Fact label="Updated" value={relativeTime(skill.updatedAt)} />
        <Fact label="Folder" value={skill.path} mono wide />
      </div>

      <section className="sheet-section">
        <div className="section-head">
          <span className="sheet-label">Contents</span>
          <div className="segmented" role="group" aria-label="File mode">
            <button data-active={mode === "view"} onClick={() => setMode("view")}>
              {Icon.Eye} View
            </button>
            <button
              data-active={mode === "edit"}
              disabled={readOnly}
              title={readOnly ? "Import the skill to edit it" : "Edit this file"}
              onClick={() => setMode("edit")}
            >
              {Icon.Edit} Edit
            </button>
          </div>
        </div>

        <div className="file-list">
          {files.map((file) => (
            <button
              key={file.path}
              className="file-chip"
              data-active={file.path === active}
              onClick={() => setActive(file.path)}
              title={file.path}
            >
              {Icon.File}
              <span className="file-name">{file.path}</span>
              <span className="file-size">{formatBytes(file.bytes)}</span>
            </button>
          ))}
        </div>

        {loading ? (
          <div className="skel skel-preview" />
        ) : mode === "edit" ? (
          <>
            <textarea
              className="input textarea mono editor"
              value={draft}
              onChange={(event) => setDraft(event.target.value)}
              spellCheck={false}
              rows={18}
              aria-label={`Edit ${active}`}
            />
            <div className="editor-actions">
              <span className="sheet-note">
                {draft === content ? "No changes yet." : "Unsaved changes."}
              </span>
              <span className="spacer" />
              <button
                className="ghost-btn"
                disabled={draft === content || saving}
                onClick={() => setDraft(content)}
              >
                Revert
              </button>
              <button
                className="primary compact"
                disabled={draft === content || saving}
                onClick={() => void save()}
              >
                {saving ? Icon.Spinner : Icon.Check} Save
              </button>
            </div>
          </>
        ) : content ? (
          <div className="reader-wrap">
            <Markdown text={content} />
          </div>
        ) : (
          <p className="sheet-note">Pick a file to read it.</p>
        )}
      </section>

      {readOnly ? (
        <section className="sheet-section">
          <span className="sheet-label">Import into the store</span>
          <p className="sheet-note">
            Hermes keeps and regenerates its own copy. Import the skill to edit it, link it for the
            CLI, and keep your changes.
          </p>
          <div className="import-row">
            <select className="input" value={target} onChange={(event) => setTarget(event.target.value)}>
              {storeCategories.map((entry) => (
                <option key={entry.id} value={entry.id}>
                  {entry.name}
                </option>
              ))}
            </select>
            <button
              className="primary compact"
              disabled={importing}
              onClick={() => void importCopy()}
            >
              {importing ? Icon.Spinner : Icon.Import} Import
            </button>
          </div>
        </section>
      ) : (
        <section className="sheet-section">
          <span className="sheet-label">Move to category</span>
          <select className="input" value={skill.category} onChange={(event) => void move(event.target.value)}>
            {storeCategories.map((entry) => (
              <option key={entry.id} value={entry.id}>
                {entry.name}
              </option>
            ))}
          </select>
        </section>
      )}

      <div className="sheet-actions">
        <button
          className="ghost-btn"
          onClick={() => api.revealStore().catch((reason) => onError(String(reason)))}
        >
          {Icon.Folder} Show store
        </button>
        <span className="spacer" />
        {!readOnly && (
          <button
            className="ghost-btn danger"
            onClick={() => (confirming ? void remove() : setConfirming(true))}
          >
            {Icon.Trash} {confirming ? "Confirm remove" : "Remove"}
          </button>
        )}
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
  const storeCategories = state.categories.filter((entry) => entry.source === "store");
  const [category, setCategory] = useState(() =>
    storeCategories.some((entry) => entry.id === item.category) ? item.category : "other"
  );
  const [preview, setPreview] = useState<string | null>(null);
  const [previewing, setPreviewing] = useState(true);
  const label =
    MARKET_CATEGORIES.find((entry) => entry.id === item.category)?.label ?? item.category;

  useEffect(() => {
    let alive = true;
    if (!item.installable) {
      setPreviewing(false);
      return;
    }
    setPreviewing(true);
    api
      .previewMarket(item.slug, item.owner)
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

      <div className="facts">
        <Fact label="Owner" value={item.owner} />
        <Fact label="Version" value={item.version || "unversioned"} />
        <Fact label="Downloads" value={item.downloads.toLocaleString()} />
        <Fact label="Category" value={label} />
        <Fact label="Kind" value={item.kind === "bundle" ? "bundle plugin" : item.kind} />
        <Fact label="Installable" value={item.installable ? "as a skill" : "as a package"} />
      </div>

      {item.topics.length > 0 && (
        <div className="chips-row">
          {item.topics.slice(0, 14).map((topic) => (
            <span className="chip" key={topic}>
              {topic}
            </span>
          ))}
        </div>
      )}

      {item.installable ? (
        <>
          <section className="sheet-section">
            <span className="sheet-label">Install into</span>
            <select className="input" value={category} onChange={(event) => setCategory(event.target.value)}>
              {storeCategories.map((entry) => (
                <option key={entry.id} value={entry.id}>
                  {entry.name}
                </option>
              ))}
            </select>
          </section>

          <section className="sheet-section">
            <span className="sheet-label">SKILL.md</span>
            {previewing ? (
              <div className="skel skel-preview" />
            ) : preview ? (
              <div className="reader-wrap tall">
                <Markdown text={preview} />
              </div>
            ) : (
              <p className="sheet-note">No preview available.</p>
            )}
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
            Plugins ship their own runtime. Kilgharrah copies the package command instead of editing
            your MCP config.
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
  const storeCategories = state.categories.filter((entry) => entry.source === "store");
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [category, setCategory] = useState(storeCategories[0]?.id ?? "design");
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
              {storeCategories.map((entry) => (
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

function Fact({
  label,
  value,
  mono,
  wide
}: {
  label: string;
  value: string;
  mono?: boolean;
  wide?: boolean;
}) {
  return (
    <div className={wide ? "fact wide" : "fact"}>
      <span className="fact-label">{label}</span>
      <span className={mono ? "fact-value mono" : "fact-value"} title={value}>
        {value}
      </span>
    </div>
  );
}

function formatBytes(bytes: number) {
  if (!bytes) return "0 B";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(bytes < 10240 ? 1 : 0)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
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
