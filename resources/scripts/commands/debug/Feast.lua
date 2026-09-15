-- Territory 745 must be restored on both server and client.
-- See tools/feast-map/README.md for the reversible metadata patch.
required_rank = GM_RANK_DEBUG
command_sender = "[feast] "

function onCommand(player, args, name)
    if #args == 1 and args[1] == "leave" then
        if player.zone.id ~= 745 then
            printf(player, "You are not in the Feast arena.")
            return
        end
        player:change_territory(128, { x = 0.0, y = 40.0, z = 0.0 })
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
    -- Initial staging is inside the west jail. The match assigns the team and
    -- moves the player to its correct jail as soon as the actor is initialized.
    player:change_territory(745, { x = -58.0, y = 0.5, z = 0.0 }, 1.570796)
    printf(player, "Entering Crystal Tower Training Grounds. Match preparation starts automatically. Use !feast status for medals/time, !feast leave to exit.")
end
