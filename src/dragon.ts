import dragon from "./assets/dragon.txt?raw";

const FRAME_SEPARATOR = "\n~~~~~~~~\n";

export const FRAME_ROWS = 23;
export const FRAME_COLUMNS = 70;

/// The banner art ships as plain text with a line of tildes between frames.
export function splitFrames(text: string) {
  return text.replace(/\r\n/g, "\n").replace(/\n$/, "").split(FRAME_SEPARATOR);
}

/// A frame fits the art box when it has every row and no line is too wide.
/// Blank rows count, because several frames rest the dragon near the top.
export function frameFits(frame: string) {
  const rows = frame.split("\n");
  return rows.length === FRAME_ROWS && rows.every((row) => row.length <= FRAME_COLUMNS);
}

export const BUNDLED_FRAMES = splitFrames(dragon);
