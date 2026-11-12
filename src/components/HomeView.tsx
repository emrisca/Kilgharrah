import { useEffect, useMemo, useState } from "react";
import type { AppState } from "../types";
import type { Tab } from "../App";
import { Icon } from "./Icons";
import dragon from "../assets/dragon.txt?raw";

type Props = {
  state: AppState;
  onOpen: (tab: Tab) => void;
};

const RAIN = ["|", ".", "'", ",", "`", ":"];
const COLUMNS = 22;

/// A small seeded generator keeps the rain layout stable between renders, so
/// the columns do not jump when a skill is toggled elsewhere in the app.
function makeColumns() {
  let seed = 113;
  const next = () => {
    seed = (seed * 1103515245 + 12345) % 2147483648;
    return seed / 2147483648;
  };
  return Array.from({ length: COLUMNS }, (_, index) => {
    const length = 7 + Math.floor(next() * 9);
    let text = "";
    for (let line = 0; line < length; line += 1) {
      text += `${RAIN[Math.floor(next() * RAIN.length)]}\n`;
    }
    return {
      key: index,
      left: ((index + 0.5) / COLUMNS) * 100,
      text,
      seconds: 7 + next() * 9,
      delay: -next() * 16,
      opacity: 0.06 + next() * 0.12,
      size: 10 + Math.floor(next() * 4)
    };
  });
}

/// The banner art is one flying dragon frame per block of text, cut from the
/// GIF and kept as plain text so it stays sharp at any size and adds no image
/// weight to the bundle.
const FRAMES = dragon.trim().split(/\n~{8}\n/);
const FRAME_MS = 60;

function prefersCalm() {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

export function HomeView({ state, onOpen }: Props) {
  const rain = useMemo(makeColumns, []);
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
  const stored = state.skills.filter((skill) => skill.source === "store").length;

  return (
    <div className="home">
      <div className="home-rain" aria-hidden="true">
        {rain.map((column) => (
          <span
            className="rain-col"
            key={column.key}
            style={{
              left: `${column.left}%`,
              fontSize: `${column.size}px`,
              opacity: column.opacity,
              animationDuration: `${column.seconds}s`,
              animationDelay: `${column.delay}s`
            }}
          >
            {column.text}
          </span>
        ))}
      </div>

      <div className="home-hero">
        <div className="home-copy">
          <span className="home-eyebrow">Skills and MCP for the Freebuff CLI</span>

          <h1 className="home-word">Kilgharrah</h1>

          <div className="home-rule" aria-hidden="true">
            <span />
            <i />
            <span />
          </div>

          <p className="home-lead">
            A local hoard for the skills your CLI loads. Summon them from the marketplace or write
            your own, group them by craft, and switch one off without losing a file.
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
              <dt>In the store</dt>
              <dd>{stored}</dd>
            </div>
          </dl>

          <p className="home-foot">Named for the dragon beneath Camelot.</p>
        </div>

        <div className="home-art" aria-hidden="true">
          <div className="home-glow" />
          <pre className="home-art-layer home-art-ghost">{art}</pre>
          <pre className="home-art-layer home-art-ink">{art}</pre>
          <div className="home-embers">
            {Array.from({ length: 7 }, (_, index) => (
              <span
                key={index}
                style={{
                  left: `${18 + index * 11}%`,
                  animationDelay: `${index * 1.4}s`,
                  animationDuration: `${8 + (index % 3) * 2}s`
                }}
              >
                {index % 2 === 0 ? "·" : "'"}
              </span>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
