param(
    [Parameter(Mandatory = $true)]
    [string]$Version,
    [switch]$SkipPush,
    [switch]$AllowDirty,
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$scriptPath = Join-Path $PSScriptRoot "bump-version.py"
$arguments = @("--version", $Version)

if ($SkipPush) {
    $arguments += "--skip-push"
}

if ($AllowDirty) {
    $arguments += "--allow-dirty"
}

if ($DryRun) {
    $arguments += "--dry-run"
}

uv run --script $scriptPath @arguments
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}
