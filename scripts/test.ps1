# Azeradio local testing entry point. Run from the repo root:
#   .\scripts\test.ps1
# One section per area, each ending in PASS, FAIL, or BLOCKED (BLOCKED means
# the check could not run on this machine -- the summary says where the real
# gate lives). Exits 1 when anything FAILs, 0 otherwise.
# Full process: docs/TESTING.md.

$script:results = @()
$script:sectionState = ""

function Invoke-Section {
    param([string]$Name, [scriptblock]$Body)
    Write-Host ""
    Write-Host "== $Name =="
    $script:sectionState = ""
    & $Body
    $script:results += [pscustomobject]@{ Area = $Name; State = $script:sectionState }
}

function Test-Command-Found([string]$Name) {
    return [bool](Get-Command $Name -ErrorAction SilentlyContinue)
}

if (-not (Test-Path "addon/Azeradio/core.lua")) {
    Write-Host "Run this from the repo root (it needs .\addon and .\tray)."
    exit 1
}

Invoke-Section "Addon syntax" {
    if (-not (Test-Command-Found "luac")) {
        Write-Host "BLOCKED: luac not on PATH (install Lua to run this locally)."
        $script:sectionState = "BLOCKED"
        return
    }
    luac -p addon/Azeradio/core.lua
    if ($LASTEXITCODE -eq 0) {
        Write-Host "PASS: core.lua compiles."
        $script:sectionState = "PASS"
    } else {
        Write-Host "FAIL: luac rejected core.lua."
        $script:sectionState = "FAIL"
    }
}

Invoke-Section "Addon contract tests" {
    if (-not (Test-Command-Found "lua")) {
        Write-Host "BLOCKED: lua not on PATH (install Lua to run this locally)."
        $script:sectionState = "BLOCKED"
        return
    }
    lua addon/tests/test_core.lua
    $coreCode = $LASTEXITCODE
    lua addon/tests/test_options.lua
    $optionsCode = $LASTEXITCODE
    if (($coreCode -eq 0) -and ($optionsCode -eq 0)) {
        Write-Host "PASS: addon bridge format matches the tray contract."
        $script:sectionState = "PASS"
    } else {
        Write-Host "FAIL: addon contract tests failed (see lines above)."
        $script:sectionState = "FAIL"
    }
}

Invoke-Section "Zone library" {
    if (-not (Test-Command-Found "node")) {
        Write-Host "BLOCKED: node not on PATH."
        $script:sectionState = "BLOCKED"
        return
    }
    node scripts/check-library.js
    if ($LASTEXITCODE -eq 0) {
        Write-Host "PASS: library.json matches the Mappings shape."
        $script:sectionState = "PASS"
    } else {
        Write-Host "FAIL: library.json is invalid (see lines above)."
        $script:sectionState = "FAIL"
    }
}

Invoke-Section "Tray unit tests" {
    if (-not (Test-Command-Found "cargo")) {
        Write-Host "BLOCKED: cargo not on PATH."
        $script:sectionState = "BLOCKED"
        return
    }
    $log = Join-Path $env:TEMP "azeradio-cargo-test.log"
    cargo test --manifest-path tray/Cargo.toml > $log 2>&1
    $code = $LASTEXITCODE
    Write-Host ((Get-Content $log -Tail 12) -join "`r`n")
    if (Select-String -Path $log -Pattern "Application Control|os error 4551|blocked this file" -Quiet) {
        Write-Host "BLOCKED: Smart App Control stops locally compiled build scripts on this machine."
        Write-Host "The tray-check workflow is the gate for this section: it runs the same cargo test on every push."
        $script:sectionState = "BLOCKED"
    } elseif ($code -eq 0) {
        Write-Host "PASS: cargo test green."
        $script:sectionState = "PASS"
    } else {
        Write-Host "FAIL: cargo test failed (see output above)."
        $script:sectionState = "FAIL"
    }
    Remove-Item $log -ErrorAction SilentlyContinue
}

Write-Host ""
Write-Host "== Summary =="
$failed = 0
foreach ($r in $script:results) {
    Write-Host ("  [{0}] {1}" -f $r.State, $r.Area)
    if ($r.State -eq "FAIL") { $failed += 1 }
}
if ($failed -gt 0) {
    Write-Host "$failed area(s) failing."
    exit 1
}
Write-Host "No failures. BLOCKED sections (if any) are covered by CI -- see docs/TESTING.md."
exit 0
