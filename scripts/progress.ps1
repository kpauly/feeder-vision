# scripts/progress.ps1
$path = "specs\tasks.md"
if (-not (Test-Path $path)) { Write-Error "Missing $path"; exit 1 }

$content = Get-Content $path -Raw

$all = ([regex]::Matches($content, '\[ \]|\[x\]', 'IgnoreCase')).Count
$done = ([regex]::Matches($content, '\[x\]', 'IgnoreCase')).Count
if ($all -eq 0) { Write-Host "No checkboxes found."; exit 0 }

$percent = [math]::Round(100.0 * $done / $all, 1)
Write-Host "Progress: $done / $all ($percent`%)"

# Per-section breakdown (lines starting with '## ')
$lines = $content -split "`r?`n"
$section = ""
$sectionTotals = @{}
$sectionDone = @{}
foreach ($line in $lines) {
    if ($line -match '^##\s+(.*)') { $section = $Matches[1]; continue }
    if ($line -match '\[( |x)\]') {
        if (-not $sectionTotals.ContainsKey($section)) { $sectionTotals[$section] = 0; $sectionDone[$section] = 0 }
        $sectionTotals[$section]++
        if ($line -match '\[x\]') { $sectionDone[$section]++ }
    }
}
foreach ($k in $sectionTotals.Keys) {
    $p = [math]::Round(100.0 * $sectionDone[$k] / $sectionTotals[$k], 1)
    Write-Host (" - {0}: {1}/{2} ({3}%)" -f $k, $sectionDone[$k], $sectionTotals[$k], $p)
}
