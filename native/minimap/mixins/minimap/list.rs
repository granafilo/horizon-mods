// The mod's row in Options > Mods.
fn official_minimap_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    use crate::game::official_minimap as m;
    mods.push(crate::game::native_hooks::NativeMod {
        id: m::ID.into(),
        title: format!("{} {}", m::NAME, m::VERSION),
        description: m::DESCRIPTION.into(),
        keys: vec![
            [
                m::KEY_PING.into(),
                "Quick Ping (Raycast)".into(),
                m::DEFAULT_PING_KEY.into(),
            ],
            [
                m::KEY_WAYPOINT.into(),
                "Create Waypoint".into(),
                m::DEFAULT_WAYPOINT_KEY.into(),
            ],
            [
                m::KEY_WAYPOINTS_GUI.into(),
                "Waypoints Menu".into(),
                m::DEFAULT_WAYPOINTS_GUI_KEY.into(),
            ],
        ],
    });
}
