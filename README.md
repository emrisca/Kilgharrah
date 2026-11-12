<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/banner-dark.png">
  <source media="(prefers-color-scheme: light)" srcset="assets/banner-light.png">
  <img alt="The Kilgharrah dragon, drawn in ASCII art" src="assets/banner-light.png" width="1280">
</picture>

# Kilgharrah

A desktop manager for the skills and MCP servers the Freebuff CLI loads. Install from the ClawHub
registry, keep your own skills in category folders, and flip one off without deleting its files.

The Home section opens first: a medieval display wordmark, a short description, live counts, and
an ASCII dragon flapping over a slow rain of glyphs.

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

Requires Node 20 or newer and a Rust toolchain. macOS also needs the Xcode command
line tools; Windows needs the MSVC build tools and WebView2, which the Tauri
prerequisites installer sets up.

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
in-app mark is a copy of that file at `src/assets/mark.png`. The README banner comes from
`python3 scripts/make-banner.py`, which draws `ascii-art.txt` directly with plain Python, so no
image library is needed.

## Home

The Home section is the default tab. It renders three layers on top of the base
theme:

- The wordmark uses Metamorphous, bundled at
  `src/assets/fonts/metamorphous-latin.woff2`, so the heading looks the same on
  macOS and Windows instead of picking up whatever serif the system has. The face
  is licensed under the SIL Open Font License 1.1, and `src/assets/fonts/OFL.txt`
  keeps the licence text next to the file. Vite fingerprints the font into
  `dist/assets`, and `font-display: swap` falls back to Palatino while it loads.
- `src/assets/dragon.txt` holds thirty-one frames of the flying dragon as plain
  text, each 70 by 23 characters. The frames were converted from a GIF, and the
  forest behind the dragon was removed before the conversion, so only the
  creature is drawn. A blurred copy sits behind the sharp one for the glow, and
  the frame index advances every 60ms.
- Twenty-two seeded columns of glyphs fall behind the copy. The seed keeps the
  layout stable when a skill is toggled elsewhere in the app.
- A handful of embers rise past the art, timed on different loops so they do not
  pulse together.

All of it uses the active palette, including the sixth palette, Merlin (forest
green and candle gold), which exists for this section. Under
`prefers-reduced-motion` the rain, embers and glow are removed and the dragon
holds its first frame rather than flapping.

## Updates

The app checks the signed updater endpoint on launch and every four hours. A new
build shows a banner with the version and a download bar, and the restart is one
click. The endpoint is the latest GitHub release:
`https://github.com/emrisca/Kilgharrah/releases/latest/download/latest.json`.

Pushing a tag that matches `v*` runs `.github/workflows/release.yml`, which
builds Apple silicon, Intel Mac and Windows bundles and publishes them with
`latest.json`. The workflow needs the `TAURI_SIGNING_PRIVATE_KEY` secret. The
version comes from `tauri.conf.json` and `package.json`, so bump both before
tagging.

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
- The Home tab is first. The other tabs keep their state while it is open.
- Motion uses springs that carry velocity through a reversal, and all of it collapses under
  `prefers-reduced-motion`.

## Layout

```
kilgharrah/
  src/                   React views, the spring helper, the updater and the token sheet
  src/components/HomeView.tsx  Home section: rain, wordmark, dragon, embers
  src/assets/dragon.txt  Thirty-one ASCII frames of the flying dragon
  src/assets/fonts/      Bundled Metamorphous web font and its OFL licence
  src/updater.ts         Signed update check, download and relaunch
  src-tauri/src/         Rust commands: store, links, mcp.json, ClawHub
  .github/workflows/     Release build for macOS and Windows
  assets/                App icon source and the README banner, dark and light
  ascii-art.txt          Source art for the README banner
  scripts/ascii_render.py Glyph renderer used by the banner script
  scripts/make-banner.py Renders the README banner in both themes
```
