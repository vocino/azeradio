# Azeradio

Zone-triggered Spotify soundtracks for World of Warcraft.

## The problem

WoW's music is fine, but it is the same everywhere. You want Orgrimmar to sound different from the Dragon Isles, combat to hit different from exploring, and boss pulls to have their own anthem, all from your own Spotify library.

## How it works

WoW addons cannot reach the network, so Azeradio is two pieces with a log file between them:

1. The addon (`addon/Azeradio`) writes one machine-readable line into the chat log file on every zone change and combat start/end. It is invisible in game.
2. The tray app (`tray/`) tails `Logs/WoWChatLog.txt`, matches the zone to a playlist in your `mappings.json`, and tells Spotify to play it through the official Web API.

One addon folder serves both Midnight and Forever. Two `.toc` files (`120100` for Midnight, `16001` for Forever), one shared `core.lua`. Each client loads the `.toc` that matches it.

This is ToS-safe: the game writes the log file itself, and the tray app only uses Spotify's official API. Nothing reads WoW's memory.

## Install

- **Addon:** download [Azeradio.zip](https://github.com/vocino/azeradio/releases/download/addon/latest/Azeradio.zip) (always the newest) and unzip it into `Interface/AddOns`. The same zip works in `_retail_` (Midnight) and `_classic_beta_` (Forever).
- **Tray:** download and run [Azeradio-setup.exe](https://github.com/vocino/azeradio/releases/download/tray/latest/Azeradio-setup.exe) (always the newest). Then follow `docs/SETUP.md`: about 10 minutes, most of it clicking around Spotify's website once.

Older versions live under [Releases](https://github.com/vocino/azeradio/releases).

## Map zones to playlists

Right-click the tray icon and open Settings → Music: combat and fallback playlists up top, one row per zone, subzone, or instance below. Play first and the names you visit appear as click-to-add shortcuts. The same data lives in `mappings.json`, shaped like this:

```json
{
  "zones": { "Orgrimmar": "spotify:playlist:..." },
  "subzones": { "Valdrakken": "spotify:playlist:..." },
  "instances": { "Amirdrassil, the Dream's Hope": "spotify:playlist:..." },
  "combat_playlist": "spotify:playlist:...",
  "fallback_playlist": "spotify:playlist:..."
}
```

Most specific match wins: subzone, then zone, then instance, then fallback. Combat always wins while you are in combat.

## Releases

- `addon/v*` tags package the addon zip.
- `tray/v*` tags build the Windows installer.

Both publish to GitHub Releases automatically. Full process: `docs/RELEASING.md`.

## What's inside

- `addon/Azeradio/`: the WoW addon (Midnight + Forever)
- `tray/`: the Tauri tray bridge (Rust)
- `docs/SETUP.md`: Spotify setup and config reference
