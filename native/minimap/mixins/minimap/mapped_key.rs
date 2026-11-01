// Suppress all game key handling (movement, hotbar, actions, escape) while the Waypoint GUI or World Map is open,
// and suppress the trailing escape event that closed the menu so Horizon doesn't open the pause menu.
fn minimap_mapped_key(
    game: &mut crate::game::Game<'_>,
    key: winit::keyboard::KeyCode,
    pressed: bool,
    repeat: bool,
) -> core::ops::ControlFlow<Result<()>> {
    let _ = (game, pressed, repeat);
    use crate::game::official_minimap as m;
    if key == winit::keyboard::KeyCode::Escape && m::take_consume_escape() {
        return core::ops::ControlFlow::Break(Ok(()));
    }
    if m::is_gui_open() || m::is_world_map_open() {
        return core::ops::ControlFlow::Break(Ok(()));
    }
    core::ops::ControlFlow::Continue(())
}
