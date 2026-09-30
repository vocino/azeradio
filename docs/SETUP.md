# Azeradio setup

## 1. Spotify (one time)

Azeradio drives Spotify through the official Web API. You need:

- Spotify Premium (playback control is Premium-only)
- Spotify open on at least one device (PC, phone, anything)

Create a Spotify app:

1. Go to the [Spotify developer dashboard](https://developer.spotify.com/dashboard) and create an app.
2. In the app settings, add this redirect URI: `http://127.0.0.1:8899/callback`
3. Copy the client ID and client secret.

On first run the tray app creates `%APPDATA%/azeradio/spotify.json`. Fill in the two values:

```json
{
  "client_id": "...",
  "client_secret": "...",
  "access_token": "",
  "refresh_token": "",
  "expires_at": 0
}
```

Then launch the tray app. It opens your browser once to connect Spotify, and refreshes its own tokens after that.

## 2. WoW

1. Install the addon from the latest `addon/v*` release into `Interface/AddOns`.
2. Turn on chat logging: tick Log Chat in the in-game options (Social section), or run `/console chatLog 1`.
3. The log lands at `<WoW>/_retail_/Logs/WoWChatLog.txt` (or `_classic_beta_/Logs/` for Forever). The tray app finds it on its own. To override, set the `AZERADIO_WOW_LOG` environment variable.

You should see `[AZERADIO]` lines appear in chat as you change zones. If you do not, the addon is not enabled or chat logging is off.

## 3. Map your music

Right-click the tray icon, open the config folder, edit `mappings.json`. Values are Spotify URIs (`spotify:playlist:...`, albums and artists work too).

- `zones`: full zone names, e.g. `"Orgrimmar"`
- `subzones`: minimap subzone names, e.g. `"Valdrakken"`
- `instances`: dungeon/raid names, e.g. `"Amirdrassil, the Dream's Hope"`
- `combat_playlist`: plays whenever you are in combat
- `fallback_playlist`: plays when nothing else matches
- `device_id`: optional. Pin playback to one device; leave empty to use whichever device is active.

Most specific match wins: subzone, then zone, then instance, then fallback. Combat always wins while you are in combat.

## Addon slash commands

- `/azeradio`: status
- `/azeradio combat on|off`: toggle combat announcements
- `/azeradio test`: emit a test line

## Troubleshooting

- Nothing switches on zone change: confirm chat logging is on and you see `[AZERADIO]` lines in chat.
- Spotify error 404: open Spotify on a device first.
- Spotify error 403: playback control needs Premium.
- Wrong playlist for a zone: check the exact in-game name. `/azeradio test` prints what the addon sees.
