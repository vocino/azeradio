# Azeradio hub (site/)

The public home for the Azeroth soundtrack playlists. Plain static
HTML, CSS, and JavaScript with no build step, so Cloudflare Pages can
serve this directory directly. Live at azeradio.vocino.com.

## The Spotify controller

The bar at the bottom of the page (`controller.js`) turns the hub into
a controller for your own Spotify, the same way the old radio page on
wow.vocino.com worked. It is bring-your-own client ID: paste your
Spotify client ID once, and the ID and the tokens live in this
browser's localStorage and nowhere else. Auth is PKCE, so there is no
secret to keep and nothing is sent to any server.

Connect flow:

1. Click Connect Spotify. If no client ID is saved, paste it and click
   Save. The tray app uses the same client ID.
2. Spotify asks you to agree. The redirect lands back on the hub root.
3. Click a zone card and its playlist plays on your active Spotify
   device. The bar shows the track and artist, with Play/Pause, Prev,
   Next, volume, and progress (drag the progress bar to seek). The
   playing card gets the accent border.

Redirect URI to register in the Spotify app's settings:

`https://azeradio.vocino.com/`

That is the hub root, with the trailing slash. The same Spotify app
can hold this URI next to the tray's `http://127.0.0.1:8899/callback`;
register both and the one client ID works everywhere.

Fallback, never a dead end: not connected, no active device, or any
API error, and the cards fall back to the in-page embed player plus
the Open in Spotify link. A playlist that is still private in Spotify
has no embed, so it only plays through the controller once connected.

## How to add a zone

1. Add one entry to `playlists.json` with name, slug, group, spotifyId,
   cover filename, a one-line essence, and public true or false.
2. Drop the pixel cover into `covers/` under the filename in the entry.
3. That is the whole change. Cards, search, chips, and embeds all read
   from `playlists.json`. Nothing is hardcoded in the HTML.

Groups are Capitals, Eastern Kingdoms, and Kalimdor. The chip
row in `index.html` lists them; a new group needs a chip there too.

## The public flag

`public: true` means the playlist is public in Spotify, so the card
gets an in-page embed player. `public: false` means it is still
private, so the card shows the cover and essence plus an Open in
Spotify link only. A private embed renders "not found" to visitors,
which is why the flag gates the iframe. Flip the flag to true once
the playlist is made public in Spotify.

## Run it locally

```
cd site && python3 -m http.server 8000
```

Then open http://localhost:8000. The page fetches `playlists.json`,
so it needs a real server; opening index.html from disk will not
render the cards.
