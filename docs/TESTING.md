# Testing Azeradio locally

One command, from the repo root:

```
.\scripts\test.ps1
```

It runs every check that can run on this machine, one section per area, and
ends with a summary like this:

```
== Summary ==
  [PASS] Addon syntax
  [PASS] Addon contract tests
  [PASS] Zone library
  [BLOCKED] Tray unit tests
No failures. BLOCKED sections (if any) are covered by CI -- see docs/TESTING.md.
```

## What each section means

| Section | What runs | What it proves |
|---|---|---|
| Addon syntax | `luac -p addon/Azeradio/core.lua` | The addon compiles. |
| Addon contract tests | `lua addon/tests/test_core.lua` (28 checks, no dependencies) | Every bridge line the addon writes matches the format the tray parses in `tray/src/bridge.rs`: prefix, all seven keys, combat flag, `why` tags, quote escaping, slash commands. |
| Zone library | `node scripts/check-library.js` | `tray/library.json` parses and every row is a real `spotify:` URI. |
| Tray unit tests | `cargo test --manifest-path tray/Cargo.toml` | Playlist picking, library merge, and WoW path resolution. |

States: **PASS** ran and is green. **FAIL** ran and is red -- fix it before
tagging. **BLOCKED** could not run here; the section says where the real gate
lives.

## Why the tray section is BLOCKED locally

Smart App Control on the dev machine blocks locally compiled Rust build
scripts (`os error 4551`), so `cargo test` cannot link here. That is a
machine policy, not a code failure. The `tray-check` workflow runs the same
`cargo test` on every push to `main`, so CI is the gate for that section: a
green run there plus no FAILs locally means the tree is good. The same
applies in reverse -- if CI goes red on `tray-check`, the tray code is broken
even when the local run shows BLOCKED.

## Manual bridge test (game to Spotify, end to end)

Automated checks stop at parsing. To see a real zone change flip music:

1. In game, type `/azeradio test`. Chat prints a confirmation (bridge lines
   themselves are invisible by design).
2. Right-click the tray icon, open Settings, and check the status line: it
   should show your zone as its last event, plus the activity feed entry.
3. To test without launching WoW, point the tray at a fake log:
   set `AZERADIO_WOW_LOG` to a file, append a bridge line, and watch the
   status line pick it up within seconds:
   ```
   [AZERADIO] zone="Orgrimmar" subzone="Valley of Honor" instance="none" instanceName="" instanceID="0" combat="0" why="test"
   ```
   Zone names must match the game exactly; the click-to-add shortcuts in
   Settings → Music are spelled exactly right.

## Before tagging a release

Run `.\scripts\test.ps1` (no FAILs) and confirm `tray-check` is green on the
pushed commit. Then follow `docs/RELEASING.md`.
