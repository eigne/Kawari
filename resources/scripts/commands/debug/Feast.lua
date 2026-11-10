-- !feast: enter the Feast arena, leave it, or travel to the Wolves' Den.
-- Territory 745 and its duty must be restored on both server and client
-- (tools/feast-map). Match commands such as !feast status are handled by the
-- world server instead (servers/world/src/server/feast/commands.rs).
required_rank = GM_RANK_DEBUG
command_sender = "[feast] "
local WOLVES_DEN_AETHERYTE = 55

local function entry_mode()
    -- The running server may predate duty support while this script is reloaded.
    local ok, mode = pcall(function() return GAME_DATA:feast_entry_mode() end)
    return ok and mode or "exploration"
end

local function can_queue(player)
    -- intended_use is serialized by variant name. Neither is a duty instance.
    return player.zone.id ~= 745 and
        (player.zone.intended_use == "Town" or player.zone.intended_use == "Overworld")
end

function onCommand(player, args, name)
    if #args == 1 and args[1] == "duty" then
        if not can_queue(player) then
            printf(player, "Return to an open-world area before queuing the Feast duty.")
            return
        end
        if entry_mode() ~= "duty" then
            printf(player, "Feast duty entry needs the updated server and restored duty metadata.")
            return
        end
        player:register_for_content(476)
        return
    end

    if #args == 1 and args[1] == "leave" then
        if player.zone.id ~= 745 then
            printf(player, "You are not in the Feast arena.")
            return
        end
        if player.zone.intended_use == "CrystallineConflict" then
            player:abandon_content()
        else
            player:change_territory(128, { x = 0.0, y = 40.0, z = 0.0 })
        end
        return
    end

    -- Feast actions and hotbars are set up in the Wolves' Den, as before 6.1.
    if #args == 1 and args[1] == "den" then
        if player.zone.id == 745 then
            printf(player, "Leave the Feast arena first with !feast leave.")
            return
        end
        player:warp_aetheryte(WOLVES_DEN_AETHERYTE, false)
        printf(player, "Travelling to the Wolves' Den. Use /feastactions there to set your PvP actions.")
        return
    end

    if #args ~= 0 then
        printf(player, "Usage: !feast to enter, !feast den for the Wolves' Den, !feast leave to exit, !feast status for medals/time, !feast teams / swap / start for a party's custom match, !feast restart after a result.")
        return
    end

    if player.zone.id == 745 then
        printf(player, "Already in the Feast arena. Use !feast status or !feast restart after a result.")
        return
    end
    local mode = entry_mode()
    if mode == "duty" then
        if not can_queue(player) then
            printf(player, "Return to an open-world area before queuing the Feast duty.")
            return
        end
        player:register_for_content(476)
        return
    elseif mode ~= "exploration" then
        printf(player, "Feast territory metadata is incomplete; entry refused.")
        return
    end
    -- Initial staging is inside the west jail. The match assigns the team and
    -- moves the player to its correct jail as soon as the actor is initialized.
    player:change_territory(745, { x = -58.0, y = 0.5, z = 0.0 }, 1.570796)
    printf(player, "Entering Crystal Tower Training Grounds. Match preparation starts automatically. Use !feast status for medals/time, !feast leave to exit.")
end
