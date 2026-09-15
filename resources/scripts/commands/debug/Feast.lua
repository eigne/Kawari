-- Territory 745 must be restored on both server and client.
-- See tools/feast-map/README.md for the reversible metadata patch.
required_rank = GM_RANK_DEBUG
command_sender = "[feast] "

local function entry_mode()
    -- The running server may predate duty support while this script is reloaded.
    local ok, mode = pcall(function() return GAME_DATA:feast_entry_mode() end)
    return ok and mode or "exploration"
end

local function can_queue(player)
    -- Town = 0, OpenWorld = 1. Neither is a duty instance.
    return player.zone.id ~= 745 and
        (player.zone.intended_use == 0 or player.zone.intended_use == 1)
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

    if #args == 1 and args[1] == "baseline" then
        if not can_queue(player) then
            printf(player, "Return to an open-world area before starting the Sastasha baseline.")
            return
        end
        -- Current-client CFC 4 resolves to Sastasha / s1d1_re. Use the same
        -- registration and acceptance path as Duty Finder, including JoinContent.
        player:register_for_content(4)
        printf(player, "Sastasha baseline queued. Accept the duty prompt, then use Duty Finder to leave when finished.")
        return
    end

    if #args == 1 and args[1] == "leave" then
        if player.zone.id ~= 745 then
            printf(player, "You are not in the Feast arena.")
            return
        end
        if player.zone.intended_use == 28 then
            player:abandon_content()
        else
            player:change_territory(128, { x = 0.0, y = 40.0, z = 0.0 })
        end
        return
    end

    if #args ~= 0 then
        printf(player, "Usage: !feast to enter, !feast leave to exit, !feast status for medals/time, !feast restart after a result.")
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
