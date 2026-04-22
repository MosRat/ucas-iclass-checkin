param(
    [Parameter(Mandatory = $true)]
    [string]$Version,
    [switch]$SkipPush,
    [switch]$AllowDirty,
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Write-Info {
    param([string]$Message)
    Write-Host "[bump-version] $Message"
}

function Assert-Success {
    param(
        [int]$ExitCode,
        [string]$Message
    )

    if ($ExitCode -ne 0) {
        throw $Message
    }
}

function Replace-Regex {
    param(
        [string]$Content,
        [string]$Pattern,
        [string]$Replacement,
        [string]$Path
    )

    $updated = [System.Text.RegularExpressions.Regex]::Replace(
        $Content,
        $Pattern,
        $Replacement,
        [System.Text.RegularExpressions.RegexOptions]::Multiline
    )

    if ($updated -eq $Content) {
        throw "failed to update version in $Path"
    }

    return $updated
}

function Write-Utf8NoBom {
    param(
        [string]$Path,
        [string]$Content
    )

    $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($Path, $Content, $utf8NoBom)
}

if ($Version -notmatch '^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$') {
    throw "version must be a SemVer string like 0.2.0 or 1.0.0-beta.1"
}

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
Set-Location $repoRoot

$trackedFiles = @(
    "Cargo.toml",
    "apps/iclass-gui/package.json",
    "apps/iclass-gui/src-tauri/tauri.conf.json"
)

$tag = "app-v$Version"
$commitMessage = "release(version): bump to $Version"

if (-not $AllowDirty -and -not $DryRun) {
    $statusOutput = git status --porcelain
    Assert-Success $LASTEXITCODE "failed to inspect git status"
    if ($statusOutput) {
        throw "working tree is not clean; commit or stash changes first, or pass -AllowDirty"
    }
}

$cargoTomlPath = Join-Path $repoRoot "Cargo.toml"
$packageJsonPath = Join-Path $repoRoot "apps/iclass-gui/package.json"
$tauriConfigPath = Join-Path $repoRoot "apps/iclass-gui/src-tauri/tauri.conf.json"

$cargoToml = Get-Content $cargoTomlPath -Raw
$packageJson = Get-Content $packageJsonPath -Raw
$tauriConfig = Get-Content $tauriConfigPath -Raw

$currentVersion = ([System.Text.RegularExpressions.Regex]::Match(
    $cargoToml,
    '(?ms)\[workspace\.package\].*?^version = "([^"]+)"'
)).Groups[1].Value

if (-not $currentVersion) {
    throw "failed to read current workspace version from Cargo.toml"
}

if ($currentVersion -eq $Version) {
    throw "version $Version is already current"
}

git fetch --tags origin | Out-Null
Assert-Success $LASTEXITCODE "failed to fetch tags from origin"

git rev-parse --verify --quiet "refs/tags/$tag" | Out-Null
if ($LASTEXITCODE -eq 0) {
    throw "local tag $tag already exists"
}

$remoteTag = git ls-remote --tags origin "refs/tags/$tag"
Assert-Success $LASTEXITCODE "failed to inspect remote tags"
if ($remoteTag) {
    throw "remote tag $tag already exists"
}

$updatedCargoToml = Replace-Regex `
    -Content $cargoToml `
    -Pattern '(?ms)(\[workspace\.package\].*?^version = ")([^"]+)(")' `
    -Replacement ('$1' + $Version + '$3') `
    -Path "Cargo.toml"

$updatedPackageJson = Replace-Regex `
    -Content $packageJson `
    -Pattern '^(\s*"version":\s*")([^"]+)(",?)' `
    -Replacement ('$1' + $Version + '$3') `
    -Path "apps/iclass-gui/package.json"

$updatedTauriConfig = Replace-Regex `
    -Content $tauriConfig `
    -Pattern '^(\s*"version":\s*")([^"]+)(",?)' `
    -Replacement ('$1' + $Version + '$3') `
    -Path "apps/iclass-gui/src-tauri/tauri.conf.json"

Write-Info "current version: $currentVersion"
Write-Info "next version: $Version"
Write-Info "tag to create: $tag"
Write-Info "files to update:"
$trackedFiles | ForEach-Object { Write-Info "  $_" }

if ($DryRun) {
    Write-Info "dry run requested; no files were changed"
    exit 0
}

Write-Utf8NoBom -Path $cargoTomlPath -Content $updatedCargoToml
Write-Utf8NoBom -Path $packageJsonPath -Content $updatedPackageJson
Write-Utf8NoBom -Path $tauriConfigPath -Content $updatedTauriConfig

git add -- $trackedFiles
Assert-Success $LASTEXITCODE "failed to stage version files"

git commit -m $commitMessage
Assert-Success $LASTEXITCODE "failed to create version bump commit"

git tag -a $tag -m "Release $Version"
Assert-Success $LASTEXITCODE "failed to create annotated tag $tag"

if ($SkipPush) {
    Write-Info "created commit and tag locally; skipping push because -SkipPush was provided"
    exit 0
}

git push origin HEAD
Assert-Success $LASTEXITCODE "failed to push commit to origin"

git push origin $tag
Assert-Success $LASTEXITCODE "failed to push tag $tag to origin"

Write-Info "release prep completed successfully"
