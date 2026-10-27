namespace Kawari.FeastHud;

internal static class NativeAddonIdentity
{
    // AtkUnitBase's inline name follows the vtable pointer. Dalamud can clone
    // that table for lifecycle callbacks, so its address is not an identity.
    internal static bool IsWhiteMageGauge(ReadOnlySpan<byte> header) =>
        header.Length >= 19 && header.Slice(8, 11).SequenceEqual("JobHudWHM0\0"u8);
}
