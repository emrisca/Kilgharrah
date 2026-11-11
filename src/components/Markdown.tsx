import { Fragment, type ReactNode } from "react";

/**
 * A small reading renderer for SKILL.md files. It covers the syntax skills use
 * in practice: frontmatter, headings, lists, quotes, tables, fenced code and
 * inline emphasis. It never injects HTML, so a skill cannot script the app.
 */
export function Markdown({ text }: { text: string }) {
  const lines = text.replace(/\r\n/g, "\n").split("\n");
  const blocks: ReactNode[] = [];
  const front: [string, string][] = [];
  let index = 0;

  if (lines[0]?.trim() === "---") {
    index = 1;
    for (; index < lines.length; index += 1) {
      if (lines[index].trim() === "---") {
        index += 1;
        break;
      }
      const line = lines[index];
      const at = line.indexOf(":");
      if (at > 0) {
        front.push([
          line.slice(0, at).trim(),
          line
            .slice(at + 1)
            .trim()
            .replace(/^["']|["']$/g, "")
        ]);
      } else if (line.trim()) {
        front.push(["note", line.trim()]);
      }
    }
  }

  if (front.length > 0) {
    blocks.push(
      <dl className="md-front" key="frontmatter">
        {front.map(([key, value], position) => (
          <Fragment key={`${key}-${position}`}>
            <dt>{key}</dt>
            <dd>{value}</dd>
          </Fragment>
        ))}
      </dl>
    );
  }

  while (index < lines.length) {
    const source = lines[index];
    const trimmed = source.trim();
    const key = `block-${index}`;

    if (!trimmed) {
      index += 1;
      continue;
    }

    if (trimmed.startsWith("```")) {
      const language = trimmed.slice(3).trim();
      const body: string[] = [];
      index += 1;
      while (index < lines.length && !lines[index].trim().startsWith("```")) {
        body.push(lines[index]);
        index += 1;
      }
      index += 1;
      blocks.push(
        <pre className="md-code" key={key} data-language={language || undefined}>
          <code>{body.join("\n")}</code>
        </pre>
      );
      continue;
    }

    const heading = /^(#{1,4})\s+(.*)$/.exec(trimmed);
    if (heading) {
      const size = heading[1].length;
      const content = inline(heading[2]);
      blocks.push(
        size === 1 ? (
          <h1 key={key}>{content}</h1>
        ) : size === 2 ? (
          <h2 key={key}>{content}</h2>
        ) : size === 3 ? (
          <h3 key={key}>{content}</h3>
        ) : (
          <h4 key={key}>{content}</h4>
        )
      );
      index += 1;
      continue;
    }

    if (/^(-{3,}|\*{3,}|_{3,})$/.test(trimmed)) {
      blocks.push(<hr key={key} />);
      index += 1;
      continue;
    }

    if (isListItem(trimmed)) {
      const ordered = /^\d+\./.test(trimmed);
      const items: string[] = [];
      while (index < lines.length && isListItem(lines[index].trim())) {
        items.push(lines[index].trim().replace(/^([-*+]|\d+\.)\s+/, ""));
        index += 1;
      }
      const rendered = items.map((item, position) => (
        <li key={position}>{inline(item)}</li>
      ));
      blocks.push(
        ordered ? <ol key={key}>{rendered}</ol> : <ul key={key}>{rendered}</ul>
      );
      continue;
    }

    if (trimmed.startsWith(">")) {
      const quote: string[] = [];
      while (index < lines.length && lines[index].trim().startsWith(">")) {
        quote.push(lines[index].trim().replace(/^>\s?/, ""));
        index += 1;
      }
      blocks.push(<blockquote key={key}>{inline(quote.join(" "))}</blockquote>);
      continue;
    }

    if (trimmed.startsWith("|")) {
      const rows: string[][] = [];
      while (index < lines.length && lines[index].trim().startsWith("|")) {
        const cells = lines[index]
          .trim()
          .replace(/^\||\|$/g, "")
          .split("|")
          .map((cell) => cell.trim());
        if (!cells.every((cell) => /^:?-{2,}:?$/.test(cell))) rows.push(cells);
        index += 1;
      }
      if (rows.length > 0) {
        const [head, ...body] = rows;
        blocks.push(
          <div className="md-table" key={key}>
            <table>
              <thead>
                <tr>
                  {head.map((cell, position) => (
                    <th key={position}>{inline(cell)}</th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {body.map((row, rowIndex) => (
                  <tr key={rowIndex}>
                    {row.map((cell, cellIndex) => (
                      <td key={cellIndex}>{inline(cell)}</td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        );
      }
      continue;
    }

    const paragraph: string[] = [];
    while (index < lines.length && isParagraphLine(lines[index].trim())) {
      paragraph.push(lines[index].trim());
      index += 1;
    }
    if (paragraph.length === 0) {
      index += 1;
      continue;
    }
    blocks.push(<p key={key}>{inline(paragraph.join(" "))}</p>);
  }

  return <div className="reader">{blocks}</div>;
}

function isListItem(line: string) {
  return /^([-*+]|\d+\.)\s+/.test(line);
}

function isParagraphLine(line: string) {
  if (!line) return false;
  if (/^(#{1,4}\s|```|>) /.test(line)) return false;
  if (line.startsWith(">")) return false;
  if (isListItem(line)) return false;
  if (line.startsWith("|")) return false;
  if (/^(-{3,}|\*{3,}|_{3,})$/.test(line)) return false;
  return true;
}

function inline(text: string): ReactNode[] {
  const pattern = /(`[^`]+`|\*\*[^*]+\*\*|\[[^\]]+\]\([^)]+\))/g;
  return text.split(pattern).map((part, position) => {
    if (!part) return null;
    if (part.startsWith("`") && part.endsWith("`")) {
      return <code key={position}>{part.slice(1, -1)}</code>;
    }
    if (part.startsWith("**") && part.endsWith("**")) {
      return <strong key={position}>{part.slice(2, -2)}</strong>;
    }
    const link = /^\[([^\]]+)\]\(([^)]+)\)$/.exec(part);
    if (link) {
      return (
        <span className="md-link" key={position} title={link[2]}>
          {link[1]}
        </span>
      );
    }
    return <Fragment key={position}>{part}</Fragment>;
  });
}
