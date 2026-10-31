// Override the minimap view when the full-screen World Map is open so that
// Horizon's update_minimap renders the panned map with the selected zoom level.
fn minimap_shown(
    game: &crate::game::Game<'_>,
) -> core::ops::ControlFlow<Option<(Corner, f32, View)>> {
    let _ = game;
    use crate::game::official_minimap as m;
    if m::is_world_map_open() {
        if let Some((pan_x, pan_z, bpp)) = m::get_world_map_view() {
            return core::ops::ControlFlow::Break(Some((
                Corner::TopLeft,
                256.0,
                View {
                    x: pan_x,
                    z: pan_z,
                    yaw: 0.0,
                    rotate: false,
                    blocks_per_pixel: bpp,
                    round: false,
                },
            )));
        }
    }
    core::ops::ControlFlow::Continue(())
}
