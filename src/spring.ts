type Options = {
  stiffness?: number;
  damping?: number;
  from?: number;
  onDone?: () => void;
};

type Entry = { x: number; v: number; raf: number };

const running = new WeakMap<HTMLElement, Entry>();
const reduceMotion = () =>
  typeof window !== "undefined" &&
  window.matchMedia("(prefers-reduced-motion: reduce)").matches;

/**
 * Moves an element on x with a real spring. A call during a running spring
 * takes over from the current position and velocity, so a reversal never
 * jumps and never has to wait for the previous motion to finish.
 */
export function springTo(el: HTMLElement, target: number, options: Options = {}) {
  const stiffness = options.stiffness ?? 260;
  const damping = options.damping ?? 30;
  const previous = running.get(el);
  if (previous) cancelAnimationFrame(previous.raf);

  const entry: Entry = previous ?? { x: target, v: 0, raf: 0 };
  if (options.from !== undefined) {
    entry.x = options.from;
    entry.v = previous?.v ?? 0;
  }

  if (reduceMotion()) {
    el.style.transform = `translate3d(${target}px, 0, 0)`;
    running.delete(el);
    options.onDone?.();
    return;
  }

  let last = performance.now();
  const step = (now: number) => {
    const dt = Math.min((now - last) / 1000, 1 / 30);
    last = now;
    const acceleration = -stiffness * (entry.x - target) - damping * entry.v;
    entry.v += acceleration * dt;
    entry.x += entry.v * dt;
    el.style.transform = `translate3d(${entry.x}px, 0, 0)`;

    if (Math.abs(entry.v) < 0.6 && Math.abs(entry.x - target) < 0.6) {
      el.style.transform = `translate3d(${target}px, 0, 0)`;
      running.delete(el);
      options.onDone?.();
      return;
    }
    entry.raf = requestAnimationFrame(step);
  };

  running.set(el, entry);
  entry.raf = requestAnimationFrame(step);
}
