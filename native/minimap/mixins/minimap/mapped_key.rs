// Suppress all game key handling (movement, hotbar, actions) while the Waypoint GUI is open.
fn minimap_mapped_key(
    game: &mut crate::game::Game<'_>,
    key: winit::keyboard::KeyCode,
    pressed: bool,
    repeat: bool,
) -> core::ops::ControlFlow<Result<()>> {
    let _ = (game, pressed, repeat);
    use crate::game::official_minimap as m;
    if m::is_gui_open() && key != winit::keyboard::KeyCode::Escape {
        return core::ops::ControlFlow::Break(Ok(()));
    }
    core::ops::ControlFlow::Continue(())
}
