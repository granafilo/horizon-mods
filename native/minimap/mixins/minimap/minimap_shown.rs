// Hide HUD minimap while the full-screen World Map is open to avoid wasting CPU recomposing it.
fn minimap_shown(
    game: &crate::game::Game<'_>,
) -> core::ops::ControlFlow<Option<(Corner, f32, View)>> {
    let _ = game;
    use crate::game::official_minimap as m;
    if m::is_world_map_open() {
        return core::ops::ControlFlow::Break(None);
    }
    core::ops::ControlFlow::Continue(())
}
