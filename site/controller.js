// Azeradio controller: play the zone playlists on your own Spotify.
//
// Bring-your-own client ID, like the tray app and the old radio page:
// the ID and the tokens live in this browser's localStorage and nowhere
// else - the hub is static and never sees them. Auth is PKCE (no secret,
// none exists to keep). The redirect lands back on the hub root, which
// is the URI to register in the Spotify app's settings.
//
// Connected, a card click plays its playlist on the active Spotify
// device and the bar below shows what is playing, with transport,
// volume, and progress. Not connected, no active device, or any API
// error, and the cards fall back to the in-page embed player - the
// board never goes dead.

var CLIENT_ID_KEY = 'azeradio:client-id';
var AUTH_KEY = 'azeradio:auth';
var VERIFIER_KEY = 'azeradio:verifier';
var SCOPES = 'user-modify-playback-state user-read-playback-state';
var POLL_MS = 5000;

function store(key) {
  try {
    return localStorage.getItem(key);
  } catch (e) {
    return null;
  }
}

function save(key, value) {
  try {
    if (value === null) localStorage.removeItem(key);
    else localStorage.setItem(key, value);
  } catch (e) {
    /* private window: the board still works unconnected */
  }
}

function loadAuth() {
  try {
    var raw = store(AUTH_KEY);
    return raw ? JSON.parse(raw) : null;
  } catch (e) {
    return null;
  }
}

function saveAuth(auth) {
  save(AUTH_KEY, auth ? JSON.stringify(auth) : null);
}

// The hub root. Register exactly this in the Spotify app's settings.
function redirectUri() {
  return window.location.origin + '/';
}

function base64url(bytes) {
  var bin = '';
  bytes.forEach(function (b) {
    bin += String.fromCharCode(b);
  });
  return btoa(bin).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

function makeVerifier() {
  var chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~';
  var bytes = new Uint8Array(64);
  crypto.getRandomValues(bytes);
  var out = '';
  bytes.forEach(function (b) {
    out += chars[b % chars.length];
  });
  return out;
}

async function makeChallenge(verifier) {
  var digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(verifier));
  return base64url(new Uint8Array(digest));
}

async function beginAuth(clientId) {
  var verifier = makeVerifier();
  try {
    sessionStorage.setItem(VERIFIER_KEY, verifier);
  } catch (e) {
    return false;
  }
  var params = new URLSearchParams({
    response_type: 'code',
    client_id: clientId,
    scope: SCOPES,
    redirect_uri: redirectUri(),
    code_challenge_method: 'S256',
    code_challenge: await makeChallenge(verifier),
  });
  window.location.assign('https://accounts.spotify.com/authorize?' + params.toString());
  return true;
}

async function tokenRequest(fields) {
  var res;
  try {
    res = await fetch('https://accounts.spotify.com/api/token', {
      method: 'POST',
      headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
      body: new URLSearchParams(fields).toString(),
    });
  } catch (e) {
    return null;
  }
  if (!res.ok) return null;
  var body = await res.json();
  var previous = loadAuth();
  return {
    access: body.access_token,
    refresh: body.refresh_token || (previous && previous.refresh) || null,
    expires: Date.now() + body.expires_in * 1000,
  };
}

async function finishAuth(code) {
  var clientId = store(CLIENT_ID_KEY);
  var verifier = null;
  try {
    verifier = sessionStorage.getItem(VERIFIER_KEY);
    sessionStorage.removeItem(VERIFIER_KEY);
  } catch (e) {
    /* no-op */
  }
  if (!clientId || !verifier) return false;
  var auth = await tokenRequest({
    grant_type: 'authorization_code',
    code: code,
    redirect_uri: redirectUri(),
    client_id: clientId,
    code_verifier: verifier,
  });
  if (!auth) return false;
  saveAuth(auth);
  return true;
}

async function accessToken() {
  var auth = loadAuth();
  if (!auth) return null;
  if (Date.now() < auth.expires - 60000) return auth.access;
  var clientId = store(CLIENT_ID_KEY);
  if (!clientId || !auth.refresh) {
    saveAuth(null);
    return null;
  }
  var next = await tokenRequest({
    grant_type: 'refresh_token',
    refresh_token: auth.refresh,
    client_id: clientId,
  });
  if (!next) {
    saveAuth(null);
    return null;
  }
  saveAuth(next);
  return next.access;
}

// status 0 means the fetch itself failed (offline, blocked): the caller
// decides, and auth is left alone - a network blip is not a sign-out.
async function api(path, options) {
  var token = await accessToken();
  if (!token) return { status: 401, body: null };
  var res;
  try {
    res = await fetch('https://api.spotify.com/v1' + path, {
      method: (options && options.method) || 'GET',
      headers: {
        Authorization: 'Bearer ' + token,
        'Content-Type': 'application/json',
      },
      body: options && options.body ? JSON.stringify(options.body) : undefined,
    });
  } catch (e) {
    return { status: 0, body: null };
  }
  var body = null;
  if (res.status !== 204) {
    try {
      body = await res.json();
    } catch (e) {
      body = null;
    }
  }
  if (res.status === 401) saveAuth(null);
  return { status: res.status, body: body };
}

function fmtTime(ms) {
  var total = Math.max(0, Math.floor((ms || 0) / 1000));
  var m = Math.floor(total / 60);
  var s = total % 60;
  return m + ':' + (s < 10 ? '0' : '') + s;
}

if (typeof document !== 'undefined') {
  document.addEventListener('DOMContentLoaded', initController);
}

function initController() {
  var bar = document.getElementById('controller');
  if (!bar) return;

  var status = document.getElementById('ctl-status');
  var now = document.getElementById('ctl-now');
  var toggle = document.getElementById('ctl-toggle');
  var prev = document.getElementById('ctl-prev');
  var next = document.getElementById('ctl-next');
  var progress = document.getElementById('ctl-progress');
  var time = document.getElementById('ctl-time');
  var volume = document.getElementById('ctl-volume');
  var idWrap = document.getElementById('ctl-id-wrap');
  var idInput = document.getElementById('ctl-id');
  var saveIdButton = document.getElementById('ctl-save-id');
  var connectButton = document.getElementById('ctl-connect');
  var disconnectButton = document.getElementById('ctl-disconnect');

  // What Spotify is doing right now. The poll below is the source of
  // truth; the one-second tick only moves the progress bar between polls.
  var state = {
    playlistId: null,
    track: '',
    artist: '',
    isPlaying: false,
    position: 0,
    duration: 0,
    volume: 100,
  };
  var seeking = false;

  function connected() {
    return !!loadAuth();
  }

  function setStatus(text) {
    if (status) status.textContent = text;
  }

  function authChanged() {
    if (typeof window.__azeradioAuthChanged === 'function') window.__azeradioAuthChanged();
  }

  function markCard() {
    var marked = document.querySelectorAll('.card.is-playing');
    for (var i = 0; i < marked.length; i++) marked[i].classList.remove('is-playing');
    if (!state.playlistId) return;
    var card = document.querySelector('.card[data-spotify-id="' + state.playlistId + '"]');
    if (card) card.classList.add('is-playing');
  }

  function render() {
    var isConnected = connected();
    connectButton.hidden = isConnected;
    disconnectButton.hidden = !isConnected;
    if (toggle) toggle.textContent = state.isPlaying ? 'Pause' : 'Play';
    if (toggle) toggle.disabled = !isConnected || !state.track;
    if (prev) prev.disabled = !isConnected || !state.track;
    if (next) next.disabled = !isConnected || !state.track;
    if (now) {
      if (state.track) {
        now.textContent = state.track + ' - ' + state.artist;
        now.hidden = false;
      } else {
        now.textContent = '';
        now.hidden = true;
      }
    }
    if (progress) {
      progress.max = state.duration > 0 ? String(state.duration) : '1000';
      if (!seeking) progress.value = String(state.position);
      progress.disabled = !isConnected || state.duration <= 0;
    }
    if (time) time.textContent = fmtTime(state.position) + ' / ' + fmtTime(state.duration);
    if (volume && document.activeElement !== volume) volume.value = String(state.volume);
    markCard();
  }

  async function refreshState() {
    if (!connected()) {
      state.playlistId = null;
      state.track = '';
      state.artist = '';
      state.isPlaying = false;
      state.position = 0;
      state.duration = 0;
      setStatus('Not connected. Play here works without connecting.');
      render();
      return;
    }
    var res = await api('/me/player');
    if (res.status === 401) {
      setStatus('Spotify sign-in expired. Connect again.');
      authChanged();
      render();
      return;
    }
    if (res.status === 0) {
      setStatus('Could not reach Spotify. Play here still works.');
      render();
      return;
    }
    if (res.status === 204 || !res.body || !res.body.item) {
      state.playlistId = null;
      state.track = '';
      state.artist = '';
      state.isPlaying = false;
      state.position = 0;
      state.duration = 0;
      setStatus('Connected. Click a zone to play it on Spotify.');
      render();
      return;
    }
    var item = res.body.item;
    state.isPlaying = !!res.body.is_playing;
    state.track = item.name || '';
    state.artist = (item.artists || []).map(function (a) { return a.name; }).join(', ');
    state.position = res.body.progress_ms || 0;
    state.duration = item.duration_ms || 0;
    state.volume = typeof res.body.device.volume_percent === 'number' ? res.body.device.volume_percent : state.volume;
    var uri = res.body.context && res.body.context.uri ? res.body.context.uri : '';
    var id = uri.replace('spotify:playlist:', '');
    state.playlistId = document.querySelector('.card[data-spotify-id="' + id + '"]') ? id : null;
    if (state.playlistId) {
      setStatus(state.isPlaying ? 'Playing on Spotify.' : 'Paused.');
    } else if (state.track) {
      setStatus(state.isPlaying ? 'Playing something else on Spotify.' : 'Paused.');
    } else {
      setStatus('Connected. Click a zone to play it on Spotify.');
    }
    render();
  }

  // True when Spotify accepted the play, false when the caller should
  // fall back to the embed player (or there is nothing to fall back to).
  async function playPlaylist(spotifyId) {
    var devices = await api('/me/player/devices');
    if (devices.status === 401) {
      setStatus('Spotify sign-in expired. Connect again.');
      authChanged();
      render();
      return false;
    }
    if (devices.status === 0) {
      setStatus('Could not reach Spotify. Play here still works.');
      return false;
    }
    var list = (devices.body && devices.body.devices) || [];
    var target = null;
    list.forEach(function (device) {
      if (device.is_active && !target) target = device;
    });
    if (!target) {
      list.forEach(function (device) {
        if (!device.is_restricted && !target) target = device;
      });
    }
    if (!target) {
      setStatus('No Spotify device. Open Spotify, play anything once, then click again.');
      return false;
    }
    var res = await api('/me/player/play?device_id=' + encodeURIComponent(target.id), {
      method: 'PUT',
      body: { context_uri: 'spotify:playlist:' + spotifyId },
    });
    if (res.status === 401) {
      setStatus('Spotify sign-in expired. Connect again.');
      authChanged();
      render();
      return false;
    }
    if (res.status === 0 || res.status >= 400) {
      setStatus('Spotify would not play it. Is Premium active on that account?');
      return false;
    }
    state.playlistId = spotifyId;
    state.isPlaying = true;
    setStatus('Playing on Spotify.');
    render();
    setTimeout(refreshState, 1500);
    return true;
  }

  toggle.addEventListener('click', async function () {
    if (!connected()) return;
    var res = await api(state.isPlaying ? '/me/player/pause' : '/me/player/play', { method: 'PUT' });
    if (res.status === 401) {
      setStatus('Spotify sign-in expired. Connect again.');
      authChanged();
      render();
      return;
    }
    state.isPlaying = !state.isPlaying;
    setStatus(state.isPlaying ? 'Playing on Spotify.' : 'Paused.');
    render();
    setTimeout(refreshState, 800);
  });

  prev.addEventListener('click', async function () {
    if (!connected()) return;
    await api('/me/player/previous', { method: 'POST' });
    setTimeout(refreshState, 800);
  });

  next.addEventListener('click', async function () {
    if (!connected()) return;
    await api('/me/player/next', { method: 'POST' });
    setTimeout(refreshState, 800);
  });

  progress.addEventListener('pointerdown', function () { seeking = true; });
  progress.addEventListener('pointerup', function () { seeking = false; });
  progress.addEventListener('change', async function () {
    if (!connected() || state.duration <= 0) return;
    var at = Number(progress.value) || 0;
    state.position = at;
    render();
    await api('/me/player/seek?position_ms=' + Math.floor(at), { method: 'PUT' });
    setTimeout(refreshState, 800);
  });

  volume.addEventListener('change', async function () {
    if (!connected()) return;
    var level = Number(volume.value);
    state.volume = level;
    await api('/me/player/volume?volume_percent=' + level, { method: 'PUT' });
  });

  connectButton.addEventListener('click', async function () {
    if (!store(CLIENT_ID_KEY)) {
      idWrap.hidden = false;
      if (idInput) idInput.focus();
      setStatus('Paste a Spotify client ID, then Save. The tray app uses the same one.');
      return;
    }
    setStatus('Sending you to Spotify…');
    await beginAuth(store(CLIENT_ID_KEY));
  });

  saveIdButton.addEventListener('click', async function () {
    var value = idInput ? idInput.value.trim() : '';
    if (!value) return;
    save(CLIENT_ID_KEY, value);
    idWrap.hidden = true;
    setStatus('Sending you to Spotify…');
    await beginAuth(value);
  });

  disconnectButton.addEventListener('click', function () {
    saveAuth(null);
    state.playlistId = null;
    state.track = '';
    state.artist = '';
    state.isPlaying = false;
    state.position = 0;
    state.duration = 0;
    setStatus('Not connected. Play here works without connecting.');
    authChanged();
    render();
  });

  // The OAuth return lands here with ?code= (or ?error=): trade the code
  // for tokens, then scrub the URL so a reload does not replay it.
  var params = new URLSearchParams(window.location.search);
  var code = params.get('code');
  var failed = params.get('error');
  if (code || failed) {
    try {
      window.history.replaceState({}, '', window.location.pathname);
    } catch (e) {
      /* no-op */
    }
    if (failed) {
      setStatus('Spotify connect was cancelled.');
      render();
    } else {
      finishAuth(code).then(function (ok) {
        if (!ok) setStatus('Connect failed. Check the client ID and try again.');
        authChanged();
        refreshState();
      });
    }
  } else {
    refreshState();
  }

  setInterval(function () {
    if (document.visibilityState === 'visible' && connected()) refreshState();
  }, POLL_MS);

  // The progress bar moves between polls; the poll corrects any drift.
  setInterval(function () {
    if (!state.isPlaying || state.duration <= 0 || seeking) return;
    state.position = Math.min(state.position + 1000, state.duration);
    if (progress) progress.value = String(state.position);
    if (time) time.textContent = fmtTime(state.position) + ' / ' + fmtTime(state.duration);
  }, 1000);

  // app.js asks this before it opens an embed: connected cards play on
  // Spotify, everything else falls back to the player on the card.
  window.Azeradio = {
    isConnected: connected,
    playPlaylist: playPlaylist,
  };
}
