<#
.SYNOPSIS
  Soak test: run Pulse for a while and watch for a memory leak.

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
    [string]$Exe = "src-tauri/target/debug/pulse.exe",
    [string]$OutFile = "soak.csv"
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $Exe)) {
    throw "Binary not found at $Exe. Build it first: npm run tauri build -- --debug --no-bundle"
}

$process = Start-Process -FilePath $Exe -PassThru
"soak: pid $($process.Id), $Hours h, sampling every $IntervalSecs s"

$samples = @()
$deadline = (Get-Date).AddHours($Hours)

try {
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Seconds $IntervalSecs

        if ($process.HasExited) {
            throw "Pulse exited unexpectedly with code $($process.ExitCode)"
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
"growth {0:N2} MB/h (budget {1:N2})" -f $perHour, $MaxGrowthMbPerHour

if ($perHour -gt $MaxGrowthMbPerHour) {
    "FAIL: memory grew faster than the budget"
    exit 1
}

"PASS"
exit 0