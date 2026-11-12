import { useEffect, useState } from "react";
import { api } from "../api";
import { BUNDLED_FRAMES, FRAME_COLUMNS, FRAME_ROWS, splitFrames } from "../dragon";
import { Icon } from "./Icons";
import { Sheet } from "./Sheet";

type Props = {
  open: boolean;
  onClose: () => void;
  notify: (message: string) => void;
  onError: (message: string) => void;
};

/// The editor works on one frame at a time. It keeps the separator lines out
/// of the text so a stray character is easy to spot and remove.
export function DragonEditor({ open, onClose, notify, onError }: Props) {
  const [frames, setFrames] = useState<string[]>(BUNDLED_FRAMES);
  const [index, setIndex] = useState(0);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!open) return;
    let alive = true;
    setLoading(true);
    api
      .readDragon()
      .then((saved) => {
        if (!alive) return;
        setFrames(saved ? splitFrames(saved) : BUNDLED_FRAMES);
        setIndex(0);
      })
      .catch((reason) => onError(String(reason)))
      .finally(() => {
        if (alive) setLoading(false);
      });
    return () => {
      alive = false;
    };
  }, [open, onError]);

  const frame = frames[index] ?? "";
  const lines = frame.split("\n");
  const width = Math.max(0, ...lines.map((line) => line.length));
  const valid = lines.length === FRAME_ROWS && width <= FRAME_COLUMNS;

  /// Arrow keys step through frames. Inside the text box they move the caret,
  /// so hold Alt there to step instead.
  useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.metaKey || event.ctrlKey || event.shiftKey) return;
      if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
      const typing = event.target instanceof HTMLTextAreaElement;
      if (typing && !event.altKey) return;
      event.preventDefault();
      setIndex((current) => {
        const next = event.key === "ArrowRight" ? current + 1 : current - 1;
        return Math.min(Math.max(next, 0), frames.length - 1);
      });
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, frames.length]);

  const edit = (value: string) => {
    setFrames((current) => current.map((item, at) => (at === index ? value : item)));
  };

  const save = async () => {
    setSaving(true);
    try {
      await api.saveDragon(frames);
      window.dispatchEvent(new Event("kilgharrah:dragon"));
      notify("Dragon frames saved");
      onClose();
    } catch (reason) {
      onError(String(reason));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Sheet open={open} onClose={onClose} eyebrow="Home animation" title="Dragon frames" wide>
      <p className="sheet-lead">
        Each frame is 23 lines of text, up to 70 columns wide. Edits are saved to your library and
        replace the bundled art. Use the arrow keys to step through frames, or hold Alt inside the
        text box.
      </p>

      <div className="dragon-nav">
        <button
          className="ghost-btn"
          disabled={index === 0 || loading}
          onClick={() => setIndex(index - 1)}
        >
          Previous
        </button>
        <span className="sheet-note">
          Frame {index + 1} of {frames.length}
        </span>
        <button
          className="ghost-btn"
          disabled={index >= frames.length - 1 || loading}
          onClick={() => setIndex(index + 1)}
        >
          Next
        </button>
      </div>

      <textarea
        className="input mono dragon-editor"
        aria-label={`Frame ${index + 1}`}
        value={frame}
        onChange={(event) => edit(event.target.value)}
        spellCheck={false}
        wrap="off"
      />

      <span className="sheet-note">
        {valid
          ? `${FRAME_ROWS} lines, ${width} of ${FRAME_COLUMNS} columns.`
          : `This frame has ${lines.length} lines and is ${width} columns wide. Keep 23 lines and 70 columns or fewer.`}
      </span>

      <div className="filters">
        <button
          className="ghost-btn"
          disabled={loading || saving}
          onClick={() => {
            setFrames(BUNDLED_FRAMES);
            setIndex(0);
          }}
        >
          Load bundled frames
        </button>
        <span className="spacer" />
        <button className="primary" disabled={!valid || loading || saving} onClick={() => void save()}>
          {saving ? Icon.Spinner : Icon.Check} Save frames
        </button>
      </div>
    </Sheet>
  );
}
