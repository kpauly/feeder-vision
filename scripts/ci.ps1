# scripts/ci.ps1
Write-Host "Running cargo format..."
cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "Running cargo clippy..."
cargo clippy --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "Running cargo test..."
cargo test --all
exit $LASTEXITCODE
