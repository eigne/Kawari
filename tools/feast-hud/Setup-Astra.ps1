# First-time setup: gives an Astra profile the Dalamud build matching the
# game (copied from XIVLauncher) and registers the Feast HUD as a dev plugin.
# Profile and account IDs are the GUIDs in %LOCALAPPDATA%/State/astrastaterc.
# For an Astra profile that already has a Dalamud config, use Install.ps1.
param(
    [Parameter(Mandatory=$true)][string]$ProfileId,
    [Parameter(Mandatory=$true)][string]$AccountId,
    [string]$DalamudVersion = '15.0.3.9',
    [string]$GameVersion = '2026.10.16.0000.0000'
)
$ErrorActionPreference = 'Stop'
if (Get-Process astra -ErrorAction SilentlyContinue) { throw 'Close Astra before configuring it.' }
$feastDll = Join-Path $PSScriptRoot 'bin/Release/net10.0-windows/Kawari.FeastHud.dll'
if (!(Test-Path -LiteralPath $feastDll)) { throw 'Build the HUD first: dotnet build -c Release' }
$feastSource = Join-Path $env:APPDATA "XIVLauncher/addon/Hooks/$DalamudVersion"
$feastVersion = Get-Content (Join-Path $feastSource 'version.json') -Raw | ConvertFrom-Json
if ($feastVersion.SupportedGameVer -ne $GameVersion) { throw 'Dalamud game version mismatch.' }
$feastState = Join-Path $env:LOCALAPPDATA 'State/astrastaterc'
$feastText = [IO.File]::ReadAllText($feastState)
$feastGroup = "[profile-$ProfileId]"
$feastPattern = '(?ms)^' + [regex]::Escape($feastGroup) + '\r?\n.*?(?=^\[|\z)'
$feastMatch = [regex]::Match($feastText, $feastPattern)
if (!$feastMatch.Success -or $feastMatch.Value -notmatch [regex]::Escape("Account=$AccountId")) {
    throw 'The selected Astra profile/account does not match; no settings changed.'
}
$feastConfigDir = Join-Path $env:LOCALAPPDATA "astra/dalamud/$AccountId"
$feastConfigPath = Join-Path $feastConfigDir 'dalamud-config.json'
if (Test-Path $feastConfigPath) {
    throw 'An Astra Dalamud config already exists. Merge the developer plugin registration manually.'
}
$feastBackup = Join-Path $env:LOCALAPPDATA ('State/astrastaterc.before-feast-' + (Get-Date -Format 'yyyyMMdd-HHmmss'))
Copy-Item -LiteralPath $feastState -Destination $feastBackup
$feastData = Join-Path $env:APPDATA 'astra/dalamud'
New-Item -ItemType Directory -Force (Join-Path $feastData 'local') | Out-Null
Copy-Item "$feastSource/*" (Join-Path $feastData 'local') -Recurse -Force
New-Item -ItemType Directory -Force (Join-Path $feastData 'runtime') | Out-Null
Copy-Item "$env:APPDATA/XIVLauncher/runtime/*" (Join-Path $feastData 'runtime') -Recurse -Force
$feastAssetVersion = (Get-Content "$env:APPDATA/XIVLauncher/dalamudAssets/asset.ver" -Raw).Trim()
New-Item -ItemType Directory -Force (Join-Path $feastData 'assets') | Out-Null
Copy-Item "$env:APPDATA/XIVLauncher/dalamudAssets/$feastAssetVersion/*" (Join-Path $feastData 'assets') -Recurse -Force
Set-Content (Join-Path $feastData 'assets/asset.ver') $feastAssetVersion -NoNewline
# Qt AppConfigLocation on Windows is LocalAppData/astra.
New-Item -ItemType Directory -Force $feastConfigDir | Out-Null
$feastPluginPath = [IO.Path]::GetFullPath($feastDll)
$feastConfig = @{
    DevPluginLoadLocations = @(@{Path=$feastPluginPath; IsEnabled=$true})
    DevPluginSettings = @{}
}
$feastConfig.DevPluginSettings[$feastPluginPath] = @{StartOnBoot=$true; AutomaticReloading=$false; NotifyForErrors=$true}
$feastConfig | ConvertTo-Json -Depth 10 | Set-Content $feastConfigPath -Encoding utf8
$feastUpdatedGroup = $feastMatch.Value
foreach ($feastPair in @(@('DalamudEnabled','true'), @('DalamudChannel','local'))) {
    $feastKey = $feastPair[0]; $feastValue = $feastPair[1]
    $feastUpdatedGroup = [regex]::Replace($feastUpdatedGroup, "(?m)^$feastKey=.*\r?\n?", '')
    $feastUpdatedGroup = $feastUpdatedGroup.TrimEnd() + "`r`n$feastKey=$feastValue`r`n"
}
$feastText = $feastText.Substring(0,$feastMatch.Index) + $feastUpdatedGroup + $feastText.Substring($feastMatch.Index+$feastMatch.Length)
[IO.File]::WriteAllText($feastState,$feastText,[Text.UTF8Encoding]::new($false))
Write-Output "Configured Astra with matching Dalamud $DalamudVersion and the Kawari Feast HUD."
Write-Output "Launcher backup: $feastBackup"
Write-Output "Plugin: $feastPluginPath"
