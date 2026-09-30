# Azeradio setup

About 10 minutes total, and most of it is clicking around Spotify's website once. You do each part exactly once.

## What you need

- Spotify Premium. Playback control is a Premium-only API, so a free account will stay silent.
- Spotify open on at least one device: your PC, your phone, anything.

## Part 1: your Spotify key (about 5 minutes, once ever)

Spotify caps shared keys at a handful of users, so everyone uses their own free app key. It is just a client ID string. No secret, no server, nothing to host, nothing to pay.

1. Go to https://developer.spotify.com/dashboard and log in with your normal Spotify account (the one with Premium).
2. Click **Create app**.
3. App name: `Azeradio`. App description: `WoW zone soundtracks`. (The values do not matter.)
4. Tick the agreement checkbox and click **Create**.
5. You land on your new app's page. Click **Settings** in the top right.
6. Find the **Redirect URIs** box. Paste this exactly:
   `http://127.0.0.1:8899/callback`
   Click **Add**, then scroll to the bottom and click **Save**. Two easy mistakes here: skipping Add, and skipping Save. Also note it must be `127.0.0.1`, not `localhost`. Spotify rejects `localhost` now.
7. On that same Settings page, copy the **Client ID** (the long mix of letters and numbers). Ignore the client secret completely. Azeradio never asks for it.

That is the whole Spotify side. Leave the tab open in case you need to double check step 6 later.

## Part 2: connect Azeradio (about 2 minutes, once)

1. Install the tray app from the latest `tray/v*` release and run it. The first run creates its config files.
2. Right-click the tray icon and choose **Open config folder**.
3. Open `spotify.json` in Notepad. Paste your client ID between the quotes:
   ```json
   {
     "client_id": "paste-your-client-id-here",
     "access_token": "",
     "refresh_token": "",
     "expires_at": 0
   }
   ```
   Paste it with no extra spaces inside the quotes.
4. Save the file and wait a few seconds. Your browser opens at Spotify on its own. Log in if asked, then click **Agree**.
5. The tab says "Azeradio is connected." Close it.

Done. Azeradio refreshes its own login from here on. If Spotify ever logs you out months down the line, the browser simply opens again by itself.

## Part 3: WoW (about 2 minutes, once)

1. Download the latest `Azeradio-x.y.z.zip` from Releases and unzip it into your `Interface/AddOns` folder. The same zip works for Midnight (`_retail_`) and Forever (`_classic_beta_`).
2. In game, type `/console chatLog 1` and press enter. This switches on the chat log file Azeradio reads. Once is enough.
3. Log out and back in, or type `/reload`. Then type `/azeradio test`. If an `[AZERADIO]` line appears in chat, the bridge is working.

## Part 4: map your music (the fun part)

1. Right-click the tray icon, **Open config folder**, open `mappings.json`.
2. To get a playlist's address: in Spotify, right-click a playlist, choose **Share**, then **Copy Spotify URI**. It looks like `spotify:playlist:37i9dQZF1DX...`. You want the URI, not the web link.
3. Fill in the map:
   ```json
   {
     "zones": { "Orgrimmar": "spotify:playlist:..." },
     "subzones": { "Valdrakken": "spotify:playlist:..." },
     "instances": { "Amirdrassil, the Dream's Hope": "spotify:playlist:..." },
     "combat_playlist": "spotify:playlist:...",
     "fallback_playlist": "spotify:playlist:..."
   }
   ```
   Albums and artists work too, anywhere a playlist does.
4. Save. That is it. No restart: the next zone change picks up your edits.

How matching works: the most specific name wins. Subzone first, then zone, then instance, then the fallback. The combat playlist wins over all of them while you are in combat, and the zone music comes back when combat ends. Zone names must match the game exactly; `/azeradio test` prints what the addon sees if you are unsure of a spelling.

## Troubleshooting

- No `[AZERADIO]` lines in chat: the addon is not enabled (check the AddOns button on the character select screen), or chat logging is off (redo step 2 of Part 3).
- The browser never opens after pasting the client ID: open `spotify.json` again and check the ID sits between the quotes with no stray spaces. Then wait about ten seconds.
- Spotify shows an error page instead of the Agree button: the redirect URI does not match. Go back to the dashboard Settings and confirm `http://127.0.0.1:8899/callback` is in the list and that you clicked Save.
- Zones change but nothing plays: is Spotify open on a device? Is the account Premium? Does `mappings.json` have a URI for that zone, or a fallback?
- Wrong music for a zone: the name in `mappings.json` must match the game exactly, including apostrophes. Use `/azeradio test` to see the exact names.
- The addon shows as "out of date" after a WoW patch: tick "Load out of date AddOns" on the character select screen, or grab the newest release.
