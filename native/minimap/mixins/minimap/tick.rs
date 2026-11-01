// Tick: update lifetime of quick pings, save waypoints/world map cache if dirty, and record explored chunks.
fn minimap_tick(game: &mut crate::game::Game<'_>) {
    use crate::game::official_minimap as m;
    if !game.native_on(m::ID) {
        return;
    }
    if m::is_gui_open() || m::is_world_map_open() {
        game.keys.clear();
        game.attack_clicks = 0;
        if let Some((y, p)) = m::get_frozen_look() {
            game.yaw = y;
            game.pitch = p;
        }
    }

    if let Some(player) = game.native_player() {
        let dim = player.dimension.to_string();
        let pcx = (player.x.floor() as i32) >> 4;
        let pcz = (player.z.floor() as i32) >> 4;
        let facts = game.res.facts();
        let air = game.world.air.0;

        let mut scanned = 0;
        for dz in -4..=4 {
            for dx in -4..=4 {
                let key = (pcx + dx, pcz + dz);
                if let Some(chunk) = game.world.chunks.get(&key) {
                    if !m::has_explored_chunk(&dim, key.0, key.1) {
                        let mut color = [0u8; 256];
                        let mut height = [64i16; 256];
                        let mut depth = [0u8; 256];
                        for lz in 0..16 {
                            for lx in 0..16 {
                                let mut y = 255;
                                while y > 0 {
                                    let state = chunk.get(lx, y, lz);
                                    if state != air {
                                        let c = facts.states.get(state as usize).map_or(0, |f| f.map_color);
                                        if c != 0 {
                                            let i = (lz * 16 + lx) as usize;
                                            color[i] = c;
                                            height[i] = y as i16;
                                            break;
                                        }
                                    }
                                    y -= 1;
                                }
                            }
                        }
                        m::record_chunk_columns(&dim, key.0, key.1, color, height, depth);
                        scanned += 1;
                        if scanned >= 4 {
                            break;
                        }
                    }
                }
            }
            if scanned >= 4 {
                break;
            }
        }
    }

    m::tick();
    if m::take_waypoints_dirty() {
        let content = m::save_waypoints_content();
        game.native_set_setting(m::ID, m::KEY_WAYPOINTS, &content);
    }
    m::save_explored_world_if_dirty();
}
