BASE_DIR = "resources/scripts/"

dofile(BASE_DIR.."commands/Commands.lua")
dofile(BASE_DIR.."items/Items.lua")

-- Use the shared implementation directly: Git may check out the 870 symlink
-- as a plain text filename on Windows.
registerAction(870, BASE_DIR.."actions/000/EnemyAutoAttack_00872.lua")

function onCommandRequiredRankInsufficientError(player)
    player:send_message("You do not have permission to run this command.")
end

function onCommandRequiredRankMissingError(additional_information, player)
    local error_msg = "Your script does not define the required_rank variable. Please define it in your script for it to run."
    printf(player, "%s\nAdditional information: %s", error_msg, additional_information)
end
