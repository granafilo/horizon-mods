// Freeze camera look completely while the Waypoint GUI or World Map is open so there is no jitter or twitch,
// and prevent camera jerk when closing the GUI by discarding accumulated mouse movement.
fn minimap_mouse_look(game: &mut crate::game::Game<'_>) -> core::ops::ControlFlow<()> {
    use crate::game::official_minimap as m;
    if m::is_gui_open() || m::is_world_map_open() {
        game.mouse = Default::default();
        if let Some((saved_yaw, saved_pitch)) = m::get_frozen_look() {
            game.yaw = saved_yaw;
            game.pitch = saved_pitch;
        }
        return core::ops::ControlFlow::Break(());
    }

    if let Some((saved_yaw, saved_pitch)) = m::take_suppress_mouse_look() {
        game.mouse = Default::default();
        game.yaw = saved_yaw;
        game.pitch = saved_pitch;
        return core::ops::ControlFlow::Break(());
    }

    core::ops::ControlFlow::Continue(())
}
