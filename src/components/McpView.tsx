import { useEffect, useState } from "react";
import { api } from "../api";
import type { AppState, CatalogItem } from "../types";
import { Icon } from "./Icons";

type Props = {
  state: AppState;
  onState: (state: AppState) => void;
  notify: (message: string) => void;
  onError: (message: string) => void;
};

export function McpView({ state, onState, notify, onError }: Props) {
  const [query, setQuery] = useState("");
  const [adding, setAdding] = useState(false);
  const [market, setMarket] = useState<CatalogItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [confirming, setConfirming] = useState<string | null>(null);

  useEffect(() => {
    const trimmed = query.trim() || "mcp";
    let alive = true;
    const timer = setTimeout(async () => {
      setLoading(true);
      try {
        const items = await api.searchMarket(trimmed, 16);
        if (alive) setMarket(items.filter((item) => item.kind !== "skill"));
      } catch (reason) {
        onError(String(reason));
      } finally {
        if (alive) setLoading(false);
      }
    }, 320);
    return () => {
      alive = false;
      clearTimeout(timer);
    };
  }, [query]);

  const needle = query.trim().toLowerCase();
  const servers = state.servers.filter((server) => {
    if (!needle) return true;
    return `${server.name} ${server.command} ${server.args.join(" ")}`.toLowerCase().includes(needle);
  });

  const remove = async (name: string) => {
    try {
      onState(await api.deleteServer(name));
      notify(`${name} removed from mcp.json`);
      setConfirming(null);
    } catch (reason) {
      onError(String(reason));
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

  return (
    <div className="view">
      <div className="searchrow">
        <div className="searchbar">
          <span className="search-icon">{Icon.Search}</span>
          <span className="scope-chip">MCP {Icon.ChevronDown}</span>
          <input
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Filter your servers, or search ClawHub plugins"
            spellCheck={false}
            aria-label="Search MCP servers"
          />
          {query && (
            <button className="icon-btn small" onClick={() => setQuery("")} aria-label="Clear search">
              {Icon.Close}
            </button>
          )}
        </div>
        <button className="primary" onClick={() => setAdding(true)}>
          {Icon.Plus} Add server
        </button>
      </div>

      <div className="filters">
        <span className="pill static">
          {state.servers.length} configured
        </span>
        <span className="filter-note mono">{state.mcpPath}</span>
        <span className="spacer" />
        <button
          className="ghost-btn"
          onClick={() => api.revealStore().catch((reason) => onError(String(reason)))}
          title="Show the store in Finder"
        >
          {Icon.Folder}
        </button>
      </div>

      <section className="group">
        <header className="group-head">
          <h2>Your servers</h2>
          <p>Read from the same mcp.json the Freebuff CLI loads</p>
          <span className="count">{servers.length}</span>
        </header>

        {servers.length === 0 ? (
          <div className="empty">
            <h3>{state.servers.length === 0 ? "No MCP servers yet" : "No match"}</h3>
            <p>
              {state.servers.length === 0
                ? "Add a command and its arguments, and Kilgharrah writes the entry for you."
                : "Try a different name or command."}
            </p>
            {state.servers.length === 0 && (
              <button className="primary" onClick={() => setAdding(true)}>
                {Icon.Plus} Add server
              </button>
            )}
          </div>
        ) : (
          <div className="rows">
            {servers.map((server) => (
              <article className="row" key={server.name}>
                <div className="row-mark">{Icon.Server}</div>
                <div className="row-main">
                  <div className="row-title">
                    <h3>{server.name}</h3>
                    <span className="chip accent">Configured</span>
                  </div>
                  <p className="mono row-command">
                    {server.command} {server.args.join(" ")}
                  </p>
                </div>
                <div className="row-actions">
                  <button
                    className="icon-btn small"
                    title="Copy the command"
                    onClick={() => void copy(`${server.command} ${server.args.join(" ")}`.trim())}
                  >
                    {Icon.Copy}
                  </button>
                  <button
                    className="ghost-btn danger"
                    onClick={() => (confirming === server.name ? void remove(server.name) : setConfirming(server.name))}
                  >
                    {Icon.Trash}
                    <span>{confirming === server.name ? "Confirm" : "Remove"}</span>
                  </button>
                </div>
              </article>
            ))}
          </div>
        )}
      </section>

      <section className="group">
        <header className="group-head">
          <h2>ClawHub tooling</h2>
          <p>Plugin packages that ship MCP servers and runtimes</p>
          <span className="count">{market.length}</span>
        </header>

        {loading && market.length === 0 ? (
          <div className="grid">
            {[0, 1, 2].map((index) => (
              <div className="skel skel-card" key={index} />
            ))}
          </div>
        ) : market.length === 0 ? (
          <div className="empty">
            <h3>Nothing found</h3>
            <p>Search ClawHub by package name, for example postgres or github.</p>
          </div>
        ) : (
          <div className="grid">
            {market.map((item) => (
              <article className="card market" key={item.id}>
                <div className="card-top">
                  <span className="eyebrow">ClawHub · {item.kind}</span>
                  <button
                    className="icon-btn small"
                    title={item.command ?? "Copy"}
                    onClick={() => void copy(item.command ?? "")}
                    aria-label={`Copy the command for ${item.name}`}
                  >
                    {Icon.Copy}
                  </button>
                </div>
                <h3>{item.name}</h3>
                <p>{item.summary || "No summary provided."}</p>
                <div className="card-foot">
                  <span className="chip">{item.owner}</span>
                  {item.version && <span className="chip ghost">v{item.version}</span>}
                </div>
                <p className="mono card-command">{item.command}</p>
              </article>
            ))}
          </div>
        )}
      </section>

      {adding && (
        <AddServerDialog
          onClose={() => setAdding(false)}
          onState={onState}
          notify={notify}
          onError={onError}
        />
      )}
    </div>
  );
}

function AddServerDialog({
  onClose,
  onState,
  notify,
  onError
}: {
  onClose: () => void;
  onState: (state: AppState) => void;
  notify: (message: string) => void;
  onError: (message: string) => void;
}) {
  const [name, setName] = useState("");
  const [command, setCommand] = useState("");
  const [args, setArgs] = useState("");
  const [env, setEnv] = useState("");
  const [saving, setSaving] = useState(false);

  const submit = async () => {
    if (!name.trim() || !command.trim() || saving) return;
    setSaving(true);
    try {
      onState(
        await api.saveServer({
          name,
          command,
          args: args.split(/\s+/).filter(Boolean),
          env: env.split("\n").filter((line) => line.trim())
        })
      );
      notify(`${name.trim()} saved to mcp.json`);
      onClose();
    } catch (reason) {
      onError(String(reason));
    } finally {
      setSaving(false);
    }
  };

  return (
    <>
      <div className="scrim" data-open onClick={onClose} />
      <div className="dialog" role="dialog" aria-label="Add MCP server">
        <header className="sheet-head">
          <div className="sheet-heading">
            <span className="eyebrow">MCP</span>
            <h2>Add server</h2>
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
              placeholder="filesystem"
              autoFocus
            />
          </label>
          <label className="field">
            <span>Command</span>
            <input
              className="input mono"
              value={command}
              onChange={(event) => setCommand(event.target.value)}
              placeholder="npx"
            />
          </label>
          <label className="field">
            <span>Arguments</span>
            <input
              className="input mono"
              value={args}
              onChange={(event) => setArgs(event.target.value)}
              placeholder="-y @modelcontextprotocol/server-filesystem ~/Documents"
            />
          </label>
          <label className="field">
            <span>Environment, one KEY=value per line</span>
            <textarea
              className="input textarea mono"
              value={env}
              onChange={(event) => setEnv(event.target.value)}
              placeholder={"API_KEY=..."}
              rows={3}
            />
          </label>
        </div>

        <div className="dialog-actions">
          <span className="sheet-note">Writes one entry under mcpServers.</span>
          <span className="spacer" />
          <button className="ghost-btn" onClick={onClose}>
            Cancel
          </button>
          <button className="primary" disabled={!name.trim() || !command.trim() || saving} onClick={() => void submit()}>
            {saving ? Icon.Spinner : Icon.Plus} Save
          </button>
        </div>
      </div>
    </>
  );
}
