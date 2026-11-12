<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/banner-dark.png">
  <source media="(prefers-color-scheme: light)" srcset="assets/banner-light.png">
  <img alt="The Kilgharrah dragon, drawn in ASCII art" src="assets/banner-light.png" width="1280">
</picture>

# Kilgharrah

A desktop manager for the skills and MCP servers the Freebuff CLI loads. Install from the ClawHub
registry, keep your own skills in category folders, and flip one off without deleting its files.

## What it touches

| Path | Role |
| --- | --- |
| `~/.agents/kilgharrah/skills/<category>/<slug>/` | The store. One folder per skill, grouped by category. |
| `~/.agents/skills/<slug>` | Symlink for each enabled skill. This is what the CLI reads. |
| `~/.agents/mcp.json` | MCP servers, edited in place with the rest of the file preserved. |

Disabling a skill removes its symlink. The store copy stays. Removing it deletes both.

The store starts with six categories: Design, Development, Writing, Research, Productivity and
Other. Folders you add by hand are listed too.

## Run it

Requires Node 20 or newer, Rust, and the Xcode command line tools on macOS.

```bash
cd kilgharrah
npm install
npm run tauri dev
```

Build a bundle:

```bash
npm run tauri build
```

The app has no Swift or native dependencies outside Tauri. The Dock icon comes from
`assets/kilicon.png`, and `npx tauri icon assets/kilicon.png` rebuilds the bundle set from it. The
README banner comes from `python3 scripts/make-banner.py`, and the in-app mark from
`python3 scripts/make-icon.py`. Both draw `ascii-art.txt` directly with plain Python, so no image
library is needed.

## Marketplace

Kilgharrah reads the public ClawHub v1 API, no token needed:

- `GET /api/v1/skills?sort=downloads` fills the shelf when the search box is empty.
- `GET /api/v1/search?q=` finds skills by name and meaning.
- `GET /api/v1/packages/search?q=` finds plugin packages, shown on the MCP tab.
- `GET /api/v1/skills/{slug}` and `/versions/{version}` give the file list, and each file comes from
  `/file?path=&version=`. Kilgharrah writes exactly those files, up to 200 files and 12 MB.

The install sheet fetches `SKILL.md` before you commit, because a skill is a set of instructions and
you should read them first. Plugin packages are not MCP configs, so Kilgharrah copies their npm command
instead of guessing one.

## Behaviour worth knowing

- Search runs as you type, 320 ms after the last key. Press `/` to focus it from anywhere.
- The install button puts the skill in the category ClawHub's topics suggest. Pick another category
  in the sheet first if you disagree.
- Removing a skill asks once, in place. There is no dialog stack.
- MCP servers are written under `mcpServers` and keep every other key in `mcp.json`.
- Motion uses springs that carry velocity through a reversal, and all of it collapses under
  `prefers-reduced-motion`.

## Layout

```
kilgharrah/
  src/                   React views, the spring helper and the token sheet
  src-tauri/src/         Rust commands: store, links, mcp.json, ClawHub
  assets/                App icon source and the README banner, dark and light
  ascii-art.txt          Source art for the icon and the banner
  scripts/ascii_render.py Glyph and tile renderer used by both scripts
  scripts/make-icon.py   Renders the PNG, icns, ico set and the in-app mark
  scripts/make-banner.py Renders the README banner in both themes
```
