// The ping / waypoint key went down or up.
fn minimap_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    use crate::game::official_minimap as m;
    if !game.native_on(m::ID) {
        return;
    }
    let ping_key = game.native_mod_key(m::ID, m::KEY_PING, m::DEFAULT_PING_KEY);
    let waypoint_key = game.native_mod_key(m::ID, m::KEY_WAYPOINT, m::DEFAULT_WAYPOINT_KEY);
    let waypoints_gui_key =
        game.native_mod_key(m::ID, m::KEY_WAYPOINTS_GUI, m::DEFAULT_WAYPOINTS_GUI_KEY);
    let world_map_key =
        game.native_mod_key(m::ID, m::KEY_WORLD_MAP, m::DEFAULT_WORLD_MAP_KEY);

    let playing = game.native_playing();
    let snapshot = game.native_player().map(|p| m::PlayerSnapshot {
        x: p.x,
        y: p.y,
        z: p.z,
        eye_x: p.x,
        eye_y: p.y + 1.62,
        eye_z: p.z,
        yaw: p.yaw,
        pitch: p.pitch,
        fov: 70.0,
        dimension: p.dimension.to_string(),
    });

    if m::is_world_map_open() {
        if pressed {
            if key == "key.keyboard.escape" || key == world_map_key {
                let saved_look = m::get_frozen_look();
                let was_c = m::was_creative();
                m::close_world_map();
                game.creative = was_c;
                if let Some((saved_yaw, saved_pitch)) = saved_look {
                    game.yaw = saved_yaw;
                    game.pitch = saved_pitch;
                }
                game.mouse = Default::default();
                if key == "key.keyboard.escape" {
                    game.screen = crate::game::Screen::Paused;
                    game.pause_menu = false;
                } else {
                    game.screen = crate::game::Screen::Playing;
                    game.capture(true);
                }
                game.keys.clear();
            } else if key == "key.mouse.left" {
                let (cx, cy) = game.cursor;
                let action = m::on_world_map_click(cx, cy, snapshot.clone());
                match action {
                    m::WorldMapClickAction::Close => {
                        let saved_look = m::get_frozen_look();
                        let was_c = m::was_creative();
                        m::close_world_map();
                        game.creative = was_c;
                        if let Some((saved_yaw, saved_pitch)) = saved_look {
                            game.yaw = saved_yaw;
                            game.pitch = saved_pitch;
                        }
                        game.mouse = Default::default();
                        game.screen = crate::game::Screen::Playing;
                        game.capture(true);
                        game.keys.clear();
                    }
                    m::WorldMapClickAction::OpenWaypointsGui => {
                        m::toggle_gui(snapshot.clone(), game.creative, game.yaw, game.pitch);
                        game.attack_clicks = 0;
                        game.keys.clear();
                    }
                    m::WorldMapClickAction::None => {
                        game.attack_clicks = 0;
                        game.keys.clear();
                    }
                }
            } else if key == "key.mouse.right" {
                let (cx, cy) = game.cursor;
                let created = m::on_world_map_right_click(cx, cy, snapshot.clone());
                if created && m::take_waypoints_dirty() {
                    let content = m::save_waypoints_content();
                    game.native_set_setting(m::ID, m::KEY_WAYPOINTS, &content);
                }
                game.attack_clicks = 0;
                game.keys.clear();
            } else {
                let action = m::on_world_map_key(key, true, &world_map_key, &waypoints_gui_key, snapshot.clone());
                match action {
                    m::WorldMapKeyAction::Close => {
                        let saved_look = m::get_frozen_look();
                        let was_c = m::was_creative();
                        m::close_world_map();
                        game.creative = was_c;
                        if let Some((saved_yaw, saved_pitch)) = saved_look {
                            game.yaw = saved_yaw;
                            game.pitch = saved_pitch;
                        }
                        game.mouse = Default::default();
                        game.screen = crate::game::Screen::Playing;
                        game.capture(true);
                        game.keys.clear();
                    }
                    m::WorldMapKeyAction::OpenWaypointsGui => {
                        m::toggle_gui(snapshot.clone(), game.creative, game.yaw, game.pitch);
                        game.attack_clicks = 0;
                        game.keys.clear();
                    }
                    m::WorldMapKeyAction::None => {}
                }
            }
        } else if key == "key.mouse.left" {
            m::end_world_map_drag();
        }
        return;
    }

    if m::is_gui_open() {
        if pressed {
            if key == "key.keyboard.escape" || key == waypoints_gui_key {
                let saved_look = m::get_frozen_look();
                let was_c = m::was_creative();
                m::close_gui();
                game.creative = was_c;
                if let Some((saved_yaw, saved_pitch)) = saved_look {
                    game.yaw = saved_yaw;
                    game.pitch = saved_pitch;
                }
                game.mouse = Default::default();
                if key == "key.keyboard.escape" {
                    // Set to Paused so Horizon's mapped_key(Escape) will execute the
                    // pause-closing branch which transitions cleanly back to
                    // Screen::Playing with capture(true), without opening the pause menu!
                    game.screen = crate::game::Screen::Paused;
                    game.pause_menu = false;
                } else {
                    game.screen = crate::game::Screen::Playing;
                    game.capture(true);
                }
                game.keys.clear();
            } else if key == "key.mouse.left" {
                let (cx, cy) = game.cursor;
                let saved_look = m::get_frozen_look();
                let closed = m::on_gui_click(cx, cy, snapshot.clone());
                if closed {
                    let was_c = m::was_creative();
                    game.creative = was_c;
                    if let Some((saved_yaw, saved_pitch)) = saved_look {
                        game.yaw = saved_yaw;
                        game.pitch = saved_pitch;
                    }
                    game.mouse = Default::default();
                    game.screen = crate::game::Screen::Playing;
                    game.capture(true);
                    game.keys.clear();
                } else {
                    game.attack_clicks = 0;
                    game.keys.clear();
                }
            } else {
                m::on_gui_key(key, true, &waypoints_gui_key, snapshot.clone());
            }
            if m::take_waypoints_dirty() {
                let content = m::save_waypoints_content();
                game.native_set_setting(m::ID, m::KEY_WAYPOINTS, &content);
            }
        }
        return;
    }

    if key == world_map_key {
        if pressed && playing {
            let was_open = m::is_world_map_open();
            if was_open {
                let saved_look = m::get_frozen_look();
                let was_c = m::was_creative();
                m::close_world_map();
                game.creative = was_c;
                if let Some((saved_yaw, saved_pitch)) = saved_look {
                    game.yaw = saved_yaw;
                    game.pitch = saved_pitch;
                }
                game.mouse = Default::default();
                game.screen = crate::game::Screen::Playing;
                game.capture(true);
                game.keys.clear();
            } else {
                let opened = m::toggle_world_map(snapshot.clone(), game.creative, game.yaw, game.pitch);
                if opened {
                    // Release OS cursor grab and show mouse cursor
                    game.capture(false);
                    // Mark captured = true so game.rs forwards mouse clicks to extras_mouse
                    game.captured = true;
                    game.creative = true;
                    game.keys.clear();
                }
            }
        }
        return;
    }

    if key == waypoints_gui_key {
        if pressed && playing {
            let was_open = m::is_gui_open();
            if was_open {
                let saved_look = m::get_frozen_look();
                let was_c = m::was_creative();
                m::close_gui();
                game.creative = was_c;
                if let Some((saved_yaw, saved_pitch)) = saved_look {
                    game.yaw = saved_yaw;
                    game.pitch = saved_pitch;
                }
                game.mouse = Default::default();
                game.screen = crate::game::Screen::Playing;
                game.capture(true);
                game.keys.clear();
            } else {
                let opened = m::toggle_gui(snapshot.clone(), game.creative, game.yaw, game.pitch);
                if opened {
                    // Release OS cursor grab and show mouse cursor
                    game.capture(false);
                    // Mark captured = true so game.rs forwards mouse clicks to extras_mouse
                    game.captured = true;
                    game.creative = true;
                    game.keys.clear();
                }
            }
        }
        return;
    }

    if key == ping_key {
        m::on_ping_key(playing, pressed, snapshot.clone());
    } else if key == waypoint_key {
        m::on_waypoint_key(playing, pressed, snapshot);
        if m::take_waypoints_dirty() {
            let content = m::save_waypoints_content();
            game.native_set_setting(m::ID, m::KEY_WAYPOINTS, &content);
        }
    }
}

