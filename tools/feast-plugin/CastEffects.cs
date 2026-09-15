using System.Runtime.InteropServices;
using Dalamud.Hooking;
using Dalamud.Plugin.Services;
using FFXIVClientStructs.FFXIV.Client.Graphics.Scene;

namespace Kawari.Feast;

/// Owns only effects created for the local Feast preparation phase.
internal sealed unsafe class CastEffects : IDisposable
{
    // Native API documented by Dalamud-VFXEditor ResourceLoader.Vfx.cs/Constants.cs.
    // Both signatures were checked to match once in the supported 7.51h2 executable.
    private const string CreateSignature = "40 53 55 56 57 48 81 EC ?? ?? ?? ?? 0F 29 B4 24 ?? ?? ?? ?? 48 8B 05 ?? ?? ?? ?? 48 33 C4 48 89 84 24 ?? ?? ?? ?? 0F B6 AC 24 ?? ?? ?? ?? 0F 28 F3 49 8B F8";
    private const string RemoveSignature = "0F 11 48 10 48 8D 05";
    private delegate nint CreateDelegate([MarshalAs(UnmanagedType.LPStr)] string path,
        nint caster, nint target, float duration, byte a5, ushort a6, byte a7);
    private delegate nint RemoveDelegate(nint effect, byte immediate);
    private readonly CreateDelegate create;
    private readonly Hook<RemoveDelegate> remove;
    private readonly HashSet<nint> owned = [];
    private readonly Dictionary<nint, long> fading = [];
    private const float CompletionFadeSeconds = 0.25f;
    private readonly object gate = new();
    private readonly IPluginLog log;

    public CastEffects(ISigScanner scanner, IGameInteropProvider interop, IPluginLog log)
    {
        this.log = log;
        create = Marshal.GetDelegateForFunctionPointer<CreateDelegate>(scanner.ScanText(CreateSignature));
        // LEA locates the vtable; its first pointer is the actor-VFX removal function.
        var displacement = scanner.ScanText(RemoveSignature) + 7;
        var table = displacement + Marshal.ReadInt32(displacement) + 4;
        var address = Marshal.ReadIntPtr(table);
        remove = interop.HookFromAddress<RemoveDelegate>(address, Removed);
        remove.Enable();
    }

    public void Start(nint actor, IEnumerable<string> keys)
    {
        foreach (var key in keys.Where(key => !string.IsNullOrWhiteSpace(key)).Distinct())
        {
            var path = $"vfx/common/eff/{key}.avfx";
            lock (gate)
            {
                var effect = create(path, actor, actor, -1, 0, 0, 0);
                if (effect != 0) owned.Add(effect);
                log.Information("Feast casting VFX {Path}: created={Created}", path, effect != 0);
            }
        }
    }

    private nint Removed(nint effect, byte immediate)
    {
        lock (gate)
        {
            // The game may destroy effects itself on actor removal or natural expiry.
            // Forget those handles before forwarding, so later cleanup cannot double-free.
            owned.Remove(effect);
            fading.Remove(effect);
            return remove.Original(effect, immediate);
        }
    }

    public void Complete()
    {
        lock (gate)
        {
            foreach (var effect in owned.ToArray())
            {
                owned.Remove(effect);
                // Native VfxObject fade units are 1/60 second, independent of FPS.
                // Keep tracking until native removal or the fallback cleanup deadline.
                fading[effect] = Environment.TickCount64 + 750;
                var vfx = (VfxObject*)effect;
                vfx->FadeOutFrames = CompletionFadeSeconds * 60;
                vfx->SomeFlags |= 0x40;
            }
            log.Information("Feast casting VFX fading into release over {Seconds}s", CompletionFadeSeconds);
        }
    }

    public void Update()
    {
        lock (gate)
        {
            var now = Environment.TickCount64;
            foreach (var (effect, deadline) in fading.ToArray())
                if (now >= deadline && fading.Remove(effect)) remove.Original(effect, 1);
        }
    }

    public void Stop()
    {
        lock (gate)
        {
            foreach (var effect in owned.ToArray())
            {
                if (owned.Remove(effect)) remove.Original(effect, 1);
            }
        }
    }

    public void Dispose()
    {
        Stop();
        lock (gate)
        {
            foreach (var effect in fading.Keys.ToArray())
                if (fading.Remove(effect)) remove.Original(effect, 1);
        }
        remove.Dispose();
    }
}
