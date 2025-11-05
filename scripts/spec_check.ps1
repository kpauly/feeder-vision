# scripts/spec_check.ps1
Write-Host "Checking that all scenarios are covered by tests..."

# Collect all "Scenario <number>" lines from spec
$scenarioLines = Select-String -Path "specs\scenarios.md" -Pattern "Scenario\s+\d+" | ForEach-Object { $_.Matches.Value } | Sort-Object -Unique

if (-not $scenarioLines) {
    Write-Error "No scenarios found in specs\scenarios.md"
    exit 1
}

$missing = @()

foreach ($s in $scenarioLines) {
    $found = Select-String -Path "tests\*.rs" -Pattern $s -Quiet
    if (-not $found) {
        $missing += $s
    }
}

if ($missing.Count -gt 0) {
    Write-Host "❌ Missing scenarios in tests:"
    $missing | ForEach-Object { Write-Host "  $_" }
    exit 1
}
else {
    Write-Host "✅ All scenarios referenced in tests."
    exit 0
}
