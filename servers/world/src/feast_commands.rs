//! Shared routing decision: match commands must bypass the Lua arena-entry command.
pub fn is_match_command(text: &str) -> bool {
    let mut words = text.split_whitespace();
    words.next() == Some("!feast")
        && matches!(
            words.next(),
            Some("status" | "hud" | "restart" | "dummies" | "attack")
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn routes_match_commands_but_keeps_lua_entry_and_exit() {
        for command in [
            "!feast status",
            "!feast hud",
            "!feast   status ",
            "!feast\trestart",
            "!feast dummies 3",
            "!feast attack off",
        ] {
            assert!(is_match_command(command));
        }
        for command in ["!feast", "!feast leave", "!feast unknown", "!whm status"] {
            assert!(!is_match_command(command));
        }
    }
}
