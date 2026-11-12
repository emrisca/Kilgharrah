import { useEffect, useRef, useState, type ReactNode } from "react";
import { springTo } from "../spring";
import { Icon } from "./Icons";

type Props = {
  open: boolean;
  onClose: () => void;
  eyebrow: string;
  title: string;
  wide?: boolean;
  children: ReactNode;
};

/**
 * Right-side inspector. It stays mounted through the exit spring so closing
 * can be grabbed and reversed, and the scrim is the only part that fades.
 */
export function Sheet({ open, onClose, eyebrow, title, wide, children }: Props) {
  const [mounted, setMounted] = useState(open);
  const panel = useRef<HTMLDivElement>(null);
  const last = useRef({ eyebrow, title, children });
  if (open) last.current = { eyebrow, title, children };
  const view = open ? { eyebrow, title, children } : last.current;

  useEffect(() => {
    if (open) setMounted(true);
  }, [open]);

  useEffect(() => {
    if (!mounted) return;
    const el = panel.current;
    if (!el) return;
    if (open) {
      springTo(el, 0, { stiffness: 300, damping: 32, from: el.offsetWidth });
    } else {
      springTo(el, el.offsetWidth, {
        stiffness: 300,
        damping: 36,
        onDone: () => setMounted(false)
      });
    }
  }, [open, mounted]);

  useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!mounted) return null;

  return (
    <>
      <div className="scrim" data-open={open} onClick={onClose} />
      <aside
        ref={panel}
        className="sheet"
        data-open={open}
        data-wide={wide || undefined}
        aria-label={view.title}
        role="dialog"
      >
        <header className="sheet-head">
          <div className="sheet-heading">
            <span className="eyebrow">{view.eyebrow}</span>
            <h2>{view.title}</h2>
          </div>
          <button className="icon-btn" onClick={onClose} aria-label="Close">
            {Icon.Close}
          </button>
        </header>
        <div className="sheet-body">{view.children}</div>
      </aside>
    </>
  );
}
