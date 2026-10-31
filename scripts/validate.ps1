param(
    [ValidateSet("", "1.88.0", "stable")]
    [string]$Toolchain = ""
)

$ErrorActionPreference = "Stop"
$CargoPrefix = @()
if ($Toolchain) {
    $CargoPrefix = @("+$Toolchain")
}

& cargo @CargoPrefix fmt --all -- --check
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
& cargo @CargoPrefix check --workspace --all-targets --locked
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
& cargo @CargoPrefix test --workspace --locked
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
& cargo @CargoPrefix clippy --workspace --all-targets --locked -- -D warnings
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
