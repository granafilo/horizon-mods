// Replace whole HUD batch when Waypoint GUI or World Map is open so that
// vanilla game HUD elements (hearts, hunger, armor, air, experience, hotbar)
// are completely suppressed and never drawn over the map.
fn minimap_hud_layers(
    game: &mut crate::game::Game<'_>,
    size: [u32; 2],
) -> core::ops::ControlFlow<HudBatch> {
    use crate::game::official_minimap as m;
    if !game.native_on(m::ID) {
        return core::ops::ControlFlow::Continue(());
    }

    if m::is_gui_open() || m::is_world_map_open() {
        let Some(player) = game.native_player() else {
            return core::ops::ControlFlow::Continue(());
        };

        let mut batch = horizon_client::hud::HudBatch::new(size);

        game.attack_clicks = 0;
        game.keys.clear();
        if let Some((y, p)) = m::get_frozen_look() {
            game.yaw = y;
            game.pitch = p;
        }

        game.captured = true;
        if let Some(w) = &game.window {
            w.set_cursor_visible(true);
            let _ = w.set_cursor_grab(winit::window::CursorGrabMode::None);
        }

        if m::is_world_map_open() {
            m::update_world_map_drag(game.cursor);
            if let Some(quads) = m::take_world_map_quadrants_to_upload(&player.dimension) {
                let (w, h) = (game.gui.atlas.width as f32, game.gui.atlas.height as f32);
                let slots = [250usize, 251, 252, 253];
                for (idx, &slot) in slots.iter().enumerate() {
                    let name = format!("horizon/map{slot}");
                    if let Some(rect) = game.gui.atlas.sprites.get(&name).copied() {
                        game.map_patches.push(horizon_client::font::Patch {
                            x: (rect.u0 * w).round() as u32,
                            y: (rect.v0 * h).round() as u32,
                            width: 128,
                            height: 128,
                            rgba: quads[idx].clone(),
                        });
                    }
                }
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
                m::HudDrawOp::Fill { rect, argb } => game.native_fill(&mut batch, rect, argb),
                m::HudDrawOp::Text {
                    text,
                    at,
                    rgb,
                    shadow,
                } => game.native_text(&mut batch, &text, at, rgb, shadow),
                m::HudDrawOp::MenuBackground { rect } => {
                    horizon_client::ui::menu_background(
                        &mut batch,
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
                        &mut batch,
                        &game.gui.atlas,
                        top,
                        bottom,
                        true,
                    );
                }
                m::HudDrawOp::Separators { top, bottom } => {
                    horizon_client::ui::separators(
                        &mut batch,
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
                        &mut batch,
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
                    let bw = batch.width;
                    horizon_client::ui::centered_text(
                        &mut batch,
                        &game.gui.atlas,
                        &horizon_client::text::Text::literal(&text),
                        0.0,
                        y,
                        bw,
                        color,
                    );
                }
                m::HudDrawOp::MapTexture { rect } => {
                    if let Some(sprite) = game.gui.atlas.sprites.get(horizon_client::hud::MINIMAP_SPRITE).copied() {
                        batch.rect_uv(rect[0], rect[1], rect[2], rect[3], sprite, u32::MAX);
                    }
                }
                m::HudDrawOp::MapTextureGrid { rect, slots } => {
                    let [x, y, w, h] = rect;
                    let hw = w / 2.0;
                    let hh = h / 2.0;
                    let quads = [
                        ([x, y, hw, hh], slots[0]),
                        ([x + hw, y, hw, hh], slots[1]),
                        ([x, y + hh, hw, hh], slots[2]),
                        ([x + hw, y + hh, hw, hh], slots[3]),
                    ];
                    for ([qx, qy, qw, qh], slot) in quads {
                        let name = format!("horizon/map{slot}");
                        if let Some(sprite) = game.gui.atlas.sprites.get(&name).copied() {
                            batch.rect_uv(qx, qy, qw, qh, sprite, u32::MAX);
                        } else if let Some(sprite) = game.gui.atlas.sprites.get(horizon_client::hud::MINIMAP_SPRITE).copied() {
                            batch.rect_uv(qx, qy, qw, qh, sprite, u32::MAX);
                        }
                    }
                }
                m::HudDrawOp::PlayerMarker { center, yaw } => {
                    let (cx, cy) = (center[0], center[1]);
                    if let Some(r) = game.gui.atlas.sprites.get("map/decorations/player").copied() {
                        let (s, c) = yaw.to_radians().sin_cos();
                        let p = |dx: f32, dy: f32| [cx + dx * c - dy * s, cy + dx * s + dy * c];
                        let h = 5.0;
                        batch.quad(
                            [p(-h, h), p(h, h), p(h, -h), p(-h, -h)],
                            [[r.u0, r.v0], [r.u1, r.v0], [r.u1, r.v1], [r.u0, r.v1]],
                            u32::MAX,
                        );
                    } else {
                        game.native_fill(&mut batch, [cx - 3.0, cy - 3.0, 6.0, 6.0], 0xFFFF_3333);
                    }
                }
            }
        }

        return core::ops::ControlFlow::Break(batch);
    }

    core::ops::ControlFlow::Continue(())
}
