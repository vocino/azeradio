# Azeradio hub (site/)

The public home for the Azeroth soundtrack playlists. Plain static
HTML, CSS, and JavaScript with no build step, so Cloudflare Pages can
serve this directory directly. Planned home: azeradio.vocino.com.

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
