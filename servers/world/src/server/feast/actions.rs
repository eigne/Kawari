//! The historical Feast action catalog, `resources/feast-actions.json`.
//!
//! Every action a Feast job can use keeps its original 6.08 Action sheet row
//! ID. Those rows still exist in today's client (with their animations, VFX
//! and sounds) even though their names and icons were blanked, so the client
//! can cast them and play them as long as the server and HUD agree on the IDs.
//! The same JSON file is embedded in the HUD plugin for icons and tooltips.
use serde::Deserialize;

/// The Action sheet's cooldown group for the global cooldown, zero-based as
/// the client's recast packets expect (the sheet itself says 58).
pub const GCD_GROUP: u32 = 57;

#[derive(Debug, Deserialize)]
pub struct JobAction {
    /// Historical Action sheet row.
    pub id: u32,
    /// Name used by the Lua engine (`actions[key]`) and in commands.
    pub key: String,
    /// Zero-based cooldown group, or -1 for none.
    pub group: i32,
    /// Zero-based additional cooldown group, or -1 for none. Weaponskills
    /// with their own recast (Drill, Air Anchor) also use the GCD here.
    pub secondary: i32,
}

pub fn all() -> &'static [JobAction] {
    static ACTIONS: std::sync::LazyLock<Vec<JobAction>> = std::sync::LazyLock::new(|| {
        serde_json::from_str(include_str!("../../../../../resources/feast-actions.json"))
            .expect("resources/feast-actions.json must parse")
    });
    &ACTIONS
}

/// Looks up an action by its historical Action sheet row.
pub fn by_id(id: u32) -> Option<&'static JobAction> {
    all().iter().find(|a| a.id == id)
}

/// Looks up an action by its Lua key, e.g. `"glare"`.
pub fn by_key(key: &str) -> Option<&'static JobAction> {
    all().iter().find(|a| a.key == key)
}

/// The action's recast group for `SetCooldownTimer`, if it has one.
pub fn cooldown_group(key: &str) -> Option<u32> {
    by_key(key).and_then(|a| u32::try_from(a.group).ok())
}
