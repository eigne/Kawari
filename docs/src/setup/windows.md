# Windows

This guide covers how to setup Kawari on Windows, which also includes Wine.

> [!NOTE]
> We only provide pre-compiled binaries for x86-64 currently. If you want to run it natively on ARM64, you'll need to compile from source.

## Requirements

* Legally obtained copy of the game that's updated to a supported version
* [Visual C++ Redistributable for Visual Studio 2015](https://www.microsoft.com/en-US/download/details.aspx?id=48145)
* Oodle Network Compression
    * You can download the [latest release of Oodle from this repository](https://github.com/WorkingRobot/OodleUE/releases/latest). Download the "msvc-x64-release.zip" file.

## Downloading Kawari

Download the [latest build for Windows x86_64 from xiv.zone](https://xiv.zone/distrib/kawari/kawari-windows-x86_64.zip) (or pick a specific build from [the CI](https://codeberg.org/redstrate/Kawari/actions).)

## Setup

Place the `oodle-network-shared.dll` from the Oodle zip you downloaded next to the Kawari executables.

## Hosts setup

{{#include hosts.md}}

This file is located under `C:\Windows\System32\Drivers\etc\hosts` on Windows.

Run `ipconfig /flushdns` if you get network issues while trying to launch. This ensures the modifications to your hosts file is actually used. If you get a version check failed message, it means the patch servers are unreachable.

## Running

Run the server by double-clicking `kawari-run.exe`.

## Logging in

{{#include logging_in.md}}
