using System.Text;
using FFXIVClientStructs.FFXIV.Component.GUI;

namespace Kawari.FeastHud;

// In PvP areas the target bars decorate a player's name with Addon 3492,
// «<job>». Feast showed the carried medals there instead (Addon 3493,
// «<num>»), but the current client no longer has that branch. Rewrite only the
// bracketed part of the native text so the name and its payloads stay intact.
internal static unsafe class NativeTargetMedals
{
    private static readonly byte[] Open = Encoding.UTF8.GetBytes("«");
    private static readonly byte[] Close = Encoding.UTF8.GetBytes("»");

    public static void Update(AtkUnitBase* addon, Participant? participant)
    {
        if (addon == null || participant == null) return;
        var job = Encoding.UTF8.GetBytes(participant.JobAbbreviation);
        var medals = Encoding.UTF8.GetBytes(participant.Medals.ToString());
        for (var i = 0; i < addon->UldManager.NodeListCount; i++)
        {
            var node = addon->UldManager.NodeList[i];
            if (node == null || node->Type != NodeType.Text) continue;
            var text = (AtkTextNode*)node;
            var bytes = text->NodeText.AsSpan();
            var start = bytes.IndexOf(Open);
            if (start < 0) continue;
            var inner = start + Open.Length;
            var length = bytes[inner..].IndexOf(Close);
            if (length < 0) continue;
            var current = bytes.Slice(inner, length);
            // Leave other bracketed text (e.g. a Free Company tag) alone.
            if (current.SequenceEqual(medals) || !current.SequenceEqual(job) && !IsNumber(current)) continue;
            var updated = new byte[bytes.Length - length + medals.Length];
            bytes[..inner].CopyTo(updated);
            medals.CopyTo(updated, inner);
            bytes[(inner + length)..].CopyTo(updated.AsSpan(inner + medals.Length));
            text->SetText(updated);
        }
    }

    private static bool IsNumber(ReadOnlySpan<byte> text)
    {
        if (text.IsEmpty) return false;
        foreach (var c in text) if (c is < (byte)'0' or > (byte)'9') return false;
        return true;
    }
}
