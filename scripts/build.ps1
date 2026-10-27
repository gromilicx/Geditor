$ErrorActionPreference = "Stop"
Set-Location (Split-Path -Parent $PSScriptRoot)
cargo fmt
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
cargo test
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
cargo clippy --all-targets
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
cargo build --release
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Write-Host "Built: target/release/geditor.exe"
