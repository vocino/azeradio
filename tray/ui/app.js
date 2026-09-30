const { invoke } = window.__TAURI__.core;

const pill = document.getElementById('pill');
const input = document.getElementById('clientId');
const saveMsg = document.getElementById('saveMsg');

function render(s) {
  document.getElementById('cidHint').textContent =
    s.client_id_set && s.client_id_hint ? 'Using client ID ' + s.client_id_hint : '';
  const cls = !s.client_id_set ? 'off' : s.connected ? 'on' : 'wait';
  pill.className = 'pill ' + cls;
  pill.textContent = !s.client_id_set
    ? 'Not set up'
    : s.connected
      ? 'Connected to Spotify'
      : 'Saved. Waiting for Spotify login…';
}

async function refresh() {
  try {
    render(await invoke('get_status'));
  } catch (e) {
    // bridge still starting; the pill keeps its default
  }
}

document.getElementById('save').addEventListener('click', async () => {
  saveMsg.textContent = '';
  try {
    await invoke('save_client_id', { clientId: input.value });
    saveMsg.textContent = 'Saved. Your browser opens at Spotify in a few seconds.';
    input.value = '';
    // Poll until the login completes and the pill flips to connected.
    for (let i = 0; i < 15; i++) {
      await new Promise((r) => setTimeout(r, 2000));
      const s = await invoke('get_status');
      render(s);
      if (s.connected) {
        saveMsg.textContent = 'Connected. You can close this window.';
        break;
      }
    }
  } catch (e) {
    saveMsg.textContent = String(e);
  }
});

document.getElementById('openConfig').addEventListener('click', () => invoke('open_config_folder'));
document.getElementById('openGuide').addEventListener('click', () => invoke('open_guide'));
document.getElementById('dashLink').addEventListener('click', (e) => {
  e.preventDefault();
  invoke('open_dashboard');
});

// --- World of Warcraft ---

const FLAVORS = ['retail', 'classic', 'forever'];
const cap = (s) => s[0].toUpperCase() + s.slice(1);

function wowStatusText(s) {
  const via = s.source === 'manual' ? 'manual' : s.source === 'auto' ? 'auto-detected' : '';
  const suffix = via ? ' (' + via + ')' : '';
  if (s.status === 'ok') return { cls: 'ok', text: 'Chat log found' + suffix };
  if (s.status === 'no_log') return { cls: 'warn', text: 'Folder found, chat logging is off' + suffix + ' — run /console chatLog 1 in game' };
  if (s.source === 'manual') return { cls: 'bad', text: 'Folder not found — check the path' };
  return { cls: 'bad', text: 'Not installed or not found' };
}

async function refreshWow() {
  try {
    const w = await invoke('get_wow');
    for (const f of FLAVORS) {
      const s = w[f];
      const input = document.getElementById('wow' + cap(f));
      // Don't clobber a field the user is typing in.
      if (document.activeElement !== input) input.value = s.manual;
      input.placeholder = s.path || 'auto-detect…';
      input.title = s.path || '';
      const st = wowStatusText(s);
      const el = document.getElementById('wow' + cap(f) + 'St');
      el.className = 'wstat ' + st.cls;
      el.textContent = st.text;
    }
  } catch (e) {
    // bridge still starting; statuses stay empty
  }
}

document.getElementById('saveWow').addEventListener('click', async () => {
  const msg = document.getElementById('wowMsg');
  msg.textContent = '';
  try {
    await invoke('save_wow', {
      retail: document.getElementById('wowRetail').value,
      classic: document.getElementById('wowClassic').value,
      forever: document.getElementById('wowForever').value,
    });
    msg.textContent = 'Saved. The watcher picks it up within seconds.';
    refreshWow();
  } catch (e) {
    msg.textContent = String(e);
  }
});

// --- Music ---

const MAPS = ['zones', 'subzones', 'instances'];

function mapRow(list, name = '', uri = '') {
  const box = document.getElementById('list' + cap(list));
  const empty = box.querySelector('.empty');
  if (empty) empty.remove();
  const div = document.createElement('div');
  div.className = 'maprow';
  div.innerHTML = '<input class="nm" type="text" placeholder="Name in game" autocomplete="off" spellcheck="false">' +
    '<input class="ur" type="text" placeholder="spotify:playlist:…" autocomplete="off" spellcheck="false">' +
    '<button class="xbtn" title="Remove">×</button>';
  const [nm, ur] = div.querySelectorAll('input');
  nm.value = name;
  ur.value = uri;
  div.querySelector('.xbtn').addEventListener('click', () => {
    div.remove();
    if (!box.querySelector('.maprow')) {
      box.innerHTML = '<p class="empty">Nothing mapped yet.</p>';
    }
  });
  box.appendChild(div);
  return div;
}

async function refreshMusic() {
  try {
    const m = await invoke('get_mappings');
    for (const k of MAPS) {
      const box = document.getElementById('list' + cap(k));
      box.innerHTML = '';
      const entries = Object.entries(m[k] || {}).sort(([a], [b]) => a.localeCompare(b));
      if (!entries.length) box.innerHTML = '<p class="empty">Nothing mapped yet.</p>';
      for (const [name, uri] of entries) mapRow(k, name, uri);
    }
    const set = (id, v) => {
      const el = document.getElementById(id);
      if (document.activeElement !== el) el.value = v || '';
    };
    set('combatUri', m.combat_playlist);
    set('fallbackUri', m.fallback_playlist);
    set('deviceId', m.device_id);
  } catch (e) {
    // bridge still starting; lists stay empty
  }
}

function collectMap(k) {
  const o = {};
  document.getElementById('list' + cap(k)).querySelectorAll('.maprow').forEach((r) => {
    const [nm, ur] = r.querySelectorAll('input');
    if (nm.value.trim() && ur.value.trim()) o[nm.value.trim()] = ur.value.trim();
  });
  return o;
}

const optVal = (id) => document.getElementById(id).value.trim() || null;

document.querySelectorAll('[data-add]').forEach((b) => {
  b.addEventListener('click', () => {
    const row = mapRow(b.getAttribute('data-add'));
    row.querySelector('.nm').focus();
  });
});

document.getElementById('saveMusic').addEventListener('click', async () => {
  const msg = document.getElementById('musicMsg');
  msg.classList.remove('err');
  msg.textContent = '';
  try {
    await invoke('save_mappings', {
      m: {
        zones: collectMap('zones'),
        subzones: collectMap('subzones'),
        instances: collectMap('instances'),
        combat_playlist: optVal('combatUri'),
        fallback_playlist: optVal('fallbackUri'),
        device_id: optVal('deviceId'),
      },
    });
    msg.textContent = 'Saved. Next zone change picks it up.';
    refreshMusic();
  } catch (e) {
    msg.textContent = String(e);
    msg.classList.add('err');
  }
});

// --- Recently seen ---

async function refreshRecent() {
  try {
    const box = document.getElementById('recentChips');
    box.innerHTML = '';
    const seen = new Set();
    let n = 0;
    for (const ev of await invoke('get_recent')) {
      for (const [name, list] of [[ev.zone, 'zones'], [ev.subzone, 'subzones'], [ev.instance_name, 'instances']]) {
        if (!name || seen.has(list + '\0' + name) || n >= 12) continue;
        seen.add(list + '\0' + name);
        n += 1;
        const b = document.createElement('button');
        b.className = 'chip';
        b.textContent = name;
        b.title = 'Add to ' + list;
        b.addEventListener('click', () => {
          const row = mapRow(list, name, '');
          row.querySelector('.ur').focus();
        });
        box.appendChild(b);
      }
    }
    if (!n) box.innerHTML = '<p class="empty">Play the game and zones show up here.</p>';
  } catch (e) {
    // bridge still starting
  }
}

// --- Bridge status ---

async function refreshBridge() {
  try {
    const b = await invoke('get_bridge');
    const dot = document.getElementById('bridgeDot');
    const txt = document.getElementById('bridgeText');
    if (b.last_error) {
      dot.className = 'bdot bad';
      txt.textContent = b.last_error;
    } else if (b.logs_watched === 0) {
      dot.className = 'bdot warn';
      txt.textContent = 'No chat logs found — see World of Warcraft below';
    } else {
      dot.className = 'bdot ok';
      txt.textContent = 'Watching ' + b.logs_watched + ' log' + (b.logs_watched > 1 ? 's' : '') +
        (b.last_event ? ' · Last: ' + b.last_event : '');
    }
  } catch (e) {
    // bridge still starting
  }
}

refresh();
refreshWow();
refreshMusic();
refreshRecent();
refreshBridge();
setInterval(() => {
  refreshWow();
  refreshRecent();
  refreshBridge();
}, 5000);
