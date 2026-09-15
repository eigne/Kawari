param(
    [switch]$Restore,
    [string]$Python
)

$ErrorActionPreference = 'Stop'
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$patchDirectory = Join-Path $repositoryRoot 'backups\feast-map-7.51h2'
$toolPath = Join-Path $PSScriptRoot 'feast_map.py'

if (-not $Python) {
    $bundledPython = Join-Path $env:USERPROFILE '.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe'
    if (Test-Path -LiteralPath $bundledPython) {
        $Python = $bundledPython
    } else {
        $Python = (Get-Command python -ErrorAction Stop).Source
    }
}
$Python = (Resolve-Path -LiteralPath $Python).Path
if (-not (Test-Path -LiteralPath (Join-Path $patchDirectory 'patch.json'))) {
    throw 'Prepare the patch first using feast_map.py prepare. See README.md.'
}

$principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    # Paths cannot contain a double quote on Windows. Quote them for CreateProcess.
    $launchArguments = "-NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`" -Python `"$Python`""
    if ($Restore) { $launchArguments += ' -Restore' }
    $elevated = Start-Process -FilePath 'powershell.exe' -Verb RunAs -WindowStyle Hidden `
        -ArgumentList $launchArguments -PassThru -Wait
    if ($elevated.ExitCode -ne 0) {
        throw "Patch operation failed. See $patchDirectory\install.log"
    }
    return
}

$operation = if ($Restore) { 'restore' } else { 'install' }
# Python rechecks the version, index, source bytes, backup hashes, and running processes.
& $Python $toolPath $operation --output $patchDirectory 2>&1 |
    Out-File -LiteralPath (Join-Path $patchDirectory 'install.log') -Encoding utf8
exit $LASTEXITCODE
