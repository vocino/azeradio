// Stamps the release version into Cargo.toml and tauri.conf.json.
// Checked in as a file (instead of `node -e` in the workflow) because the
// workflow runs on Windows, where PowerShell mangles nested quoting.
const fs = require('fs');

const v = (process.argv[2] || '').replace(/^tray\/v/, '');
if (!v) {
  console.error('usage: node stamp-version.js <tray/vX.Y.Z tag or version>');
  process.exit(1);
}

const tc = 'tray/tauri.conf.json';
const conf = JSON.parse(fs.readFileSync(tc, 'utf8'));
conf.version = v;
fs.writeFileSync(tc, JSON.stringify(conf, null, 2) + '\n');

const ct = 'tray/Cargo.toml';
fs.writeFileSync(
  ct,
  fs.readFileSync(ct, 'utf8').replace(/^version = .*/m, `version = "${v}"`)
);

console.log('stamped', v);
