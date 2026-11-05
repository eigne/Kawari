# Kawari Feast HUD

A Dalamud plugin that makes today's FINAL FANTASY XIV client show and use the
Feast, the PvP mode this fork of Kawari restores: the match header, team
lists, job gauges, medals on target bars and nameplates, the historical
actions on the hotbars, and the PvP Actions window. The server decides every
rule; this plugin only presents its state.

Read the developer guide in `docs/src/feast/`, especially
[The HUD plugin](../../docs/src/feast/hud-plugin.md) and
[Setting up](../../docs/src/feast/setup.md).

```powershell
dotnet build -c Release                              # build
dotnet run --project ../feast-hud-tests/Tests.csproj # test
./Install.ps1 -ConfigPath <dalamud-config.json>     # register with an existing Dalamud
./Setup-Astra.ps1 -ProfileId <guid> -AccountId <guid> # or set up Astra from scratch
```

It supports one client build at a time (`Director/NativeClient.cs`) and
disables its native hooks on any other.
