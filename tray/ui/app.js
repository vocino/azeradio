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

refresh();
refreshWow();
setInterval(refreshWow, 5000);
