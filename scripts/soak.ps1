<#
.SYNOPSIS
  Soak test: run GitPulse for a while and watch for a memory leak.

.DESCRIPTION
  A monitor that grows a few megabytes an hour is a monitor that gets killed by
  the OS overnight. This samples the resident set at a fixed interval and reports
  growth per hour, exiting non-zero if it exceeds a budget.

  It needs a signed-in profile with repositories in it — an idle app with an
  empty cache will not exhibit the leak this is looking for. Point it at a
  profile that has been running normally.

.EXAMPLE
  ./scripts/soak.ps1 -Hours 8 -MaxGrowthMbPerHour 2

.NOTES
  Not yet executed: it needs a real GitHub token to produce a representative
  load. See VERIFICATION.md.
#>
param(
    [int]$Hours = 8,
    [int]$IntervalSecs = 60,
    [double]$MaxGrowthMbPerHour = 2.0,
    [string]$Exe = "",
    [string]$OutFile = "soak.csv",
    [int]$Minutes = 0
)

$ErrorActionPreference = "Stop"

# Resolve the binary instead of assuming a build tree exists: after a cleanup the
# only copy may be the released one under releases/. Prefer a release build, then
# a debug build, then whichever kept release binary is newest.
if (-not $Exe) {
    foreach ($candidate in @("src-tauri/target/release/gitpulse.exe", "src-tauri/target/debug/gitpulse.exe")) {
        if (Test-Path $candidate) { $Exe = $candidate; break }
    }
}
if (-not $Exe -or -not (Test-Path $Exe)) {
    $kept = Get-ChildItem "releases" -Recurse -File -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -in @("GitPulse.exe", "gitpulse.exe") } |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if ($kept) { $Exe = $kept.FullName }
}
if (-not $Exe -or -not (Test-Path $Exe)) {
    throw "Binary not found. Build one (npm run tauri build) or pass -Exe <path>."
}

$process = Start-Process -FilePath $Exe -PassThru
$window = if ($Minutes -gt 0) { "$Minutes min" } else { "$Hours h" }
"soak: pid $($process.Id), $window, sampling every $IntervalSecs s"

$samples = @()
$deadline = if ($Minutes -gt 0) { (Get-Date).AddMinutes($Minutes) } else { (Get-Date).AddHours($Hours) }

try {
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Seconds $IntervalSecs

        if ($process.HasExited) {
            throw "GitPulse exited unexpectedly with code $($process.ExitCode)"
        }

        $process.Refresh()
        $sample = [pscustomobject]@{
            At     = (Get-Date).ToString("o")
            RssMb  = [math]::Round($process.WorkingSet64 / 1MB, 2)
            PrivateMb = [math]::Round($process.PrivateMemorySize64 / 1MB, 2)
        }

        $samples += $sample
        $samples | Export-Csv -Path $OutFile -NoTypeInformation
        "{0}  rss {1:N1} MB" -f $sample.At, $sample.RssMb
    }
}
finally {
    if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force }
}

$first = $samples | Select-Object -First 1
$last = $samples | Select-Object -Last 1
$elapsedHours = ((Get-Date $last.At) - (Get-Date $first.At)).TotalHours
$growth = $last.RssMb - $first.RssMb
$perHour = if ($elapsedHours -gt 0) { $growth / $elapsedHours } else { 0 }

""
"start {0:N1} MB -> end {1:N1} MB over {2:N2} h" -f $first.RssMb, $last.RssMb, $elapsedHours

# Extrapolating a per-hour rate from a few minutes turns ordinary allocator
# jitter into a fake leak: 0.4 MB of movement over five minutes reads as
# ~5 MB/h. Below half an hour the only honest verdict is "not enough data".
if ($elapsedHours -lt 0.5) {
    "movement {0:N2} MB total - too small a window for a per-hour rate" -f $growth
    "INCONCLUSIVE: run with -Hours 1 (or more) for a verdict"
    exit 0
}

"growth {0:N2} MB/h (budget {1:N2})" -f $perHour, $MaxGrowthMbPerHour

if ($perHour -gt $MaxGrowthMbPerHour) {
    "FAIL: memory grew faster than the budget"
    exit 1
}

"PASS"
exit 0