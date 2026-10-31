// native_hooks::Game::native_hud: draw ping and waypoint markers on HUD and minimap.
fn minimap_hud(
    game: &mut crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
    corners: &mut horizon_client::extras_hud::Corners,
) {
    use crate::game::official_minimap as m;
    let _ = corners;
    if !game.native_on(m::ID) || game.debug {
        return;
    }
    let Some(player) = game.native_player() else {
        return;
    };

    if m::is_gui_open() || m::is_world_map_open() {
        batch.vertices.clear();
        batch.blends.clear();
        batch.patches.clear();

        game.captured = true;
        game.creative = true;
        game.attack_clicks = 0;
        game.keys.clear();
        if let Some((y, p)) = m::get_frozen_look() {
            game.yaw = y;
            game.pitch = p;
        }

        if m::is_world_map_open() {
            m::update_world_map_drag(game.cursor);
        }
    }

    let settings = m::settings(|key| game.native_setting(m::ID, key));
    let (cam_x, cam_y, cam_z, fov) = match game.locator_camera {
        Some((pos, _, _, fov)) => (pos[0], pos[1], pos[2], fov),
        None => (player.x, player.y + 1.62, player.z, 70.0),
    };
    let snapshot = m::PlayerSnapshot {
        x: player.x,
        y: player.y,
        z: player.z,
        eye_x: cam_x,
        eye_y: cam_y,
        eye_z: cam_z,
        yaw: player.yaw,
        pitch: player.pitch,
        fov,
        dimension: player.dimension.to_string(),
    };
    let ops = m::draw_hud(
        batch.width,
        batch.height,
        game.cursor,
        &snapshot,
        &settings,
        |text| game.native_text_width(text),
    );
    for op in ops {
        match op {
            m::HudDrawOp::Fill { rect, argb } => game.native_fill(batch, rect, argb),
            m::HudDrawOp::Text {
                text,
                at,
                rgb,
                shadow,
            } => game.native_text(batch, &text, at, rgb, shadow),
            m::HudDrawOp::MenuBackground { rect } => {
                horizon_client::ui::menu_background(
                    batch,
                    &game.gui.atlas,
                    rect[0],
                    rect[1],
                    rect[2],
                    rect[3],
                    true,
                );
            }
            m::HudDrawOp::ListBackground { top, bottom } => {
                horizon_client::ui::list_background(
                    batch,
                    &game.gui.atlas,
                    top,
                    bottom,
                    true,
                );
            }
            m::HudDrawOp::Separators { top, bottom } => {
                horizon_client::ui::separators(
                    batch,
                    &game.gui.atlas,
                    top,
                    bottom,
                    true,
                );
            }
            m::HudDrawOp::Button {
                rect,
                label,
                active,
                hover,
            } => {
                horizon_client::ui::button(
                    batch,
                    &game.gui.atlas,
                    rect[0],
                    rect[1],
                    rect[2],
                    rect[3],
                    &horizon_client::text::Text::literal(&label),
                    active,
                    hover,
                );
            }
            m::HudDrawOp::CenteredText { text, y, color } => {
                horizon_client::ui::centered_text(
                    batch,
                    &game.gui.atlas,
                    &horizon_client::text::Text::literal(&text),
                    0.0,
                    y,
                    batch.width,
                    color,
                );
            }
            m::HudDrawOp::MapTexture { rect } => {
                if let Some(sprite) = game.gui.atlas.sprites.get(horizon_client::hud::MINIMAP_SPRITE).copied() {
                    batch.rect_uv(rect[0], rect[1], rect[2], rect[3], sprite, u32::MAX);
                }
            }
            m::HudDrawOp::PlayerMarker { center, yaw } => {
                let (cx, cy) = (center[0], center[1]);
                if let Some(r) = game.gui.atlas.sprites.get("map/decorations/player").copied() {
                    let (s, c) = (yaw + 180.0).to_radians().sin_cos();
                    let p = |dx: f32, dy: f32| [cx + dx * c - dy * s, cy + dx * s + dy * c];
                    let h = 5.0;
                    batch.quad(
                        [p(-h, h), p(h, h), p(h, -h), p(-h, -h)],
                        [[r.u0, r.v0], [r.u1, r.v0], [r.u1, r.v1], [r.u0, r.v1]],
                        u32::MAX,
                    );
                } else {
                    game.native_fill(batch, [cx - 3.0, cy - 3.0, 6.0, 6.0], 0xFFFF_3333);
                }
            }
        }
    }
}

