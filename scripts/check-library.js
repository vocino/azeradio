// Zone library check. Runs anywhere Node exists (local script and CI share it):
//   node scripts/check-library.js [path/to/library.json]
// Verifies tray/library.json still matches the Mappings shape the tray loads
// in tray/src/config.rs: three name->URI tables, optional combat/fallback
// URIs, every URI a real spotify: URI. Exits 0 on success, 1 on failure.
const fs = require("fs");
const path = require("path");

const file = process.argv[2] || path.join(__dirname, "..", "tray", "library.json");

let failed = 0;
function check(name, cond, detail) {
  if (cond) {
    console.log("  ok: " + name);
  } else {
    failed += 1;
    console.log("  FAIL: " + name + (detail ? " -- " + detail : ""));
  }
}

console.log("zone library (" + file + ")");

let m;
try {
  m = JSON.parse(fs.readFileSync(file, "utf8"));
} catch (e) {
  console.log("  FAIL: parses as JSON -- " + e.message);
  process.exit(1);
}
check("parses as JSON", true);

const tables = ["zones", "subzones", "instances"];
for (const t of tables) {
  check(t + " is a name->URI table", m[t] !== null && typeof m[t] === "object" && !Array.isArray(m[t]));
}

let rows = 0;
for (const t of tables) {
  const table = m[t] || {};
  for (const [name, uri] of Object.entries(table)) {
    rows += 1;
    if (typeof uri !== "string" || !uri.startsWith("spotify:")) {
      check(t + ' "' + name + '" is a spotify: URI', false, String(uri));
    }
  }
}
check("every row is a spotify: URI", failed === 0, rows + " rows checked");
check("library is not empty", rows > 0, rows + " rows");

for (const key of ["combat_playlist", "fallback_playlist"]) {
  const v = m[key];
  check(key + " is empty or a spotify: URI", v == null || (typeof v === "string" && (v === "" || v.startsWith("spotify:"))), String(v));
}

console.log(failed === 0 ? rows + " rows, all valid" : failed + " check(s) failed");
process.exit(failed === 0 ? 0 : 1);
