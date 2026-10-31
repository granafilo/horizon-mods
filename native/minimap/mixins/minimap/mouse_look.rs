// Freeze camera look completely while the Waypoint GUI is open so there is no jitter or twitch.
fn minimap_mouse_look(game: &mut crate::game::Game<'_>) -> core::ops::ControlFlow<()> {
    let _ = game;
    use crate::game::official_minimap as m;
    if m::is_gui_open() {
        return core::ops::ControlFlow::Break(());
    }
    core::ops::ControlFlow::Continue(())
}
