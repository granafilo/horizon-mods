// The mod's settings page: the settings and interactive waypoints manager list.
fn official_minimap_page(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    use crate::game::official_minimap as m;
    if mod_id != m::ID {
        return;
    }
    let game = &*game;
    buttons.extend(m::settings_page_buttons(|key| game.native_setting(m::ID, key)));
}
