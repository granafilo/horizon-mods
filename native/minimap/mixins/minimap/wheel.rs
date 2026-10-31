// Handle mouse wheel zoom when the World Map is open.
fn minimap_wheel(game: &crate::game::Game<'_>, steps: f32, claimed: &mut bool) {
    let _ = game;
    use crate::game::official_minimap as m;
    if m::is_world_map_open() {
        if steps > 0.0 {
            m::world_map_zoom_in();
        } else if steps < 0.0 {
            m::world_map_zoom_out();
        }
        *claimed = true;
    }
}
