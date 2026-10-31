// The ping / waypoint key went down or up.
fn minimap_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    use crate::game::official_minimap as m;
    if !game.native_on(m::ID) {
        return;
    }
    let ping_key = game.native_mod_key(m::ID, m::KEY_PING, m::DEFAULT_PING_KEY);
    let waypoint_key = game.native_mod_key(m::ID, m::KEY_WAYPOINT, m::DEFAULT_WAYPOINT_KEY);
    let waypoints_gui_key =
        game.native_mod_key(m::ID, m::KEY_WAYPOINTS_GUI, m::DEFAULT_WAYPOINTS_GUI_KEY);

    let playing = game.native_playing();
    let snapshot = game.native_player().map(|p| m::PlayerSnapshot {
        x: p.x,
        y: p.y,
        z: p.z,
        eye_x: p.x,
        eye_y: p.y + 1.62,
        eye_z: p.z,
        yaw: p.yaw,
        pitch: p.pitch,
        fov: 70.0,
        dimension: p.dimension.to_string(),
    });

    if m::is_gui_open() {
        m::on_gui_key(key, pressed, &waypoints_gui_key, snapshot);
        if m::take_waypoints_dirty() {
            let content = m::save_waypoints_content();
            game.native_set_setting(m::ID, m::KEY_WAYPOINTS, &content);
        }
        return;
    }

    if key == waypoints_gui_key {
        if pressed && playing {
            m::toggle_gui(snapshot);
        }
        return;
    }

    if key == ping_key {
        m::on_ping_key(playing, pressed, snapshot);
    } else if key == waypoint_key {
        m::on_waypoint_key(playing, pressed, snapshot);
        if m::take_waypoints_dirty() {
            let content = m::save_waypoints_content();
            game.native_set_setting(m::ID, m::KEY_WAYPOINTS, &content);
        }
    }
}

