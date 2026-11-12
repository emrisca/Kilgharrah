import { useEffect, useState } from "react";
import { api } from "../api";
import type { AppState } from "../types";
import type { Tab } from "../App";
import { Icon } from "./Icons";
import { BUNDLED_FRAMES, splitFrames } from "../dragon";

type Props = {
  state: AppState;
  onOpen: (tab: Tab) => void;
};

/// The banner art is one flying dragon frame per block of text, cut from the
/// GIF and kept as plain text so it stays sharp at any size and adds no image
/// weight to the bundle. A saved copy in the library wins over the bundle.
const FRAME_MS = 60;

function prefersCalm() {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

export function HomeView({ state, onOpen }: Props) {
  const [frames, setFrames] = useState(BUNDLED_FRAMES);
  const [frame, setFrame] = useState(0);

  useEffect(() => {
    let alive = true;
    const load = async () => {
      try {
        const saved = await api.readDragon();
        if (!alive) return;
        setFrames(saved ? splitFrames(saved) : BUNDLED_FRAMES);
        setFrame(0);
      } catch {
        // Keep the bundled art when the saved copy cannot be read.
      }
    };
    void load();
    const onSaved = () => void load();
    window.addEventListener("kilgharrah:dragon", onSaved);
    return () => {
      alive = false;
      window.removeEventListener("kilgharrah:dragon", onSaved);
    };
  }, []);

  useEffect(() => {
    if (prefersCalm()) return;
    const timer = window.setInterval(() => {
      setFrame((current) => (current + 1) % frames.length);
    }, FRAME_MS);
    return () => window.clearInterval(timer);
  }, [frames.length]);

  const art = frames[frame] ?? frames[0];
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
