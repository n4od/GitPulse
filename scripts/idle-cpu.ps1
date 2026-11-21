<#
.SYNOPSIS
  Idle CPU check: confirm GitPulse is quiet when nothing is happening.

.DESCRIPTION
  A background monitor that burns 5% of a core at idle is a battery complaint.
  This samples total processor time over a window with the window hidden and
  reports the average, exiting non-zero above the budget.

  Run it with the app hidden (close the window; close-to-tray keeps it alive)
  and with no builds in flight.

.EXAMPLE
  ./scripts/idle-cpu.ps1 -Minutes 10 -MaxPercent 1.0

.NOTES
  Not yet executed: it needs a real signed-in profile. See VERIFICATION.md.
#>
param(
    [int]$Minutes = 10,
    [int]$IntervalSecs = 15,
    [double]$MaxPercent = 1.0,
    [string]$Exe = ""
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

$cores = [Environment]::ProcessorCount
$process = Start-Process -FilePath $Exe -PassThru
"idle-cpu: pid $($process.Id), $Minutes min, $cores logical cores"

# Let startup settle: the first poll and the migration are not idle behaviour.
Start-Sleep -Seconds 30

$process.Refresh()
$lastCpu = $process.TotalProcessorTime
$lastAt = Get-Date
$readings = @()
$deadline = (Get-Date).AddMinutes($Minutes)

try {
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Seconds $IntervalSecs

        if ($process.HasExited) {
            throw "GitPulse exited unexpectedly with code $($process.ExitCode)"
        }

        $process.Refresh()
        $nowCpu = $process.TotalProcessorTime
        $nowAt = Get-Date

        $cpuSeconds = ($nowCpu - $lastCpu).TotalSeconds
        $wallSeconds = ($nowAt - $lastAt).TotalSeconds
        $percent = if ($wallSeconds -gt 0) { ($cpuSeconds / ($wallSeconds * $cores)) * 100 } else { 0 }

        $readings += $percent
        "{0}  {1:N2}% of total CPU" -f $nowAt.ToString("HH:mm:ss"), $percent

        $lastCpu = $nowCpu
        $lastAt = $nowAt
    }
}
finally {
    if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force }
}

$average = if ($readings.Count -gt 0) { ($readings | Measure-Object -Average).Average } else { 0 }
$peak = if ($readings.Count -gt 0) { ($readings | Measure-Object -Maximum).Maximum } else { 0 }

""
"average {0:N2}%  peak {1:N2}%  (budget {2:N2}%)" -f $average, $peak, $MaxPercent

if ($average -gt $MaxPercent) {
    "FAIL: idle CPU above budget"
    exit 1
}

"PASS"
exit 0