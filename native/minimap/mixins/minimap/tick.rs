// Tick: update lifetime of quick pings and save waypoints if dirty.
fn minimap_tick(game: &mut crate::game::Game<'_>) {
    use crate::game::official_minimap as m;
    if !game.native_on(m::ID) {
        return;
    }
    m::tick();
    if m::take_waypoints_dirty() {
        let content = m::save_waypoints_content();
        game.native_set_setting(m::ID, m::KEY_WAYPOINTS, &content);
    }
}
