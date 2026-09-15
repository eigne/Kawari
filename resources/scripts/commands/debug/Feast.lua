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

    -- The current arena's planmap.lgb contains a PC arrival at (-37, 0, 0).
    -- Start slightly above its floor to allow the client to settle onto collision.
    local position = { x = -37.0, y = 1.0, z = 0.0 }
    if player.zone.id == 745 then
        player:set_position(position, 1.570451)
    else
        player:change_territory(745, position, 1.570451)
    end
    printf(player, "Entering Crystal Tower Training Grounds. Match preparation starts automatically. Use !feast status for medals/time, !feast leave to exit.")
end
