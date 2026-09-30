const { invoke } = window.__TAURI__.core;

const pill = document.getElementById('pill');
const input = document.getElementById('clientId');
const saveMsg = document.getElementById('saveMsg');

function render(s) {
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

refresh();
