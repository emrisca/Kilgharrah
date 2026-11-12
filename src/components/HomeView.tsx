import { useEffect, useState } from "react";
import type { AppState } from "../types";
import type { Tab } from "../App";
import { Icon } from "./Icons";
import dragon from "../assets/dragon.txt?raw";

type Props = {
  state: AppState;
  onOpen: (tab: Tab) => void;
};

/// The banner art is one flying dragon frame per block of text, cut from the
/// GIF and kept as plain text so it stays sharp at any size and adds no image
/// weight to the bundle.
const FRAMES = dragon.trim().split(/\n~{8}\n/);
const FRAME_MS = 60;

function prefersCalm() {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

export function HomeView({ state, onOpen }: Props) {
  const [frame, setFrame] = useState(0);

  useEffect(() => {
    if (prefersCalm()) return;
    const timer = window.setInterval(() => {
      setFrame((current) => (current + 1) % FRAMES.length);
    }, FRAME_MS);
    return () => window.clearInterval(timer);
  }, []);

  const art = FRAMES[frame];
  const enabled = state.skills.filter((skill) => skill.enabled).length;
  const installed = state.skills.length;

  return (
    <div className="home">
      <div className="home-hero">
        <div className="home-copy">
          <span className="home-eyebrow">Skills and MCP for your CLI</span>

          <h1 className="home-word">Kilgharrah</h1>

          <div className="home-rule" aria-hidden="true">
            <span />
            <i />
            <span />
          </div>

          <p className="home-lead">
            Kilgharrah keeps the skills and MCP servers for your CLI. Add skills from the
            marketplace or write your own. Link them to one agent or several. Switch a skill off
            and its files stay on disk.
          </p>

          <div className="home-actions">
            <button className="primary" onClick={() => onOpen("skills")}>
              {Icon.Star} Browse skills
            </button>
            <button className="ghost-btn" onClick={() => onOpen("mcp")}>
              {Icon.Server} MCP servers
            </button>
          </div>

          <dl className="home-stats">
            <div>
              <dt>Skills bound</dt>
              <dd>{enabled}</dd>
            </div>
            <div>
              <dt>MCP servers</dt>
              <dd>{state.servers.length}</dd>
            </div>
            <div>
              <dt>Installed</dt>
              <dd>{installed}</dd>
            </div>
          </dl>
        </div>

        <div className="home-art" aria-hidden="true">
          <div className="home-glow" />
          <pre className="home-art-layer home-art-ink">{art}</pre>
        </div>
      </div>
    </div>
  );
}
