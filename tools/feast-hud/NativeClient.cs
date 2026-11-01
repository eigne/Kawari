using System.Diagnostics;
using System.Security.Cryptography;

namespace Kawari.FeastHud;

// Functions ClientStructs does not name, pinned per verified client executable.
// 7.57 addresses were matched from 7.51h2 through ClientStructs' address maps
// (same offsets from AgentPvPHeader, AgentActionDetail and AddonJobHud
// constructors, legacy director vtable shifted with its neighbours) and their
// entry bytes checked in the 7.57 executable.
public sealed record NativeClient(string Version, int PvPHeaderRefresh, int PvPHeaderOpen,
    int LegacyDirectorVtable, int ActionDetailUpdate, int JobHudRequestedUpdate)
{
    private static readonly Dictionary<string, NativeClient> Known = new()
    {
        ["4236E770E673150E85F8D10BEAB2FC4834C82F86AAB8A555A9175439FC906A6D"] =
            new("7.51h2 / 2026.06.18", 0xD5C0A0, 0xD5C670, 0x234E100, 0xEE2530, 0x15A0210),
        ["46A52AB1DBE80B9A8D6C4D4531BF66693904200C5135FACACD053EF28A5DE14A"] =
            new("7.57h1 / 2026.10.16", 0xD62790, 0xD62D60, 0x23780E0, 0xEE93D0, 0x15AB770),
    };
    private static NativeClient? current;

    public static NativeClient? ForHash(string sha256) => Known.GetValueOrDefault(sha256);

    // Hashes the running executable once; unknown builds get no native hooks.
    public static NativeClient Current
    {
        get
        {
            if (current != null) return current;
            using var file = File.OpenRead(Process.GetCurrentProcess().MainModule!.FileName);
            var hash = Convert.ToHexString(SHA256.HashData(file));
            return current = ForHash(hash) ?? throw new InvalidOperationException(
                $"Feast native UI does not know this client executable ({hash[..12]}). Supported: "
                + string.Join(", ", Known.Values.Select(c => c.Version)) + ".");
        }
    }
}
