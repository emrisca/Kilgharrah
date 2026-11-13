# Windows handoff — Emrys profile and skill scan

This file splits one task across two machines. The Mac has written the code; this
document is the Windows half. Read it, do the checks in order, and report the
outputs back.

The rule for the split: **only one machine edits `src-tauri/src/main.rs`.** Do not
edit it here. If a check fails, report the failure with its exact output instead
of patching around it.

## What the Mac changed

| File | Change |
| --- | --- |
| `src-tauri/src/main.rs` | Added `frontmatter_platforms`, `is_cross_platform`, `portable_hermes_candidates`; renamed the built-in profile from Hermes to Emrys and made it drop single-platform skills; added `scan_installed_skills` and `import_installed_skills`; two new unit tests |
| `src/types.ts` | `ProfileInfo.excludedNames`, new `DiscoveredSkill` |
| `src/api.ts` | `scanInstalledSkills`, `importInstalledSkills` |
| `src/components/SettingsView.tsx` | New Scan panel, Emrys wording, "left out" line on the profile row |
| `WINDOWS-HANDOFF.md` | This file |

The built-in profile is now derived from the Hermes install at runtime, not from a
stored JSON file:

- **Included:** a skill whose frontmatter has no `platforms:` key, or whose
  `platforms:` list contains both `macos` and `windows`.
- **Excluded:** anything else. On this Mac that is `apple-notes`,
  `apple-reminders`, `findmy`, `imessage` (all `[macos]`) and `xurl`
  (`[linux, macos]`).

## Checks to run here

### 1. Build and unit tests

```powershell
cd kilgharrah
npm ci
node node_modules\typescript\bin\tsc --noEmit
cd src-tauri
cargo test --quiet
```

Expected: `tsc` exits 0; `cargo test` passes 25 tests, including
`emrys_leaves_out_single_platform_skills` and
`frontmatter_platforms_reads_the_inline_list`. Paste any failure in full.

### 2. Hermes resolves to the Windows home

The known bug this guards is a POSIX `HOME` (`/c/Users/...`) reaching Rust and
breaking every path. `home_dir()` prefers `USERPROFILE` on Windows.

```powershell
echo $env:USERPROFILE
echo $env:HERMES_HOME
Test-Path "$env:USERPROFILE\.hermes\skills"
```

If Hermes lives elsewhere, set `HERMES_HOME` (the folder containing `skills`) or
`KILGHARRAH_HERMES` (the skills folder itself) before launching the app, then
confirm Settings → Profiles shows the real path on the Emrys row.

### 3. Emrys contains the right skills

Launch the app, open Settings → Profiles.

- The row is named **Emrys**, marked built in.
- The skill count matches the number of cross-platform skills in your Hermes
  install.
- The row lists the excluded names under "Left out, not cross platform".
- `imessage`, `apple-notes`, `apple-reminders` and `findmy` are **not** in the
  skill chips.
- The MCP server chips match `%USERPROFILE%\.hermes\config.yaml`.

Then press **Apply** and confirm the library gains those skills and the servers,
with no macOS-only skill copied in.

### 4. The scan finds real folders, not mirrors

```powershell
New-Item -ItemType Directory -Force "$env:USERPROFILE\.claude\skills\scan-probe"
@"
---
name: scan-probe
description: Probe skill for the scan feature.
platforms: [linux, macos, windows]
---
# Scan probe
"@ | Set-Content "$env:USERPROFILE\.claude\skills\scan-probe\SKILL.md"
```

Settings → Scan → **Scan for installed skills**. Expect:

- `scan-probe` listed, source `Claude Code`, marked portable, with Import enabled.
- Any skill already in the library marked `in library` with no Import button.
- No duplicate flood from `~/.agents/skills` mirrors — those are symlinks, and
  symlinks are skipped.

Press **Import**, confirm the skill appears in the library at
`<store>\skills\<category>\scan-probe`, then delete the probe folder and the
library copy.

### 5. Import cannot be pointed anywhere

This is the safety property, worth one attempt: the command refuses any path that
is not inside a folder the scan itself offers. Passing
`C:\Windows\System32` must import nothing rather than copy files.

## Windows-specific risks to look at

- Junctions and directory symlinks: `symlink_metadata(...).file_type().is_symlink()`
  reports both. A junction mirror of the library should be skipped, same as a
  POSIX symlink. If a mirror still appears as findings, say so — that is a real
  bug, not a cosmetic one.
- Path comparison in `import_installed_skills` uses `Path::starts_with`, which is
  component-wise and case-sensitive on Windows in Rust. If a scan result is
  refused with a path that visually matches, that is the cause; report it rather
  than lowercasing paths by hand.
- `copy_dir` must handle a skill folder with nested `scripts/` and `references/`.

## Reply with

1. `cargo test` and `tsc` results.
2. The Emrys row: skill count, excluded names, server names, resolved location.
3. Whether the probe skill was found, imported, and skipped afterwards as
   already-in-library.
4. Anything that looked wrong, with the exact message.
