using Dalamud.Game.ClientState.Objects.Types;
using Dalamud.Plugin.Services;
using FFXIVClientStructs.FFXIV.Client.UI;

namespace Kawari.FeastHud;

// June 7.51h2 PvPMKSPartyList's native RequestedUpdate and ReceiveEvent share
// this array contract. Keep real row components, HUD positions and mouse events.
internal static unsafe class NativeRoster
{
    public static void Update(DirectorFrame? frame, IObjectTable objects, ITargetManager targets)
    {
        var ui = RaptureAtkModule.Instance();
        if (ui == null) return;
        var numbers = ui->GetNumberArrayData(117);
        var strings = ui->GetStringArrayData(105);
        if (numbers == null || strings == null || numbers->Size < 602 || strings->Size < 130) return;
        // -1 is the native unassigned-party sentinel. Both teams then use the
        // native actor-ID targeting path instead of regular party-slot selection.
        numbers->SetValue(3, -1);
        numbers->SetValue(8, unchecked((int)(targets.Target?.EntityId ?? 0xE0000000)));
        numbers->SetValue(9, unchecked((int)(targets.SoftTarget?.EntityId ?? 0xE0000000)));
        for (int team = 0; team < 2; team++)
        {
            var players = frame?.Players.Where(p => p.Team == team + 1).Take(4).ToArray() ?? [];
            int teamBase = 10 + team * 296;
            for (int row = 0; row < 5; row++)
            {
                numbers->SetValue(teamBase + 11 + row, row);
                int n = teamBase + 16 + row * 56, s = team * 65 + row * 12;
                var p = row < players.Length ? players[row] : null;
                var actor = p == null ? null : objects.SearchByEntityId(p.Id) as IBattleChara;
                // Never leave a departed/unloaded actor's clickable row behind.
                for (int i = 0; i < 56; i++) numbers->SetValue(n + i, 0);
                numbers->SetValue(n + 1, unchecked((int)0xE0000000));
                if (p == null || actor == null) continue;
                numbers->SetValue(n, 1);
                numbers->SetValue(n + 1, unchecked((int)p.Id));
                numbers->SetValue(n + 2, p.Alive ? 1 : 0);
                numbers->SetValue(n + 3, 1);
                numbers->SetValue(n + 4, 2);
                numbers->SetValue(n + 5, 62100 + p.Job);
                numbers->SetValue(n + 7, (int)actor.CurrentHp);
                numbers->SetValue(n + 8, (int)Math.Max(1u, actor.MaxHp));
                numbers->SetValue(n + 10, (int)actor.CurrentMp);
                numbers->SetValue(n + 11, (int)Math.Max(1u, actor.MaxMp));
                numbers->SetValue(n + 12, p.Adrenaline);
                strings->SetValue(s + 5, actor.Name.TextValue, managed: true);
                strings->SetValue(s + 6, "", managed: true);
                for (int icon = 0; icon < 10; icon++) strings->SetValue(s + 7 + icon, "", managed: true);
                numbers->SetValue(n + 15, p.Actions.Count);
                for (int icon = 0; icon < p.Actions.Count; icon++)
                {
                    var action = p.Actions[icon];
                    numbers->SetValue(n + 36 + icon, action.Icon);
                    numbers->SetValue(n + 46 + icon, action.RemainingTenths);
                    strings->SetValue(s + 7 + icon, action.RemainingTenths == 0 ? "" : ((action.RemainingTenths + 9) / 10).ToString(), managed: true);
                }
            }
        }
    }
}
