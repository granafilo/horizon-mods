// Suppress block attacking, mining, and item usage while the Waypoint GUI is open.
fn minimap_play_mouse(
    game: &mut crate::game::Game<'_>,
    button: winit::event::MouseButton,
    pressed: bool,
) -> core::ops::ControlFlow<Result<()>> {
    let _ = (game, button, pressed);
    use crate::game::official_minimap as m;
    if m::is_gui_open() {
        return core::ops::ControlFlow::Break(Ok(()));
    }
    core::ops::ControlFlow::Continue(())
}
