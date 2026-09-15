param([Parameter(Mandatory=$true)][string]$ConfigPath)
$ErrorActionPreference='Stop'
if(Get-Process ffxiv_dx11 -ErrorAction SilentlyContinue){throw 'Close FFXIV before updating its Dalamud settings.'}
$hudDll=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot 'bin/Release/net10.0-windows/Kawari.FeastHud.dll'))
if(!(Test-Path -LiteralPath $hudDll)){throw 'Build the HUD plugin first.'}
$hudConfig=Get-Content -LiteralPath $ConfigPath -Raw | ConvertFrom-Json -AsHashtable
$hudBackup=$ConfigPath+'.before-feast-hud-'+(Get-Date -Format 'yyyyMMdd-HHmmss')
Copy-Item -LiteralPath $ConfigPath -Destination $hudBackup
$hudLocations=$hudConfig.DevPluginLoadLocations
if($hudLocations -is [System.Collections.IDictionary]){$hudEntries=@($hudLocations['$values'])}else{$hudEntries=@($hudLocations)}
$hudEntries=@($hudEntries | Where-Object {$_ -and $_.Path -ne $hudDll})
foreach($hudEntry in $hudEntries){if([IO.Path]::GetFileName($hudEntry.Path) -eq 'Kawari.Feast.dll'){$hudEntry.IsEnabled=$false}}
$hudEntries+=@{Path=$hudDll;IsEnabled=$true;Nickname='Feast HUD'}
if($hudLocations -is [System.Collections.IDictionary]){$hudLocations['$values']=$hudEntries}else{$hudConfig.DevPluginLoadLocations=$hudEntries}
if(!$hudConfig.DevPluginSettings){$hudConfig.DevPluginSettings=@{}}
foreach($hudKey in @($hudConfig.DevPluginSettings.Keys)){if($hudKey -ne '$type' -and [IO.Path]::GetFileName($hudKey) -eq 'Kawari.Feast.dll'){$hudConfig.DevPluginSettings[$hudKey].StartOnBoot=$false}}
$hudConfig.DevPluginSettings[$hudDll]=@{StartOnBoot=$true;AutomaticReloading=$false;NotifyForErrors=$true}
$hudConfig | ConvertTo-Json -Depth 100 | Set-Content -LiteralPath $ConfigPath -Encoding utf8
Write-Output "Registered $hudDll"
Write-Output "Backup: $hudBackup"
