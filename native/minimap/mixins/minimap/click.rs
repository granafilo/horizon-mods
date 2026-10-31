// A click on the mod's settings page: change settings or manage waypoints.
fn official_minimap_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    use crate::game::official_minimap as m;
    if mod_id != m::ID {
        return;
    }
    m::on_settings_click(button, |key| game.native_setting(m::ID, key));
    let settings = m::settings(|key| game.native_setting(m::ID, key));
    for (key, value) in settings.stored() {
        game.native_set_setting(m::ID, key, &value);
    }
    if m::take_waypoints_dirty() {
        let content = m::save_waypoints_content();
        game.native_set_setting(m::ID, m::KEY_WAYPOINTS, &content);
    }
}
