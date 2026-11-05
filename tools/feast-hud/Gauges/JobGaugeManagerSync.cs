using FFXIVClientStructs.FFXIV.Client.Game.Gauge;
using FFXIVClientStructs.FFXIV.Client.Game;

namespace Kawari.FeastHud;

// Copies the server's job gauge (director record F7) into the client's
// JobGaugeManager. The client reads these values for its own action checks
// and hotbar highlights; the gauge widgets themselves are fed separately by
// NativeResourceGauges. The byte meanings per job come from M.job_gauge in
// the Lua engine (Jobs.lua and AdditionalJobs.lua).
internal static unsafe class JobGaugeManagerSync
{
    public static void Apply(JobGauge gauge)
    {
        var manager = JobGaugeManager.Instance();
        if (manager == null || manager->ClassJobId != gauge.Job) return;
        switch (gauge.Job)
        {
            case 19: manager->Paladin.OathGauge = gauge.Resource; break;
            case 20: manager->Monk.Chakra = gauge.Resource; break;
            case 21: manager->Warrior.BeastGauge = gauge.Resource; break;
            case 22:
                manager->Dragoon.EyeCount = gauge.Resource;
                manager->Dragoon.LotdTimer = (short)gauge.Timer;
                manager->Dragoon.LotdState = gauge.Element;
                break;
            case 23:
                manager->Bard.SoulVoice = gauge.Resource;
                manager->Bard.Repertoire = gauge.Meditation;
                manager->Bard.SongTimer = gauge.Timer;
                manager->Bard.SongFlags = gauge.Element == 1 ? SongFlags.WanderersMinuet
                    : gauge.Element == 2 ? SongFlags.ArmysPaeon : 0;
                break;
            case 24:
                manager->WhiteMage.Lily = gauge.Resource;
                manager->WhiteMage.BloodLily = gauge.Meditation;
                manager->WhiteMage.LilyTimer = (short)gauge.Timer;
                break;
            case 25:
                manager->BlackMage.ElementStance = (sbyte)(gauge.Element == 1 ? 1 : gauge.Element == 2 ? -1 : 0);
                manager->BlackMage.PolyglotStacks = gauge.Polyglot;
                manager->BlackMage.EnochianTimer = (short)gauge.Timer;
                manager->BlackMage.EnochianFlags = gauge.Enochian ? EnochianFlags.Enochian : 0;
                break;
            case 27:
                // Retail stores one bit per stack; the director sends a count.
                manager->Summoner.AetherFlags = (AetherFlags)((1 << gauge.Resource) - 1);
                manager->Summoner.SummonTimer = gauge.Timer;
                break;
            case 28: manager->Scholar.FairyGauge = gauge.Resource; break;
            case 30: manager->Ninja.Ninki = gauge.Resource; break;
            case 31:
                manager->Machinist.Heat = gauge.Resource;
                manager->Machinist.OverheatTimeRemaining = (short)gauge.Timer;
                manager->Machinist.TimerActive = gauge.Element;
                break;
            case 32:
                manager->DarkKnight.Blood = gauge.Resource;
                manager->DarkKnight.DarksideTimer = gauge.Timer;
                manager->DarkKnight.DarkArtsState = gauge.Element;
                break;
            // CurrentArcana reads the upper nibble of Cards in this SDK.
            case 33: manager->Astrologian.Cards = (short)(gauge.Resource << 12); break;
            case 34: manager->Samurai.Kenki = gauge.Resource; manager->Samurai.MeditationStacks = gauge.Meditation; break;
            case 35: manager->RedMage.BlackMana = gauge.Resource; manager->RedMage.WhiteMana = gauge.Meditation; break;
            case 37: manager->Gunbreaker.Ammo = gauge.Resource; break;
            case 38:
                manager->Dancer.Esprit = gauge.Resource;
                manager->Dancer.Feathers = gauge.Meditation;
                manager->Dancer.StepIndex = gauge.Polyglot;
                for (int i = 0; i < 4; i++) manager->Dancer.DanceSteps[i] = (byte)((gauge.Timer >> (4 * i)) & 15);
                break;
            case 39:
                manager->Reaper.Soul = gauge.Resource;
                manager->Reaper.Shroud = gauge.Meditation;
                manager->Reaper.LemureShroud = gauge.Polyglot;
                manager->Reaper.VoidShroud = gauge.Element;
                manager->Reaper.EnshroudedTimeRemaining = gauge.Timer;
                break;
            case 40:
                manager->Sage.Addersgall = gauge.Resource;
                manager->Sage.AddersgallTimer = (short)gauge.Timer;
                manager->Sage.Eukrasia = gauge.Element;
                break;
        }
    }
}
