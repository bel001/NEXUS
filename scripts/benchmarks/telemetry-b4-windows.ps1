param(
    [Parameter(Mandatory = $true)][string]$Executable,
    [ValidateRange(5, 30)][int]$Runs = 10,
    [ValidateRange(1, 5)][int]$Warmup = 2,
    [ValidateRange(1000, 60000)][int]$TimeoutMs = 15000,
    [string]$ReportPath = ""
)
$ErrorActionPreference = "Stop"

$exe = (Resolve-Path -LiteralPath $Executable -ErrorAction Stop).Path
if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) { throw "Executable not found." }
$repo = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path

function Get-Percentile([double[]]$Numbers, [double]$Fraction) {
    if ($Numbers.Count -eq 0) { return $null }
    $ordered = @($Numbers | Sort-Object)
    $index = [int][Math]::Ceiling($Fraction * $ordered.Count) - 1
    return [double]$ordered[[Math]::Max(0, $index)]
}

# Deterministic check before any native sample.
if ((Get-Percentile -Numbers @(1.0,2.0,3.0,4.0,5.0) -Fraction 0.95) -ne 5.0) {
    throw "Percentile self-test failed."
}

function Invoke-Sample([string]$Argument, [string]$Metric) {
    $startInfo = New-Object System.Diagnostics.ProcessStartInfo
    $startInfo.FileName = $exe
    $startInfo.Arguments = $Argument
    $startInfo.WorkingDirectory = $repo
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $startInfo.StandardOutputEncoding = New-Object System.Text.UTF8Encoding($false)
    $startInfo.StandardErrorEncoding = New-Object System.Text.UTF8Encoding($false)
    $process = New-Object System.Diagnostics.Process
    $process.StartInfo = $startInfo
    $watch = New-Object System.Diagnostics.Stopwatch
    try {
        $watch.Start()
        if (-not $process.Start()) { throw "Could not launch agent." }
        # Keep a native process handle and observe the peak while the child is alive.
        $null = $process.Handle
        $peakBytes = [long]0
        $cpuMs = $null
        $peakIssue = "not_observed"
        $cpuIssue = "not_observed"
        $outTask = $process.StandardOutput.ReadToEndAsync()
        $errTask = $process.StandardError.ReadToEndAsync()
        $finished = $false
        do {
            try {
                $process.Refresh()
                $samplePeak = [long]$process.PeakWorkingSet64
                if ($samplePeak -gt $peakBytes) { $peakBytes = $samplePeak }
                if ($samplePeak -gt 0) { $peakIssue = "none" }
            } catch { $peakIssue = $_.Exception.GetType().Name }
            try {
                $sampleCpu = [double]$process.TotalProcessorTime.TotalMilliseconds
                if ($sampleCpu -ge 0 -and -not [double]::IsNaN($sampleCpu) -and
                    -not [double]::IsInfinity($sampleCpu)) {
                    $cpuMs = $sampleCpu
                    $cpuIssue = "none"
                }
            } catch { $cpuIssue = $_.Exception.GetType().Name }
            $finished = $process.WaitForExit(2)
            if (-not $finished -and $watch.ElapsedMilliseconds -ge $TimeoutMs) {
                try { $process.Kill() } catch { }
                $null = $process.WaitForExit(1000)
                throw "Agent sample exceeded timeout ($TimeoutMs ms)."
            }
        } while (-not $finished)
        $process.WaitForExit()
        $watch.Stop()
        $stdout = $outTask.GetAwaiter().GetResult()
        $null = $errTask.GetAwaiter().GetResult() # Never log raw diagnostics.
        if ($process.ExitCode -ne 0) { throw "Agent returned nonzero exit code." }
        try { $json = $stdout | ConvertFrom-Json } catch { throw "Agent emitted invalid JSON." }
        if ($json.schema_version -ne 1) { throw "Unexpected telemetry schema." }
        switch ($Metric) {
            "memory" {
                if ($null -eq $json.memory_total_visible_bytes -or
                    $null -eq $json.memory_available_bytes) { throw "Missing memory fields." }
            }
            "cpu" {
                if ($null -eq $json.cpu_usage -or
                    $null -eq $json.cpu_usage.usage_percent) { throw "Missing CPU fields." }
            }
            "power" {
                if ($null -eq $json.power_source -or
                    $null -eq $json.battery_percent) { throw "Missing power fields." }
            }
            default { throw "Unexpected metric." }
        }
        # A final read can improve CPU accounting; never replace a valid peak with zero.
        try {
            $finalCpu = [double]$process.TotalProcessorTime.TotalMilliseconds
            if ($finalCpu -ge 0 -and -not [double]::IsNaN($finalCpu) -and
                -not [double]::IsInfinity($finalCpu)) {
                $cpuMs = $finalCpu
                $cpuIssue = "none"
            }
        } catch { $cpuIssue = $_.Exception.GetType().Name }
        try {
            $finalPeak = [long]$process.PeakWorkingSet64
            if ($finalPeak -gt $peakBytes) { $peakBytes = $finalPeak }
            if ($finalPeak -gt 0) { $peakIssue = "none" }
        } catch { $peakIssue = $_.Exception.GetType().Name }
        if ($null -eq $cpuMs -or $peakBytes -le 0) {
            throw "OS process counters missing: CPU=$cpuIssue; PeakWS=$peakIssue. No fabricated readings."
        }
        return [pscustomobject]@{
            wall_ms = [Math]::Round($watch.Elapsed.TotalMilliseconds, 3)
            process_cpu_ms = [Math]::Round($cpuMs, 3)
            peak_working_set_bytes = $peakBytes
        }
    }
    finally {
        $watch.Stop()
        $process.Dispose()
    }
}

$modes = @(
    @{ name = "memory"; argument = "--telemetry-memory" },
    @{ name = "cpu"; argument = "--telemetry-cpu" },
    @{ name = "power"; argument = "--telemetry-power" }
)
$entries = @()
foreach ($mode in $modes) {
    Write-Host "=== B4 $($mode.name): $Warmup warmups + $Runs measured ==="
    for ($w = 0; $w -lt $Warmup; $w++) {
        $null = Invoke-Sample -Argument $mode.argument -Metric $mode.name
    }
    $samples = @()
    for ($i = 1; $i -le $Runs; $i++) {
        $sample = Invoke-Sample -Argument $mode.argument -Metric $mode.name
        $samples += $sample
    }
    $wall = [double[]]@($samples | ForEach-Object { $_.wall_ms })
    $cpu = [double[]]@($samples | ForEach-Object { $_.process_cpu_ms })
    $rss = [double[]]@($samples | ForEach-Object { $_.peak_working_set_bytes })
    $result = [pscustomobject]@{
        metric = $mode.name
        sample_count = $samples.Count
        wall_ms_p50 = [Math]::Round((Get-Percentile $wall 0.50), 3)
        wall_ms_p95 = [Math]::Round((Get-Percentile $wall 0.95), 3)
        process_cpu_ms_p50 = [Math]::Round((Get-Percentile $cpu 0.50), 3)
        process_cpu_ms_p95 = [Math]::Round((Get-Percentile $cpu 0.95), 3)
        peak_working_set_mib_p95 = [Math]::Round((Get-Percentile $rss 0.95) / 1MB, 2)
        samples = $samples
    }
    $entries += $result
}

Write-Host "=== B4 WINDOWS: SUMMARY (includes startup and JSON output) ===" -ForegroundColor Cyan
$entries | Select-Object metric, sample_count, wall_ms_p50, wall_ms_p95, process_cpu_ms_p95, peak_working_set_mib_p95 | Format-Table -AutoSize
Write-Host "Energy in watts/joules: NOT MEASURED"
Write-Host "Process CPU is CPU time (not system CPU usage percent)."
Write-Host "RSS peak includes shared memory and runtime startup; no hardware IDs collected."

if ([string]::IsNullOrWhiteSpace($ReportPath)) {
    $ReportPath = Join-Path $env:TEMP ("NEXUS_B4_WINDOWS_" + (Get-Date -Format "yyyyMMdd_HHmmss_fff") + ".json")
}
if (Test-Path -LiteralPath $ReportPath) { throw "Report already exists; no file overwritten." }
$report = [pscustomobject]@{
    schema_version = 1
    platform = "windows"
    profile = "release"
    method = "external_process"
    includes_process_startup = $true
    wall_clock_includes_json_output = $true
    energy_measured = $false
    runs = $Runs
    warmup_runs = $Warmup
    timeout_ms = $TimeoutMs
    result = $entries
}
$writer = New-Object System.Text.UTF8Encoding($false)
$bytes = $writer.GetBytes(($report | ConvertTo-Json -Depth 8))
$stream = [IO.File]::Open($ReportPath, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
try { $stream.Write($bytes, 0, $bytes.Length) } finally { $stream.Dispose() }
Write-Host "B4 Windows report (local TEMP): $ReportPath" -ForegroundColor Green
Write-Host "B4 Windows metrics: PASS" -ForegroundColor Green
