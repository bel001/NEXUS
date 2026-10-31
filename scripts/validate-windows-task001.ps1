param(
    [switch]$WriteAnonymizedCapture
)

$ErrorActionPreference = "Stop"

if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
    throw "TASK-001 native validation requires a Windows host"
}

$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$CapturePath = Join-Path $RepositoryRoot "docs/examples/hardware-inventory-windows.anonymized.json"

function Assert-CargoSuccess {
    param([string]$Step)

    if ($LASTEXITCODE -ne 0) {
        throw "$Step failed with exit code $LASTEXITCODE"
    }
}

function Assert-UsableObservation {
    param(
        [Parameter(Mandatory = $true)]$Observation,
        [Parameter(Mandatory = $true)][string]$Name,
        [switch]$AllowEmptyCollection
    )

    if ($Observation.status -notin @("available", "partial")) {
        throw "$Name must be available or partial, got '$($Observation.status)'"
    }
    if ($null -eq $Observation.value) {
        throw "$Name has no value"
    }
    if (-not $AllowEmptyCollection -and $Observation.value -is [System.Array] -and $Observation.value.Count -eq 0) {
        throw "$Name must not be an empty collection"
    }
}

Push-Location $RepositoryRoot
try {
    foreach ($Toolchain in @("1.88.0", "stable")) {
        & (Join-Path $PSScriptRoot "validate.ps1") -Toolchain $Toolchain
        Assert-CargoSuccess "workspace validation with Rust $Toolchain"

        & cargo "+$Toolchain" test -p adaptive-platform-windows --locked -- --nocapture
        Assert-CargoSuccess "native WMI tests with Rust $Toolchain"
    }

    $JsonLines = & cargo +stable run --quiet --locked -p adaptive-agent
    Assert-CargoSuccess "native Windows inventory capture"
    $RawJson = $JsonLines -join [Environment]::NewLine
    $Inventory = $RawJson | ConvertFrom-Json

    if ($Inventory.schema_version -ne 1) {
        throw "unexpected inventory schema version '$($Inventory.schema_version)'"
    }
    if ([string]::IsNullOrWhiteSpace($Inventory.captured_at)) {
        throw "captured_at is required when the Windows civil clock is available"
    }
    $CapturedAt = [DateTimeOffset]::Parse(
        $Inventory.captured_at,
        [Globalization.CultureInfo]::InvariantCulture,
        [Globalization.DateTimeStyles]::RoundtripKind
    )
    if ($CapturedAt.Offset -ne [TimeSpan]::Zero) {
        throw "captured_at must be serialized in UTC"
    }
    if ($RawJson -match '"timestamp"\s*:') {
        throw "per-observation timestamp found in the inventory"
    }

    Assert-UsableObservation $Inventory.operating_system "operating_system"
    Assert-UsableObservation $Inventory.cpu "cpu"
    Assert-UsableObservation $Inventory.memory "memory"
    Assert-UsableObservation $Inventory.gpus "gpus" -AllowEmptyCollection
    Assert-UsableObservation $Inventory.storage "storage"
    Assert-UsableObservation $Inventory.batteries "batteries" -AllowEmptyCollection
    Assert-UsableObservation $Inventory.capabilities "capabilities"

    foreach ($Issue in @($Inventory.issues)) {
        if ($Issue.detail.Length -gt 240 -or $Issue.detail -match '[\r\n]') {
            throw "inventory contains an unsanitized issue detail"
        }
    }

    $SensitivePatterns = @(
        '(?i)[A-Z]:\\\\Users\\\\',
        '(?i)/home/',
        '(?i)/sys/',
        '(?i)/proc/',
        '(?i)machine[-_ ]?id',
        '(?i)serial(number)?',
        '(?i)S-1-5-[0-9-]+'
    )
    if ($env:USERPROFILE) {
        $SerializedUserProfile = ($env:USERPROFILE | ConvertTo-Json -Compress).Trim('"')
        $SensitivePatterns += [Regex]::Escape($SerializedUserProfile)
    }
    foreach ($Pattern in $SensitivePatterns) {
        if ($RawJson -match $Pattern) {
            throw "inventory privacy validation matched a prohibited pattern"
        }
    }

    $Queries = Get-Content -Raw (Join-Path $RepositoryRoot "crates/platform-windows/src/queries.rs")
    $ProductionQueries = ($Queries -split '#\[cfg\(test\)\]')[0]
    if ($ProductionQueries -match '(?i)SELECT\s+\*') {
        throw "a Windows query uses SELECT *"
    }
    $WindowsSource = Get-ChildItem (Join-Path $RepositoryRoot "crates/platform-windows/src") -Filter *.rs |
        ForEach-Object { Get-Content -Raw $_.FullName }
    if (($WindowsSource -join "`n") -match '(?i)ExecMethod|PowerShell|Command::new|std::process::Command') {
        throw "Windows production source contains a prohibited modifying or external-command API"
    }

    if ($WriteAnonymizedCapture) {
        $Utf8WithoutBom = New-Object System.Text.UTF8Encoding($false)
        [IO.File]::WriteAllText($CapturePath, "$RawJson`n", $Utf8WithoutBom)
    }

    [PSCustomObject]@{
        schema_version = $Inventory.schema_version
        captured_at = $Inventory.captured_at
        cpu_status = $Inventory.cpu.status
        battery_status = $Inventory.batteries.status
        battery_count = @($Inventory.batteries.value).Count
        storage_status = $Inventory.storage.status
        storage_count = @($Inventory.storage.value).Count
        issue_count = @($Inventory.issues).Count
        capture_written = [bool]$WriteAnonymizedCapture
        capture_path = if ($WriteAnonymizedCapture) { $CapturePath } else { $null }
    } | ConvertTo-Json
}
finally {
    Pop-Location
}
