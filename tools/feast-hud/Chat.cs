using FFXIVClientStructs.FFXIV.Client.System.String;
using FFXIVClientStructs.FFXIV.Client.UI;

namespace Kawari.FeastHud;

internal static unsafe class Chat
{
    // Sends text as if the player typed it, e.g. a `!feast` server command.
    public static void Send(string text)
    {
        var ui = UIModule.Instance();
        if (ui == null) return;
        var message = Utf8String.FromString(text);
        try { ui->ProcessChatBoxEntry(message); }
        finally { message->Dtor(true); }
    }
}
