// Horizon's official Minimap, extended with Quick Ping and Waypoint system (Xaero's Style).
use std::cell::RefCell;

/// The mod's id (its settings folder), name and version.
pub const ID: &str = "minimap";
pub const NAME: &str = "Minimap";
pub const VERSION: &str = "2.1.0";
pub const DESCRIPTION: &str =
    "A map of the terrain with Xaero's style Quick Ping and Waypoints.";

/// Key bind identifiers and defaults.
pub const KEY_PING: &str = "ping";
pub const DEFAULT_PING_KEY: &str = "key.keyboard.v";
pub const KEY_WAYPOINT: &str = "waypoint";
pub const DEFAULT_WAYPOINT_KEY: &str = "key.keyboard.b";
pub const KEY_WAYPOINTS_GUI: &str = "waypoints_gui";
pub const DEFAULT_WAYPOINTS_GUI_KEY: &str = "key.keyboard.u";
pub const KEY_WORLD_MAP: &str = "world_map";
pub const DEFAULT_WORLD_MAP_KEY: &str = "key.keyboard.m";
pub const WORLD_MAP_ZOOMS: [f32; 4] = [0.5, 1.0, 2.0, 4.0];

/// Quick ping lifetime in ticks (20 ticks = 1 sec -> 200 ticks = 10 sec).
pub const PING_LIFETIME_TICKS: u32 = 200;
/// Maximum raycast reach in blocks.
pub const RAYCAST_MAX_REACH: f64 = 128.0;

/// The corners, as the settings files name them and the page labels them.
pub const CORNERS: [(&str, &str); 4] = [
    ("top_left", "Top left"),
    ("top_right", "Top right"),
    ("bottom_left", "Bottom left"),
    ("bottom_right", "Bottom right"),
];
/// Sides in GUI pixels, and blocks per map pixel.
pub const SIZES: [(u8, &str); 3] = [(64, "Small"), (96, "Medium"), (128, "Large")];
pub const ZOOMS: [f32; 4] = [0.5, 1.0, 2.0, 4.0];

/// Waypoint data structure.
#[derive(Clone, Debug, PartialEq)]
pub struct Waypoint {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub dimension: String,
    pub name: String,
    pub rgb: u32,
    pub enabled: bool,
}

pub const KEY_WAYPOINTS: &str = "waypoints";

impl Waypoint {
    pub fn distance_to(&self, px: f64, py: f64, pz: f64) -> f64 {
        let dx = self.x - px;
        let dy = self.y - py;
        let dz = self.z - pz;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Serializes a waypoint to a TSV string: name\tx\ty\tz\tdimension\trgb_hex\tenabled
    pub fn serialize(&self) -> String {
        format!(
            "{}\t{:.2}\t{:.2}\t{:.2}\t{}\t{:06X}\t{}",
            self.name.replace('\t', " "),
            self.x,
            self.y,
            self.z,
            self.dimension,
            self.rgb & 0x00FF_FFFF,
            self.enabled
        )
    }

    /// Deserializes a waypoint from a TSV line.
    pub fn deserialize(line: &str) -> Option<Self> {
        let parts: Vec<&str> = line.trim().split('\t').collect();
        if parts.len() < 7 {
            return None;
        }
        let name = parts[0].to_owned();
        let x = parts[1].parse::<f64>().ok()?;
        let y = parts[2].parse::<f64>().ok()?;
        let z = parts[3].parse::<f64>().ok()?;
        let dimension = parts[4].to_owned();
        let rgb = u32::from_str_radix(parts[5], 16).ok()?;
        let enabled = parts[6].eq_ignore_ascii_case("true");
        Some(Self {
            x,
            y,
            z,
            dimension,
            name,
            rgb,
            enabled,
        })
    }
}

/// Ping data structure (temporary marker with fade-out).
#[derive(Clone, Debug, PartialEq)]
pub struct Ping {
    pub waypoint: Waypoint,
    pub remaining_ticks: u32,
    pub initial_ticks: u32,
}

impl Ping {
    pub fn new(x: f64, y: f64, z: f64, dimension: String, name: String, rgb: u32, ticks: u32) -> Self {
        Self {
            waypoint: Waypoint {
                x,
                y,
                z,
                dimension,
                name,
                rgb,
                enabled: true,
            },
            remaining_ticks: ticks,
            initial_ticks: ticks,
        }
    }

    pub fn alpha(&self) -> f32 {
        if self.initial_ticks == 0 {
            0.0
        } else {
            (self.remaining_ticks as f32 / self.initial_ticks as f32).clamp(0.0, 1.0)
        }
    }
}

/// The settings, as the WebAssembly Minimap kept them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    /// Indices in `CORNERS`, `SIZES` and `ZOOMS`.
    pub corner: usize,
    pub size: usize,
    pub round: bool,
    pub zoom: usize,
    pub rotate: bool,
    pub show_hud_pings: bool,
    pub show_minimap_pings: bool,
    pub show_coords: bool,
    pub show_beacons: bool,
}

impl Settings {
    /// From the stored values; the defaults for the ones missing.
    pub fn read(get: impl Fn(&str) -> Option<String>) -> Self {
        let get = |key: &str| get(key).map(|v| v.trim().to_owned());
        let find = |key: &str, values: &[String], default: usize| {
            get(key)
                .and_then(|v| values.iter().position(|s| *s == v))
                .unwrap_or(default)
        };
        let corners: Vec<String> = CORNERS.iter().map(|c| c.0.to_owned()).collect();
        let sizes: Vec<String> = SIZES.iter().map(|s| s.0.to_string()).collect();
        let zooms: Vec<String> = ZOOMS.iter().map(f32::to_string).collect();
        Self {
            corner: find("corner", &corners, 1),
            size: find("size", &sizes, 1),
            round: get("shape").as_deref() == Some("round"),
            zoom: find("zoom", &zooms, 1),
            rotate: get("rotate").as_deref() == Some("true"),
            show_hud_pings: get("hud_pings").as_deref() != Some("false"),
            show_minimap_pings: get("minimap_pings").as_deref() != Some("false"),
            show_coords: get("show_coords").as_deref() != Some("false"),
            show_beacons: get("show_beacons").as_deref() != Some("false"),
        }
    }

    pub fn labels(&self) -> Vec<String> {
        let n = ZOOMS[self.zoom];
        let blocks = if n < 1.0 {
            "½ block".to_owned()
        } else if n == 1.0 {
            "1 block".to_owned()
        } else {
            format!("{n} blocks")
        };
        vec![
            format!("Corner: {}", CORNERS[self.corner].1),
            format!("Size: {}", SIZES[self.size].1),
            format!("Shape: {}", if self.round { "Round" } else { "Square" }),
            format!("Zoom: {blocks} per pixel"),
            format!(
                "Turn with the player: {}",
                if self.rotate { "ON" } else { "OFF" }
            ),
            format!(
                "In-World HUD Pins: {}",
                if self.show_hud_pings { "ON" } else { "OFF" }
            ),
            format!(
                "Minimap Pins: {}",
                if self.show_minimap_pings { "ON" } else { "OFF" }
            ),
            format!(
                "Coordinates & Facing: {}",
                if self.show_coords { "ON" } else { "OFF" }
            ),
            format!(
                "Waypoint Beacons: {}",
                if self.show_beacons { "ON" } else { "OFF" }
            ),
        ]
    }

    pub fn click(&mut self, i: usize) {
        let next = |i: usize, n: usize| (i + 1) % n;
        match i {
            0 => self.corner = next(self.corner, CORNERS.len()),
            1 => self.size = next(self.size, SIZES.len()),
            2 => self.round = !self.round,
            3 => self.zoom = next(self.zoom, ZOOMS.len()),
            4 => self.rotate = !self.rotate,
            5 => self.show_hud_pings = !self.show_hud_pings,
            6 => self.show_minimap_pings = !self.show_minimap_pings,
            7 => self.show_coords = !self.show_coords,
            8 => self.show_beacons = !self.show_beacons,
            _ => {}
        }
    }

    pub fn stored(&self) -> [(&'static str, String); 9] {
        [
            ("corner", CORNERS[self.corner].0.to_owned()),
            ("size", SIZES[self.size].0.to_string()),
            (
                "shape",
                if self.round { "round" } else { "square" }.to_owned(),
            ),
            ("zoom", ZOOMS[self.zoom].to_string()),
            (
                "rotate",
                if self.rotate { "true" } else { "false" }.to_owned(),
            ),
            (
                "hud_pings",
                if self.show_hud_pings { "true" } else { "false" }.to_owned(),
            ),
            (
                "minimap_pings",
                if self.show_minimap_pings { "true" } else { "false" }.to_owned(),
            ),
            (
                "show_coords",
                if self.show_coords { "true" } else { "false" }.to_owned(),
            ),
            (
                "show_beacons",
                if self.show_beacons { "true" } else { "false" }.to_owned(),
            ),
        ]
    }

    pub fn view(&self) -> (f32, f32) {
        (f32::from(SIZES[self.size].0), ZOOMS[self.zoom])
    }
}

/// Returns the cardinal facing direction name and axis representation.
pub fn cardinal_facing(yaw: f32) -> &'static str {
    let y = ((yaw + 180.0) % 360.0) - 180.0;
    if (-45.0..45.0).contains(&y) {
        "South (+Z)"
    } else if (45.0..135.0).contains(&y) {
        "West (-X)"
    } else if (-135.0..-45.0).contains(&y) {
        "East (+X)"
    } else {
        "North (-Z)"
    }
}

/// Raycast view vector from yaw and pitch (degrees).
/// Horizon yaw: 0 faces south (+Z), 90 faces west (-X).
pub fn view_vector(yaw: f32, pitch: f32) -> [f64; 3] {
    let y = f64::from(yaw).to_radians();
    let p = f64::from(pitch).to_radians();
    [-y.sin() * p.cos(), -p.sin(), y.cos() * p.cos()]
}

/// Simple eye-ray projection to estimate target coordinate up to `max_distance` blocks.
pub fn raycast_target(
    player_pos: [f64; 3],
    yaw: f32,
    pitch: f32,
    eye_height: f64,
    max_distance: f64,
) -> [f64; 3] {
    let dir = view_vector(yaw, pitch);
    [
        player_pos[0] + dir[0] * max_distance,
        player_pos[1] + eye_height + dir[1] * max_distance,
        player_pos[2] + dir[2] * max_distance,
    ]
}

/// Player snapshot info passed from the hook.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerSnapshot {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub eye_x: f64,
    pub eye_y: f64,
    pub eye_z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub fov: f32,
    pub dimension: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GuiTab {
    #[default]
    List,
    EditName,
    ColorGrid,
}

#[derive(Clone, Debug, Default)]
pub struct WaypointsGuiState {
    pub is_open: bool,
    pub selected_index: usize,
    pub active_tab: GuiTab,
    pub color_cursor: usize,
    pub name_buffer: String,
    pub scroll_offset: usize,
    pub screen_width: f32,
    pub screen_height: f32,
    pub was_creative: bool,
    pub frozen_look: Option<(f32, f32)>,
    pub suppress_mouse_frames: u8,
    pub last_frozen_look: Option<(f32, f32)>,
}

#[derive(Clone, Debug)]
pub struct WorldMapState {
    pub is_open: bool,
    pub pan_x: f64,
    pub pan_z: f64,
    pub zoom_index: usize,
    pub dragging: bool,
    pub drag_start_cursor: (f32, f32),
    pub drag_start_pan: (f64, f64),
    pub screen_width: f32,
    pub screen_height: f32,
    pub was_creative: bool,
    pub frozen_look: Option<(f32, f32)>,
    pub suppress_mouse_frames: u8,
    pub last_frozen_look: Option<(f32, f32)>,
    pub message: Option<(String, u32)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExploredChunk {
    pub color: [u8; 256],
    pub height: [i16; 256],
    pub depth: [u8; 256],
}

#[derive(Clone, Debug, Default)]
pub struct ExploredWorld {
    pub chunks: std::collections::HashMap<(String, i32, i32), ExploredChunk>,
    pub dirty: bool,
    pub revision: u64,
}

impl ExploredWorld {
    pub fn get_block(&self, dim: &str, x: i32, z: i32) -> Option<(u8, i16, u8)> {
        let cx = x >> 4;
        let cz = z >> 4;
        let chunk = self.chunks.get(&(dim.to_string(), cx, cz))?;
        let lx = (x & 15) as usize;
        let lz = (z & 15) as usize;
        let idx = lz * 16 + lx;
        Some((chunk.color[idx], chunk.height[idx], chunk.depth[idx]))
    }

    pub fn record_chunk(
        &mut self,
        dim: &str,
        cx: i32,
        cz: i32,
        color: [u8; 256],
        height: [i16; 256],
        depth: [u8; 256],
    ) {
        let key = (dim.to_string(), cx, cz);
        let updated = match self.chunks.get(&key) {
            Some(existing) => existing.color != color || existing.height != height,
            None => true,
        };
        if updated {
            self.chunks.insert(key, ExploredChunk { color, height, depth });
            self.dirty = true;
            self.revision = self.revision.wrapping_add(1);
        }
    }

    pub fn save_to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(12 + self.chunks.len() * 1050);
        buf.extend_from_slice(b"HMAP");
        buf.extend_from_slice(&1u32.to_le_bytes()); // Version 1
        buf.extend_from_slice(&(self.chunks.len() as u32).to_le_bytes());

        for ((dim, cx, cz), chunk) in &self.chunks {
            let dim_bytes = dim.as_bytes();
            buf.extend_from_slice(&(dim_bytes.len() as u16).to_le_bytes());
            buf.extend_from_slice(dim_bytes);
            buf.extend_from_slice(&cx.to_le_bytes());
            buf.extend_from_slice(&cz.to_le_bytes());
            buf.extend_from_slice(&chunk.color);
            for h in &chunk.height {
                buf.extend_from_slice(&h.to_le_bytes());
            }
            buf.extend_from_slice(&chunk.depth);
        }
        buf
    }

    pub fn load_from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 12 || &data[0..4] != b"HMAP" {
            return None;
        }
        let ver = u32::from_le_bytes(data[4..8].try_into().ok()?);
        if ver != 1 {
            return None;
        }
        let count = u32::from_le_bytes(data[8..12].try_into().ok()?) as usize;
        let mut offset = 12;
        let mut chunks = std::collections::HashMap::with_capacity(count);

        for _ in 0..count {
            if offset + 2 > data.len() { break; }
            let dim_len = u16::from_le_bytes(data[offset..offset+2].try_into().ok()?) as usize;
            offset += 2;
            if offset + dim_len > data.len() { break; }
            let dim = std::str::from_utf8(&data[offset..offset+dim_len]).ok()?.to_string();
            offset += dim_len;

            if offset + 8 + 256 + 512 + 256 > data.len() { break; }
            let cx = i32::from_le_bytes(data[offset..offset+4].try_into().ok()?);
            let cz = i32::from_le_bytes(data[offset+4..offset+8].try_into().ok()?);
            offset += 8;

            let mut color = [0u8; 256];
            color.copy_from_slice(&data[offset..offset+256]);
            offset += 256;

            let mut height = [0i16; 256];
            for i in 0..256 {
                height[i] = i16::from_le_bytes(data[offset+i*2..offset+i*2+2].try_into().ok()?);
            }
            offset += 512;

            let mut depth = [0u8; 256];
            depth.copy_from_slice(&data[offset..offset+256]);
            offset += 256;

            chunks.insert((dim, cx, cz), ExploredChunk { color, height, depth });
        }

        Some(Self {
            chunks,
            dirty: false,
            revision: 1,
        })
    }
}

pub fn save_explored_cache(explored: &ExploredWorld) {
    if !explored.dirty {
        return;
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/granafilo".into());
    let path = std::path::PathBuf::from(home).join(".local/share/horizon-launcher/minimap/explored_chunks.bin");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let bytes = explored.save_to_bytes();
    let _ = std::fs::write(path, bytes);
}

pub fn load_explored_cache() -> ExploredWorld {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/granafilo".into());
    let path = std::path::PathBuf::from(home).join(".local/share/horizon-launcher/minimap/explored_chunks.bin");
    if let Ok(bytes) = std::fs::read(path) {
        if let Some(world) = ExploredWorld::load_from_bytes(&bytes) {
            return world;
        }
    }
    ExploredWorld::default()
}

impl Default for WorldMapState {
    fn default() -> Self {
        Self {
            is_open: false,
            pan_x: 0.0,
            pan_z: 0.0,
            zoom_index: 1, // Default 1.0 bpp
            dragging: false,
            drag_start_cursor: (0.0, 0.0),
            drag_start_pan: (0.0, 0.0),
            screen_width: 0.0,
            screen_height: 0.0,
            was_creative: false,
            frozen_look: None,
            suppress_mouse_frames: 0,
            last_frozen_look: None,
            message: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WorldMapClickAction {
    None,
    Close,
    OpenWaypointsGui,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WorldMapKeyAction {
    None,
    Close,
    OpenWaypointsGui,
}

#[derive(Default)]
pub struct RuntimeState {
    pub settings: Option<Settings>,
    pub pings: Vec<Ping>,
    pub waypoints: Vec<Waypoint>,
    pub waypoints_loaded: bool,
    pub waypoints_dirty: bool,
    pub gui: WaypointsGuiState,
    pub world_map: WorldMapState,
    pub explored_world: ExploredWorld,
    pub explored_loaded: bool,
    pub map_texture_cache: Option<(i32, i32, usize, u64)>,
    pub map_quadrants: Option<[Vec<u8>; 4]>,
    pub consume_escape: bool,
}

thread_local! {
    static STATE: RefCell<RuntimeState> = RefCell::new(RuntimeState::default());
}

pub fn request_consume_escape() {
    STATE.with_borrow_mut(|s| {
        s.consume_escape = true;
    });
}

pub fn take_consume_escape() -> bool {
    STATE.with_borrow_mut(|s| {
        let val = s.consume_escape;
        s.consume_escape = false;
        val
    })
}

/// Loads waypoints from storage via reader if not already loaded.
pub fn load_waypoints(get: impl Fn(&str) -> Option<String>) {
    STATE.with_borrow_mut(|s| {
        if s.waypoints_loaded {
            return;
        }
        s.waypoints_loaded = true;
        if let Some(content) = get(KEY_WAYPOINTS) {
            s.waypoints.clear();
            for line in content.lines() {
                if let Some(wp) = Waypoint::deserialize(line) {
                    s.waypoints.push(wp);
                }
            }
        }
    });
}

/// Encodes all current waypoints for saving.
pub fn save_waypoints_content() -> String {
    STATE.with_borrow(|s| {
        let lines: Vec<String> = s.waypoints.iter().map(|wp| wp.serialize()).collect();
        lines.join("\n")
    })
}

/// Checks if waypoints need saving, and clears the dirty flag if so.
pub fn take_waypoints_dirty() -> bool {
    STATE.with_borrow_mut(|s| {
        if s.waypoints_dirty {
            s.waypoints_dirty = false;
            true
        } else {
            false
        }
    })
}

/// The settings, read once through `get` (one file per key).
pub fn settings(get: impl Fn(&str) -> Option<String>) -> Settings {
    load_waypoints(&get);
    STATE.with_borrow_mut(|s| *s.settings.get_or_insert_with(|| Settings::read(get)))
}

/// Replaces the settings.
pub fn set_settings(settings: Settings) {
    STATE.with_borrow_mut(|s| s.settings = Some(settings));
}

/// Adds a quick ping.
pub fn add_ping(ping: Ping) {
    STATE.with_borrow_mut(|s| {
        s.pings.push(ping);
    });
}

/// Adds a persistent waypoint.
pub fn add_waypoint(waypoint: Waypoint) {
    STATE.with_borrow_mut(|s| {
        s.waypoints.push(waypoint);
        s.waypoints_dirty = true;
    });
}

/// Available palette colors for waypoints.
pub const COLOR_PALETTE: [(&str, u32); 8] = [
    ("Cyan", 0x0033_CCFF),
    ("Lime", 0x0055_FF55),
    ("Amber", 0x00FF_AA00),
    ("Red", 0x00FF_5555),
    ("Purple", 0x00AA_00AA),
    ("Magenta", 0x00FF_55FF),
    ("Yellow", 0x00FF_FF55),
    ("White", 0x00FF_FFFF),
];

pub fn color_name(rgb: u32) -> &'static str {
    let pure = rgb & 0x00FF_FFFF;
    COLOR_PALETTE
        .iter()
        .find(|(_, c)| *c == pure)
        .map(|(n, _)| *n)
        .unwrap_or("Custom")
}

pub fn next_color(rgb: u32) -> u32 {
    let pure = rgb & 0x00FF_FFFF;
    let idx = COLOR_PALETTE.iter().position(|(_, c)| *c == pure).unwrap_or(0);
    COLOR_PALETTE[(idx + 1) % COLOR_PALETTE.len()].1
}

/// Generates the list of interactive buttons for the mod's settings page:
/// 1. Main Minimap & HUD settings
/// 2. Waypoint Management actions & items (Toggle visibility, cycle color, delete)
pub fn settings_page_buttons(get: impl Fn(&str) -> Option<String>) -> Vec<String> {
    let mut buttons = settings(&get).labels();
    STATE.with_borrow(|s| {
        if s.waypoints.is_empty() {
            buttons.push("--- No Waypoints (Press B in-game) ---".to_owned());
        } else {
            buttons.push(format!("--- WAYPOINTS LIST ({}) ---", s.waypoints.len()));
            for wp in &s.waypoints {
                let status = if wp.enabled { "VISIBLE" } else { "HIDDEN" };
                let col = color_name(wp.rgb);
                buttons.push(format!(
                    "[{status}] {} ({:.0}, {:.0}, {:.0}) [{col}]",
                    wp.name, wp.x, wp.y, wp.z
                ));
            }
            buttons.push("[Delete Last Waypoint]".to_owned());
            buttons.push("[Clear All Waypoints]".to_owned());
        }
    });
    buttons
}

/// Handles button clicks on the mod settings page.
pub fn on_settings_click(button: usize, get: impl Fn(&str) -> Option<String>) {
    let mut s = settings(&get);
    let labels_count = s.labels().len();

    if button < labels_count {
        s.click(button);
        set_settings(s);
        return;
    }

    let action_idx = button - labels_count;
    STATE.with_borrow_mut(|st| {
        if st.waypoints.is_empty() {
            return;
        }
        // action_idx 0 is header: "--- WAYPOINTS LIST ---"
        if action_idx == 0 {
            return;
        }
        let wp_idx = action_idx - 1;
        if wp_idx < st.waypoints.len() {
            let wp = &mut st.waypoints[wp_idx];
            // Clicking toggles visibility, or cycles color if visible
            if wp.enabled {
                wp.rgb = next_color(wp.rgb);
                // If it cycled back to Cyan, toggle off
                if (wp.rgb & 0x00FF_FFFF) == (COLOR_PALETTE[0].1 & 0x00FF_FFFF) {
                    wp.enabled = false;
                }
            } else {
                wp.enabled = true;
            }
            st.waypoints_dirty = true;
        } else {
            let extra_action = wp_idx - st.waypoints.len();
            if extra_action == 0 {
                // Delete Last
                if !st.waypoints.is_empty() {
                    st.waypoints.pop();
                    st.waypoints_dirty = true;
                }
            } else if extra_action == 1 {
                // Clear All
                if !st.waypoints.is_empty() {
                    st.waypoints.clear();
                    st.waypoints_dirty = true;
                }
            }
        }
    });
}

pub fn is_gui_open() -> bool {
    STATE.with_borrow(|s| s.gui.is_open)
}

fn do_close_gui(gui: &mut WaypointsGuiState) {
    gui.is_open = false;
    if let Some(look) = gui.frozen_look.take() {
        gui.last_frozen_look = Some(look);
        gui.suppress_mouse_frames = 3;
    }
}

pub fn close_gui() {
    STATE.with_borrow_mut(|s| {
        do_close_gui(&mut s.gui);
    });
}

pub fn was_creative() -> bool {
    false
}

pub fn get_frozen_look() -> Option<(f32, f32)> {
    STATE.with_borrow(|s| {
        if s.world_map.is_open {
            s.world_map.frozen_look.or(s.world_map.last_frozen_look)
        } else if s.gui.is_open {
            s.gui.frozen_look.or(s.gui.last_frozen_look)
        } else {
            None
        }
    })
}

pub fn take_suppress_mouse_look() -> Option<(f32, f32)> {
    STATE.with_borrow_mut(|s| {
        if s.world_map.suppress_mouse_frames > 0 {
            s.world_map.suppress_mouse_frames -= 1;
            let look = s.world_map.last_frozen_look;
            if s.world_map.suppress_mouse_frames == 0 {
                s.world_map.last_frozen_look = None;
            }
            look
        } else if s.gui.suppress_mouse_frames > 0 {
            s.gui.suppress_mouse_frames -= 1;
            let look = s.gui.last_frozen_look;
            if s.gui.suppress_mouse_frames == 0 {
                s.gui.last_frozen_look = None;
            }
            look
        } else {
            None
        }
    })
}

pub fn has_explored_chunk(dim: &str, cx: i32, cz: i32) -> bool {
    STATE.with_borrow_mut(|s| {
        if !s.explored_loaded {
            s.explored_loaded = true;
            s.explored_world = load_explored_cache();
        }
        s.explored_world.chunks.contains_key(&(dim.to_string(), cx, cz))
    })
}

pub fn record_chunk_columns(
    dim: &str,
    cx: i32,
    cz: i32,
    color: [u8; 256],
    height: [i16; 256],
    depth: [u8; 256],
) {
    STATE.with_borrow_mut(|s| {
        if !s.explored_loaded {
            s.explored_loaded = true;
            s.explored_world = load_explored_cache();
        }
        s.explored_world.record_chunk(dim, cx, cz, color, height, depth);
    });
}

pub fn save_explored_world_if_dirty() {
    STATE.with_borrow_mut(|s| {
        if s.explored_world.dirty {
            save_explored_cache(&s.explored_world);
            s.explored_world.dirty = false;
        }
    });
}

fn do_close_world_map(wm: &mut WorldMapState) {
    wm.is_open = false;
    wm.dragging = false;
    if let Some(look) = wm.frozen_look.take() {
        wm.last_frozen_look = Some(look);
        wm.suppress_mouse_frames = 3;
    }
}

pub fn is_world_map_open() -> bool {
    STATE.with_borrow(|s| s.world_map.is_open)
}

pub fn get_world_map_view() -> Option<(f64, f64, f32)> {
    STATE.with_borrow(|s| {
        if s.world_map.is_open {
            let bpp = WORLD_MAP_ZOOMS[s.world_map.zoom_index];
            Some((s.world_map.pan_x, s.world_map.pan_z, bpp))
        } else {
            None
        }
    })
}

pub fn close_world_map() {
    STATE.with_borrow_mut(|s| {
        do_close_world_map(&mut s.world_map);
    });
}

pub fn toggle_world_map(
    player: Option<PlayerSnapshot>,
    _was_creative: bool,
    yaw: f32,
    pitch: f32,
) -> bool {
    STATE.with_borrow_mut(|s| {
        if s.world_map.is_open {
            do_close_world_map(&mut s.world_map);
        } else {
            if s.gui.is_open {
                do_close_gui(&mut s.gui);
            }
            if !s.explored_loaded {
                s.explored_loaded = true;
                s.explored_world = load_explored_cache();
            }
            s.world_map.is_open = true;
            s.world_map.was_creative = false;
            s.world_map.frozen_look = Some((yaw, pitch));
            s.world_map.last_frozen_look = Some((yaw, pitch));
            s.world_map.suppress_mouse_frames = 0;
            s.world_map.dragging = false;
            if let Some(p) = player {
                s.world_map.pan_x = p.x;
                s.world_map.pan_z = p.z;
            }
        }
        s.world_map.is_open
    })
}

pub fn world_map_zoom_in() {
    STATE.with_borrow_mut(|s| {
        if s.world_map.zoom_index > 0 {
            s.world_map.zoom_index -= 1;
        }
    });
}

pub fn world_map_zoom_out() {
    STATE.with_borrow_mut(|s| {
        if s.world_map.zoom_index + 1 < WORLD_MAP_ZOOMS.len() {
            s.world_map.zoom_index += 1;
        }
    });
}

pub fn world_map_pan(dx: f64, dz: f64) {
    STATE.with_borrow_mut(|s| {
        s.world_map.pan_x += dx;
        s.world_map.pan_z += dz;
    });
}

pub fn world_map_center(player: Option<PlayerSnapshot>) {
    if let Some(p) = player {
        STATE.with_borrow_mut(|s| {
            s.world_map.pan_x = p.x;
            s.world_map.pan_z = p.z;
            s.world_map.message = Some(("Centered on Player".to_string(), 40));
        });
    }
}

pub fn start_world_map_drag(cursor: (f32, f32)) {
    STATE.with_borrow_mut(|s| {
        s.world_map.dragging = true;
        s.world_map.drag_start_cursor = cursor;
        s.world_map.drag_start_pan = (s.world_map.pan_x, s.world_map.pan_z);
    });
}

pub fn update_world_map_drag(cursor: (f32, f32)) {
    STATE.with_borrow_mut(|s| {
        if !s.world_map.dragging {
            return;
        }
        let sw = if s.world_map.screen_width > 0.0 { s.world_map.screen_width } else { 427.0 };
        let sh = if s.world_map.screen_height > 0.0 { s.world_map.screen_height } else { 240.0 };
        let map_size = (sw - 24.0).min(sh - 62.0).min(512.0);
        let bpp = WORLD_MAP_ZOOMS[s.world_map.zoom_index];
        let pixels_per_block = map_size / (256.0 * bpp);

        let dx_screen = cursor.0 - s.world_map.drag_start_cursor.0;
        let dy_screen = cursor.1 - s.world_map.drag_start_cursor.1;
        let world_dx = -(f64::from(dx_screen / pixels_per_block));
        let world_dz = -(f64::from(dy_screen / pixels_per_block));

        s.world_map.pan_x = s.world_map.drag_start_pan.0 + world_dx;
        s.world_map.pan_z = s.world_map.drag_start_pan.1 + world_dz;
    });
}

pub fn end_world_map_drag() {
    STATE.with_borrow_mut(|s| {
        s.world_map.dragging = false;
    });
}

pub fn on_world_map_click(
    cursor_x: f32,
    cursor_y: f32,
    player: Option<PlayerSnapshot>,
) -> WorldMapClickAction {
    let cursor = (cursor_x, cursor_y);
    let mut action = WorldMapClickAction::None;

    STATE.with_borrow_mut(|s| {
        if !s.world_map.is_open {
            return;
        }
        let sw = if s.world_map.screen_width > 0.0 { s.world_map.screen_width } else { 427.0 };
        let sh = if s.world_map.screen_height > 0.0 { s.world_map.screen_height } else { 240.0 };
        let map_size = (sw - 24.0).min(sh - 62.0).min(512.0);
        let map_x = ((sw - map_size) / 2.0).round();
        let map_y = (30.0 + (sh - 62.0 - map_size) / 2.0).round();
        let map_rect = [map_x, map_y, map_size, map_size];

        let btn_y = sh - 26.0;

        // Button 1: Zoom [-]
        let btn_zoom_out = [map_x, btn_y, 48.0, 20.0];
        if is_inside(cursor, btn_zoom_out) {
            if s.world_map.zoom_index + 1 < WORLD_MAP_ZOOMS.len() {
                s.world_map.zoom_index += 1;
            }
            return;
        }

        // Button 2: Zoom [+]
        let btn_zoom_in = [map_x + 52.0, btn_y, 48.0, 20.0];
        if is_inside(cursor, btn_zoom_in) {
            if s.world_map.zoom_index > 0 {
                s.world_map.zoom_index -= 1;
            }
            return;
        }

        // Button 3: Center
        let btn_center = [map_x + 104.0, btn_y, 60.0, 20.0];
        if is_inside(cursor, btn_center) {
            if let Some(p) = player {
                s.world_map.pan_x = p.x;
                s.world_map.pan_z = p.z;
                s.world_map.message = Some(("Centered on Player".to_string(), 40));
            }
            return;
        }

        // Button 4: Waypoints
        let btn_waypoints = [map_x + map_size - 144.0, btn_y, 76.0, 20.0];
        if is_inside(cursor, btn_waypoints) {
            do_close_world_map(&mut s.world_map);
            action = WorldMapClickAction::OpenWaypointsGui;
            return;
        }

        // Button 5: Close
        let btn_close = [map_x + map_size - 64.0, btn_y, 64.0, 20.0];
        if is_inside(cursor, btn_close) {
            do_close_world_map(&mut s.world_map);
            action = WorldMapClickAction::Close;
            return;
        }

        // Click on map viewport -> start dragging!
        if is_inside(cursor, map_rect) {
            s.world_map.dragging = true;
            s.world_map.drag_start_cursor = cursor;
            s.world_map.drag_start_pan = (s.world_map.pan_x, s.world_map.pan_z);
        }
    });

    action
}

pub fn create_world_map_waypoint(
    cursor_x: f32,
    cursor_y: f32,
    player: Option<PlayerSnapshot>,
) -> bool {
    let cursor = (cursor_x, cursor_y);
    let mut created = false;

    STATE.with_borrow_mut(|s| {
        if !s.world_map.is_open {
            return;
        }
        let sw = if s.world_map.screen_width > 0.0 { s.world_map.screen_width } else { 427.0 };
        let sh = if s.world_map.screen_height > 0.0 { s.world_map.screen_height } else { 240.0 };
        let map_size = (sw - 24.0).min(sh - 62.0).min(512.0);
        let map_x = ((sw - map_size) / 2.0).round();
        let map_y = (30.0 + (sh - 62.0 - map_size) / 2.0).round();
        let map_rect = [map_x, map_y, map_size, map_size];

        let (world_x, world_z, world_y, dimension) = if is_inside(cursor, map_rect) {
            let map_cx = map_x + map_size / 2.0;
            let map_cy = map_y + map_size / 2.0;
            let bpp = WORLD_MAP_ZOOMS[s.world_map.zoom_index];
            let scale = map_size / 128.0;

            let norm_x = (cursor.0 - map_cx) / scale;
            let norm_z = (cursor.1 - map_cy) / scale;
            let wx = (s.world_map.pan_x + f64::from(norm_x * bpp)).round();
            let wz = (s.world_map.pan_z + f64::from(norm_z * bpp)).round();
            let dim = player.as_ref().map(|p| p.dimension.clone()).unwrap_or_else(|| "minecraft:overworld".into());
            let wy = s.explored_world.get_block(&dim, wx as i32, wz as i32).map(|(_, h, _)| h as f64).unwrap_or_else(|| player.as_ref().map(|p| p.y).unwrap_or(64.0));
            (wx, wz, wy, dim)
        } else if let Some(ref p) = player {
            (p.x.round(), p.z.round(), p.y.round(), p.dimension.clone())
        } else {
            (0.0, 0.0, 64.0, "minecraft:overworld".into())
        };

        let new_idx = s.waypoints.len() + 1;
        let wp = Waypoint {
            name: format!("Waypoint #{}", new_idx),
            x: world_x,
            y: world_y,
            z: world_z,
            dimension,
            rgb: COLOR_PALETTE[0].1, // Cyan default
            enabled: true,
        };
        s.waypoints.push(wp);
        s.waypoints_dirty = true;
        s.world_map.message = Some((
            format!("Created Waypoint #{} at [{}, {}, {}]", new_idx, world_x as i32, world_y as i32, world_z as i32),
            60,
        ));
        created = true;
    });

    created
}

pub fn on_world_map_right_click(
    _cursor_x: f32,
    _cursor_y: f32,
    _player: Option<PlayerSnapshot>,
) -> bool {
    false
}

pub fn on_world_map_key(
    key: &str,
    pressed: bool,
    world_map_key: &str,
    waypoints_key: &str,
    player: Option<PlayerSnapshot>,
) -> WorldMapKeyAction {
    if !pressed {
        return WorldMapKeyAction::None;
    }

    let mut action = WorldMapKeyAction::None;

    STATE.with_borrow_mut(|s| {
        if !s.world_map.is_open {
            return;
        }

        if key == "key.keyboard.escape" || key == world_map_key {
            do_close_world_map(&mut s.world_map);
            action = WorldMapKeyAction::Close;
            return;
        }

        if key == waypoints_key {
            do_close_world_map(&mut s.world_map);
            action = WorldMapKeyAction::OpenWaypointsGui;
            return;
        }

        let bpp = WORLD_MAP_ZOOMS[s.world_map.zoom_index];
        let pan_step = (32.0 * bpp) as f64;

        match key {
            "key.keyboard.w" | "key.keyboard.up" => {
                s.world_map.pan_z -= pan_step;
            }
            "key.keyboard.s" | "key.keyboard.down" => {
                s.world_map.pan_z += pan_step;
            }
            "key.keyboard.a" | "key.keyboard.left" => {
                s.world_map.pan_x -= pan_step;
            }
            "key.keyboard.d" | "key.keyboard.right" => {
                s.world_map.pan_x += pan_step;
            }
            "key.keyboard.equal" | "key.keyboard.plus" => {
                if s.world_map.zoom_index > 0 {
                    s.world_map.zoom_index -= 1;
                }
            }
            "key.keyboard.minus" => {
                if s.world_map.zoom_index + 1 < WORLD_MAP_ZOOMS.len() {
                    s.world_map.zoom_index += 1;
                }
            }
            "key.keyboard.space" | "key.keyboard.c" => {
                if let Some(p) = player {
                    s.world_map.pan_x = p.x;
                    s.world_map.pan_z = p.z;
                    s.world_map.message = Some(("Centered on Player".to_string(), 40));
                }
            }
            _ => {}
        }
    });

    action
}

pub fn toggle_gui(
    player: Option<PlayerSnapshot>,
    _was_creative: bool,
    yaw: f32,
    pitch: f32,
) -> bool {
    let _ = player;
    STATE.with_borrow_mut(|s| {
        if s.gui.is_open {
            do_close_gui(&mut s.gui);
        } else {
            s.gui.is_open = true;
            s.gui.was_creative = false;
            s.gui.frozen_look = Some((yaw, pitch));
            s.gui.last_frozen_look = Some((yaw, pitch));
            s.gui.suppress_mouse_frames = 0;
            if s.waypoints.is_empty() {
                s.gui.selected_index = 0;
                s.gui.name_buffer.clear();
                s.gui.color_cursor = 0;
                s.gui.active_tab = GuiTab::List;
            } else {
                if s.gui.selected_index >= s.waypoints.len() {
                    s.gui.selected_index = 0;
                }
                let wp = &s.waypoints[s.gui.selected_index];
                s.gui.name_buffer = wp.name.clone();
                let pure = wp.rgb & 0x00FF_FFFF;
                s.gui.color_cursor =
                    COLOR_PALETTE.iter().position(|(_, c)| *c == pure).unwrap_or(0);
                s.gui.active_tab = GuiTab::List;
            }
        }
        s.gui.is_open
    })
}

pub fn key_to_char(key: &str, current_text: &str) -> Option<char> {
    if let Some(suffix) = key.strip_prefix("key.keyboard.") {
        if suffix.len() == 1 {
            let c = suffix.chars().next().unwrap();
            if c.is_ascii_alphabetic() {
                let start_word = current_text.is_empty() || current_text.ends_with(' ');
                return Some(if start_word {
                    c.to_ascii_uppercase()
                } else {
                    c.to_ascii_lowercase()
                });
            }
            if c.is_ascii_digit() {
                return Some(c);
            }
        }
        match suffix {
            "space" => Some(' '),
            "minus" => Some('-'),
            "period" => Some('.'),
            "slash" => Some('/'),
            _ => None,
        }
    } else {
        None
    }
}

pub fn on_gui_key(
    key: &str,
    pressed: bool,
    waypoints_gui_key: &str,
    player: Option<PlayerSnapshot>,
) {
    if !pressed {
        return;
    }

    STATE.with_borrow_mut(|s| {
        if !s.gui.is_open {
            return;
        }

        // Closing with Escape or Menu key
        if key == "key.keyboard.escape" || key == waypoints_gui_key {
            if s.gui.active_tab != GuiTab::List {
                // If editing or in color grid, first Escape cancels and returns to List
                if let Some(wp) = s.waypoints.get(s.gui.selected_index) {
                    s.gui.name_buffer = wp.name.clone();
                    let pure = wp.rgb & 0x00FF_FFFF;
                    s.gui.color_cursor =
                        COLOR_PALETTE.iter().position(|(_, c)| *c == pure).unwrap_or(0);
                }
                s.gui.active_tab = GuiTab::List;
            } else {
                do_close_gui(&mut s.gui);
            }
            return;
        }

        // Tab switches between panes/modes
        if key == "key.keyboard.tab" {
            if s.waypoints.is_empty() {
                return;
            }
            s.gui.active_tab = match s.gui.active_tab {
                GuiTab::List => GuiTab::EditName,
                GuiTab::EditName => {
                    // Commit name before moving to color grid
                    if !s.gui.name_buffer.trim().is_empty()
                        && s.gui.selected_index < s.waypoints.len()
                    {
                        s.waypoints[s.gui.selected_index].name =
                            s.gui.name_buffer.trim().to_string();
                        s.waypoints_dirty = true;
                    }
                    GuiTab::ColorGrid
                }
                GuiTab::ColorGrid => GuiTab::List,
            };
            return;
        }

        match s.gui.active_tab {
            GuiTab::EditName => {
                if key == "key.keyboard.enter" {
                    if !s.gui.name_buffer.trim().is_empty()
                        && s.gui.selected_index < s.waypoints.len()
                    {
                        s.waypoints[s.gui.selected_index].name =
                            s.gui.name_buffer.trim().to_string();
                        s.waypoints_dirty = true;
                    }
                    s.gui.active_tab = GuiTab::List;
                } else if key == "key.keyboard.backspace" {
                    s.gui.name_buffer.pop();
                } else if let Some(c) = key_to_char(key, &s.gui.name_buffer) {
                    if s.gui.name_buffer.len() < 24 {
                        s.gui.name_buffer.push(c);
                    }
                }
            }
            GuiTab::ColorGrid => {
                let grid_cols = 4;
                match key {
                    "key.keyboard.left" => {
                        if s.gui.color_cursor % grid_cols > 0 {
                            s.gui.color_cursor -= 1;
                        }
                    }
                    "key.keyboard.right" => {
                        if s.gui.color_cursor % grid_cols < grid_cols - 1
                            && s.gui.color_cursor + 1 < COLOR_PALETTE.len()
                        {
                            s.gui.color_cursor += 1;
                        }
                    }
                    "key.keyboard.up" => {
                        if s.gui.color_cursor >= grid_cols {
                            s.gui.color_cursor -= grid_cols;
                        }
                    }
                    "key.keyboard.down" => {
                        if s.gui.color_cursor + grid_cols < COLOR_PALETTE.len() {
                            s.gui.color_cursor += grid_cols;
                        }
                    }
                    "key.keyboard.enter" | "key.keyboard.space" => {
                        if s.gui.selected_index < s.waypoints.len() {
                            s.waypoints[s.gui.selected_index].rgb =
                                COLOR_PALETTE[s.gui.color_cursor].1;
                            s.waypoints_dirty = true;
                        }
                        s.gui.active_tab = GuiTab::List;
                    }
                    _ => {}
                }
            }
            GuiTab::List => match key {
                "key.keyboard.up" | "key.keyboard.w" => {
                    if s.gui.selected_index > 0 {
                        s.gui.selected_index -= 1;
                    }
                    if s.gui.selected_index < s.gui.scroll_offset {
                        s.gui.scroll_offset = s.gui.selected_index;
                    }
                    if let Some(wp) = s.waypoints.get(s.gui.selected_index) {
                        s.gui.name_buffer = wp.name.clone();
                        let pure = wp.rgb & 0x00FF_FFFF;
                        s.gui.color_cursor =
                            COLOR_PALETTE.iter().position(|(_, c)| *c == pure).unwrap_or(0);
                    }
                }
                "key.keyboard.down" | "key.keyboard.s" => {
                    if !s.waypoints.is_empty() && s.gui.selected_index + 1 < s.waypoints.len() {
                        s.gui.selected_index += 1;
                    }
                    let max_visible = 6;
                    if s.gui.selected_index >= s.gui.scroll_offset + max_visible {
                        s.gui.scroll_offset = s.gui.selected_index + 1 - max_visible;
                    }
                    if let Some(wp) = s.waypoints.get(s.gui.selected_index) {
                        s.gui.name_buffer = wp.name.clone();
                        let pure = wp.rgb & 0x00FF_FFFF;
                        s.gui.color_cursor =
                            COLOR_PALETTE.iter().position(|(_, c)| *c == pure).unwrap_or(0);
                    }
                }
                "key.keyboard.v" => {
                    if let Some(wp) = s.waypoints.get_mut(s.gui.selected_index) {
                        wp.enabled = !wp.enabled;
                        s.waypoints_dirty = true;
                    }
                }
                "key.keyboard.d" | "key.keyboard.delete" => {
                    if !s.waypoints.is_empty() && s.gui.selected_index < s.waypoints.len() {
                        s.waypoints.remove(s.gui.selected_index);
                        s.waypoints_dirty = true;
                        if s.gui.selected_index >= s.waypoints.len() && !s.waypoints.is_empty() {
                            s.gui.selected_index = s.waypoints.len() - 1;
                        }
                        if let Some(wp) = s.waypoints.get(s.gui.selected_index) {
                            s.gui.name_buffer = wp.name.clone();
                            let pure = wp.rgb & 0x00FF_FFFF;
                            s.gui.color_cursor =
                                COLOR_PALETTE.iter().position(|(_, c)| *c == pure).unwrap_or(0);
                        } else {
                            s.gui.name_buffer.clear();
                            s.gui.color_cursor = 0;
                        }
                    }
                }
                "key.keyboard.e" | "key.keyboard.enter" => {
                    if !s.waypoints.is_empty() {
                        if let Some(wp) = s.waypoints.get(s.gui.selected_index) {
                            s.gui.name_buffer = wp.name.clone();
                        }
                        s.gui.active_tab = GuiTab::EditName;
                    }
                }
                "key.keyboard.c" => {
                    if !s.waypoints.is_empty() {
                        if let Some(wp) = s.waypoints.get(s.gui.selected_index) {
                            let pure = wp.rgb & 0x00FF_FFFF;
                            s.gui.color_cursor =
                                COLOR_PALETTE.iter().position(|(_, c)| *c == pure).unwrap_or(0);
                        }
                        s.gui.active_tab = GuiTab::ColorGrid;
                    }
                }
                "key.keyboard.n" => {
                    if let Some(p) = player {
                        let new_idx = s.waypoints.len() + 1;
                        let wp = Waypoint {
                            name: format!("Waypoint #{}", new_idx),
                            x: p.x,
                            y: p.y,
                            z: p.z,
                            dimension: p.dimension,
                            rgb: COLOR_PALETTE[0].1,
                            enabled: true,
                        };
                        s.waypoints.push(wp);
                        s.gui.selected_index = s.waypoints.len() - 1;
                        s.gui.name_buffer = format!("Waypoint #{}", new_idx);
                        s.gui.color_cursor = 0;
                        s.gui.active_tab = GuiTab::EditName;
                        s.waypoints_dirty = true;
                    }
                }
                _ => {}
            },
        }
    });
}

pub fn is_inside(cursor: (f32, f32), rect: [f32; 4]) -> bool {
    cursor.0 >= rect[0]
        && cursor.0 < rect[0] + rect[2]
        && cursor.1 >= rect[1]
        && cursor.1 < rect[1] + rect[3]
}

pub fn on_gui_click(
    cursor_x: f32,
    cursor_y: f32,
    player: Option<PlayerSnapshot>,
) -> bool {
    let cursor = (cursor_x, cursor_y);
    let mut close = false;

    STATE.with_borrow_mut(|s| {
        if !s.gui.is_open {
            return;
        }

        let sw = if s.gui.screen_width > 0.0 { s.gui.screen_width } else { 427.0 };
        let sh = if s.gui.screen_height > 0.0 { s.gui.screen_height } else { 240.0 };

        // 1. Color Palette Swatches (y = 30.0)
        let pal_x = ((sw - 192.0) / 2.0).round();
        for (idx, _col) in COLOR_PALETTE.iter().enumerate() {
            let swatch_rect = [pal_x + (idx as f32) * 24.0, 30.0, 20.0, 14.0];
            if is_inside(cursor, swatch_rect) {
                if s.gui.selected_index < s.waypoints.len() {
                    s.waypoints[s.gui.selected_index].rgb = COLOR_PALETTE[idx].1;
                    s.waypoints_dirty = true;
                    s.gui.color_cursor = idx;
                }
                return;
            }
        }

        // 2. Waypoints List Rows
        let list_top = 48.0;
        let list_bottom = (sh - 60.0).max(list_top + 40.0);
        let list_w = 280.0_f32.min(sw - 24.0);
        let list_x = ((sw - list_w) / 2.0).round();
        let spacing = 26.0;
        let max_visible = (((list_bottom - list_top - 6.0) / spacing).floor() as usize).max(1);

        for (i, wp) in s.waypoints.iter().enumerate().skip(s.gui.scroll_offset).take(max_visible) {
            let item_y = list_top + 4.0 + ((i - s.gui.scroll_offset) as f32) * spacing;
            let item_rect = [list_x, item_y, list_w, 24.0];
            if is_inside(cursor, item_rect) {
                s.gui.selected_index = i;
                s.gui.name_buffer = wp.name.clone();
                let pure = wp.rgb & 0x00FF_FFFF;
                s.gui.color_cursor =
                    COLOR_PALETTE.iter().position(|(_, c)| *c == pure).unwrap_or(0);
                return;
            }
        }

        let cx = (sw / 2.0).round();

        // Button 1: Toggle Visibility
        let btn1_rect = [cx - 145.0, sh - 52.0, 92.0, 20.0];
        if is_inside(cursor, btn1_rect) {
            if let Some(wp) = s.waypoints.get_mut(s.gui.selected_index) {
                wp.enabled = !wp.enabled;
                s.waypoints_dirty = true;
            }
            return;
        }

        // Button 2: Edit Name
        let btn2_rect = [cx - 46.0, sh - 52.0, 92.0, 20.0];
        if is_inside(cursor, btn2_rect) {
            if s.gui.active_tab == GuiTab::EditName {
                if !s.gui.name_buffer.trim().is_empty() && s.gui.selected_index < s.waypoints.len() {
                    s.waypoints[s.gui.selected_index].name = s.gui.name_buffer.trim().to_string();
                    s.waypoints_dirty = true;
                }
                s.gui.active_tab = GuiTab::List;
            } else if !s.waypoints.is_empty() {
                if let Some(wp) = s.waypoints.get(s.gui.selected_index) {
                    s.gui.name_buffer = wp.name.clone();
                }
                s.gui.active_tab = GuiTab::EditName;
            }
            return;
        }

        // Button 3: Delete
        let btn3_rect = [cx + 53.0, sh - 52.0, 92.0, 20.0];
        if is_inside(cursor, btn3_rect) {
            if !s.waypoints.is_empty() && s.gui.selected_index < s.waypoints.len() {
                s.waypoints.remove(s.gui.selected_index);
                s.waypoints_dirty = true;
                if s.gui.selected_index >= s.waypoints.len() && !s.waypoints.is_empty() {
                    s.gui.selected_index = s.waypoints.len() - 1;
                }
                if let Some(wp) = s.waypoints.get(s.gui.selected_index) {
                    s.gui.name_buffer = wp.name.clone();
                    let pure = wp.rgb & 0x00FF_FFFF;
                    s.gui.color_cursor =
                        COLOR_PALETTE.iter().position(|(_, c)| *c == pure).unwrap_or(0);
                } else {
                    s.gui.name_buffer.clear();
                    s.gui.color_cursor = 0;
                }
            }
            return;
        }

        // Button 4: Create Waypoint
        let btn4_rect = [cx - 145.0, sh - 28.0, 140.0, 20.0];
        if is_inside(cursor, btn4_rect) {
            if let Some(p) = player {
                let new_idx = s.waypoints.len() + 1;
                let wp = Waypoint {
                    name: format!("Waypoint #{}", new_idx),
                    x: p.x,
                    y: p.y,
                    z: p.z,
                    dimension: p.dimension,
                    rgb: COLOR_PALETTE[0].1,
                    enabled: true,
                };
                s.waypoints.push(wp);
                s.gui.selected_index = s.waypoints.len() - 1;
                s.gui.name_buffer = format!("Waypoint #{}", new_idx);
                s.gui.color_cursor = 0;
                s.gui.active_tab = GuiTab::EditName;
                s.waypoints_dirty = true;
            }
            return;
        }

        // Button 5: Done
        let btn5_rect = [cx + 5.0, sh - 28.0, 140.0, 20.0];
        if is_inside(cursor, btn5_rect) {
            if s.gui.active_tab == GuiTab::EditName
                && !s.gui.name_buffer.trim().is_empty()
                && s.gui.selected_index < s.waypoints.len()
            {
                s.waypoints[s.gui.selected_index].name = s.gui.name_buffer.trim().to_string();
                s.waypoints_dirty = true;
            }
            do_close_gui(&mut s.gui);
            close = true;
            return;
        }
    });

    close
}

pub fn draw_waypoints_gui(
    screen_width: f32,
    screen_height: f32,
    cursor: (f32, f32),
    player: &PlayerSnapshot,
    ops: &mut Vec<HudDrawOp>,
) {
    STATE.with_borrow_mut(|s| {
        if !s.gui.is_open {
            return;
        }
        s.gui.screen_width = screen_width;
        s.gui.screen_height = screen_height;

        // 1. Fullscreen dark backdrop & menu background
        ops.push(HudDrawOp::Fill {
            rect: [0.0, 0.0, screen_width, screen_height],
            argb: 0xD010_1010,
        });
        ops.push(HudDrawOp::MenuBackground {
            rect: [0.0, 0.0, screen_width, screen_height],
        });

        // 2. Title: "Waypoints"
        ops.push(HudDrawOp::CenteredText {
            text: "Waypoints".to_string(),
            y: 10.0,
            color: 0xFFFF_FFFF,
        });

        // 3. Color palette swatches row (centered at y = 30.0)
        let pal_x = ((screen_width - 192.0) / 2.0).round();
        for (idx, (_cname, rgb)) in COLOR_PALETTE.iter().enumerate() {
            let bx = pal_x + (idx as f32) * 24.0;
            let by = 30.0;
            let bw = 20.0;
            let bh = 14.0;
            let rect = [bx, by, bw, bh];

            let is_swatch_hover = is_inside(cursor, rect);
            let is_selected_color = s
                .waypoints
                .get(s.gui.selected_index)
                .is_some_and(|w| (w.rgb & 0x00FF_FFFF) == (*rgb & 0x00FF_FFFF));

            ops.push(HudDrawOp::Fill {
                rect,
                argb: 0xFF00_0000 | rgb,
            });
            // 1px black inner border
            ops.push(HudDrawOp::Fill {
                rect: [bx, by, bw, 1.0],
                argb: 0xFF00_0000,
            });
            ops.push(HudDrawOp::Fill {
                rect: [bx, by + bh - 1.0, bw, 1.0],
                argb: 0xFF00_0000,
            });
            ops.push(HudDrawOp::Fill {
                rect: [bx, by, 1.0, bh],
                argb: 0xFF00_0000,
            });
            ops.push(HudDrawOp::Fill {
                rect: [bx + bw - 1.0, by, 1.0, bh],
                argb: 0xFF00_0000,
            });

            if is_selected_color || is_swatch_hover {
                let border_col = if is_selected_color {
                    0xFFFF_FFFF
                } else {
                    0xFFAA_AAAA
                };
                ops.push(HudDrawOp::Fill {
                    rect: [bx - 1.0, by - 1.0, bw + 2.0, 1.0],
                    argb: border_col,
                });
                ops.push(HudDrawOp::Fill {
                    rect: [bx - 1.0, by + bh, bw + 2.0, 1.0],
                    argb: border_col,
                });
                ops.push(HudDrawOp::Fill {
                    rect: [bx - 1.0, by - 1.0, 1.0, bh + 2.0],
                    argb: border_col,
                });
                ops.push(HudDrawOp::Fill {
                    rect: [bx + bw, by - 1.0, 1.0, bh + 2.0],
                    argb: border_col,
                });
            }
        }

        // 4. List Container Background & Separators
        let list_top = 48.0;
        let list_bottom = (screen_height - 60.0).max(list_top + 40.0);
        ops.push(HudDrawOp::ListBackground {
            top: list_top,
            bottom: list_bottom,
        });
        ops.push(HudDrawOp::Separators {
            top: list_top,
            bottom: list_bottom,
        });

        // 5. Waypoints List Rows
        let list_w = 280.0_f32.min(screen_width - 24.0);
        let list_x = ((screen_width - list_w) / 2.0).round();
        let spacing = 26.0;
        let max_visible = (((list_bottom - list_top - 6.0) / spacing).floor() as usize).max(1);

        if s.waypoints.is_empty() {
            ops.push(HudDrawOp::CenteredText {
                text: "No waypoints yet. Click [Create Waypoint] below!".to_string(),
                y: list_top + 30.0,
                color: 0xFFAA_AAAA,
            });
        } else {
            for (i, wp) in s
                .waypoints
                .iter()
                .enumerate()
                .skip(s.gui.scroll_offset)
                .take(max_visible)
            {
                let item_y = list_top + 4.0 + ((i - s.gui.scroll_offset) as f32) * spacing;
                let item_rect = [list_x, item_y, list_w, 24.0];
                let is_hover = is_inside(cursor, item_rect);
                let is_selected = i == s.gui.selected_index;

                if is_selected {
                    // Minecraft list item selected outline
                    ops.push(HudDrawOp::Fill {
                        rect: item_rect,
                        argb: 0x8022_2222,
                    });
                    ops.push(HudDrawOp::Fill {
                        rect: [list_x, item_y, list_w, 1.0],
                        argb: 0xFF88_8888,
                    });
                    ops.push(HudDrawOp::Fill {
                        rect: [list_x, item_y + 23.0, list_w, 1.0],
                        argb: 0xFF88_8888,
                    });
                    ops.push(HudDrawOp::Fill {
                        rect: [list_x, item_y, 1.0, 24.0],
                        argb: 0xFF88_8888,
                    });
                    ops.push(HudDrawOp::Fill {
                        rect: [list_x + list_w - 1.0, item_y, 1.0, 24.0],
                        argb: 0xFF88_8888,
                    });
                } else if is_hover {
                    ops.push(HudDrawOp::Fill {
                        rect: item_rect,
                        argb: 0x30FF_FFFF,
                    });
                }

                // Color swatch
                ops.push(HudDrawOp::Fill {
                    rect: [list_x + 5.0, item_y + 6.0, 12.0, 12.0],
                    argb: 0xFF00_0000 | wp.rgb,
                });
                ops.push(HudDrawOp::Fill {
                    rect: [list_x + 5.0, item_y + 6.0, 12.0, 1.0],
                    argb: 0xFF00_0000,
                });
                ops.push(HudDrawOp::Fill {
                    rect: [list_x + 5.0, item_y + 17.0, 12.0, 1.0],
                    argb: 0xFF00_0000,
                });
                ops.push(HudDrawOp::Fill {
                    rect: [list_x + 5.0, item_y + 6.0, 1.0, 12.0],
                    argb: 0xFF00_0000,
                });
                ops.push(HudDrawOp::Fill {
                    rect: [list_x + 16.0, item_y + 6.0, 1.0, 12.0],
                    argb: 0xFF00_0000,
                });

                // Waypoint Name
                let name_display: String = wp.name.chars().take(16).collect();
                ops.push(HudDrawOp::Text {
                    text: name_display,
                    at: [list_x + 22.0, item_y + 3.0],
                    rgb: if wp.enabled {
                        0x00FF_FFFF
                    } else {
                        0x0088_8888
                    },
                    shadow: true,
                });

                // Coordinates & Distance
                let dist = wp.distance_to(player.x, player.y, player.z);
                let coords_line =
                    format!("{:.0}, {:.0}, {:.0} ({:.0}m)", wp.x, wp.y, wp.z, dist);
                ops.push(HudDrawOp::Text {
                    text: coords_line,
                    at: [list_x + 22.0, item_y + 13.0],
                    rgb: 0x00AA_AAAA,
                    shadow: true,
                });

                // Status tag
                let (status_text, status_col) = if wp.enabled {
                    ("[Vis]", 0x0055_FF55)
                } else {
                    ("[Hid]", 0x00FF_5555)
                };
                ops.push(HudDrawOp::Text {
                    text: status_text.to_string(),
                    at: [list_x + list_w - 38.0, item_y + 8.0],
                    rgb: status_col,
                    shadow: true,
                });
            }
        }

        // 6. Name Edit Box (if editing)
        let cx = (screen_width / 2.0).round();
        let sh = screen_height;
        let is_editing = s.gui.active_tab == GuiTab::EditName;

        if is_editing {
            let edit_box_rect = [cx - 145.0, sh - 76.0, 290.0, 20.0];
            ops.push(HudDrawOp::Fill {
                rect: edit_box_rect,
                argb: 0xFA00_0000,
            });
            ops.push(HudDrawOp::Fill {
                rect: [cx - 145.0, sh - 76.0, 290.0, 1.0],
                argb: 0xFFAA_AAAA,
            });
            ops.push(HudDrawOp::Fill {
                rect: [cx - 145.0, sh - 57.0, 290.0, 1.0],
                argb: 0xFFAA_AAAA,
            });
            ops.push(HudDrawOp::Fill {
                rect: [cx - 145.0, sh - 76.0, 1.0, 20.0],
                argb: 0xFFAA_AAAA,
            });
            ops.push(HudDrawOp::Fill {
                rect: [cx + 144.0, sh - 76.0, 1.0, 20.0],
                argb: 0xFFAA_AAAA,
            });

            let input_text = format!("Name: {}_", s.gui.name_buffer);
            ops.push(HudDrawOp::Text {
                text: input_text,
                at: [cx - 140.0, sh - 70.0],
                rgb: 0x00FF_FFFF,
                shadow: true,
            });
        }

        // 7. Bottom Minecraft Buttons
        let has_selection = !s.waypoints.is_empty();
        let wp_enabled = s
            .waypoints
            .get(s.gui.selected_index)
            .map_or(false, |w| w.enabled);

        // Row 1 Buttons
        let btn1_rect = [cx - 145.0, sh - 52.0, 92.0, 20.0];
        ops.push(HudDrawOp::Button {
            rect: btn1_rect,
            label: if wp_enabled {
                "Vis: ON".into()
            } else {
                "Vis: OFF".into()
            },
            active: has_selection,
            hover: has_selection && is_inside(cursor, btn1_rect),
        });

        let btn2_rect = [cx - 46.0, sh - 52.0, 92.0, 20.0];
        ops.push(HudDrawOp::Button {
            rect: btn2_rect,
            label: if is_editing {
                "Save Name".into()
            } else {
                "Edit Name".into()
            },
            active: has_selection,
            hover: has_selection && is_inside(cursor, btn2_rect),
        });

        let btn3_rect = [cx + 53.0, sh - 52.0, 92.0, 20.0];
        ops.push(HudDrawOp::Button {
            rect: btn3_rect,
            label: "Delete".into(),
            active: has_selection,
            hover: has_selection && is_inside(cursor, btn3_rect),
        });

        // Row 2 Buttons
        let btn4_rect = [cx - 145.0, sh - 28.0, 140.0, 20.0];
        ops.push(HudDrawOp::Button {
            rect: btn4_rect,
            label: "Create Waypoint".into(),
            active: true,
            hover: is_inside(cursor, btn4_rect),
        });

        let btn5_rect = [cx + 5.0, sh - 28.0, 140.0, 20.0];
        ops.push(HudDrawOp::Button {
            rect: btn5_rect,
            label: "Done".into(),
            active: true,
            hover: is_inside(cursor, btn5_rect),
        });
    });
}

/// Periodic tick: decrement remaining ticks and remove expired pings.
pub fn tick() {
    STATE.with_borrow_mut(|s| {
        for ping in &mut s.pings {
            if ping.remaining_ticks > 0 {
                ping.remaining_ticks -= 1;
            }
        }
        s.pings.retain(|p| p.remaining_ticks > 0);
    });
}

/// Ping key pressed.
pub fn on_ping_key(
    playing: bool,
    pressed: bool,
    player: Option<impl Into<PlayerSnapshot>>,
) {
    if !playing || !pressed {
        return;
    }
    let Some(player) = player.map(Into::into) else {
        return;
    };
    let target = raycast_target(
        [player.x, player.y, player.z],
        player.yaw,
        player.pitch,
        1.62,
        32.0, // default quick ping reach
    );
    let ping = Ping::new(
        target[0],
        target[1],
        target[2],
        player.dimension,
        "Ping".to_owned(),
        0x00FF_AA00, // Vibrant orange/amber
        PING_LIFETIME_TICKS,
    );
    add_ping(ping);
}

/// Waypoint key pressed.
pub fn on_waypoint_key(
    playing: bool,
    pressed: bool,
    player: Option<impl Into<PlayerSnapshot>>,
) {
    if !playing || !pressed {
        return;
    }
    let Some(player) = player.map(Into::into) else {
        return;
    };
    let count = STATE.with_borrow(|s| s.waypoints.len() + 1);
    let wp = Waypoint {
        x: player.x,
        y: player.y,
        z: player.z,
        dimension: player.dimension,
        name: format!("Waypoint #{count}"),
        rgb: 0x0033_CCFF, // Cyan
        enabled: true,
    };
    add_waypoint(wp);
}

/// Calculate 2D position on minimap for a target coordinate.
/// Returns relative (dx, dy) in GUI pixels from minimap center, and whether it was clamped.
pub fn project_to_minimap(
    target_x: f64,
    target_z: f64,
    player_x: f64,
    player_z: f64,
    player_yaw: f32,
    rotate_with_player: bool,
    blocks_per_pixel: f32,
    minimap_radius: f32,
) -> (f32, f32, bool) {
    let diff_x = (target_x - player_x) as f32;
    let diff_z = (target_z - player_z) as f32;

    let (mut rx, mut ry) = if !rotate_with_player {
        // North is up (-Z), East is right (+X), South is down (+Z), West is left (-X)
        (diff_x / blocks_per_pixel, diff_z / blocks_per_pixel)
    } else {
        // Match Horizon's internal minimap axes:
        // axes: r = [-c, -s], f = [-s, c]
        // wx = view.x + bpp * (u * r[0] - v * f[0])
        // wz = view.z + bpp * (u * r[1] - v * f[1])
        // Inverting this mapping:
        let (s, c) = (player_yaw as f64).to_radians().sin_cos();
        let r = [-c, -s];
        let f = [-s, c];
        let det = (f[0] * r[1] - r[0] * f[1]) as f32;
        let u = ((-f[1] as f32 * diff_x + f[0] as f32 * diff_z) / det) / blocks_per_pixel;
        let v = ((-r[1] as f32 * diff_x + r[0] as f32 * diff_z) / det) / blocks_per_pixel;
        (u, v)
    };

    let dist = (rx * rx + ry * ry).sqrt();
    let clamped = dist > minimap_radius;
    if clamped && dist > 0.0 {
        rx = (rx / dist) * minimap_radius;
        ry = (ry / dist) * minimap_radius;
    }
    (rx, ry, clamped)
}

/// Project in-world 3D point to screen coordinates.
/// Returns Some([screen_x, screen_y]) if in front of player, or None if behind.
pub fn project_in_world_hud(
    target: [f64; 3],
    player_eye: [f64; 3],
    yaw: f32,
    pitch: f32,
    screen_width: f32,
    screen_height: f32,
    fov_degrees: f32,
) -> Option<[f32; 2]> {
    let dx = (target[0] - player_eye[0]) as f32;
    let dy = (target[1] - player_eye[1]) as f32;
    let dz = (target[2] - player_eye[2]) as f32;

    let (sy, cy) = (yaw as f32).to_radians().sin_cos();
    let (sp, cp) = (pitch as f32).to_radians().sin_cos();

    // In Horizon's camera coordinate system:
    // Right on screen is positive x_cam:
    let x_cam = -(dx * cy + dz * sy);
    // Horizontal forward distance:
    let z_horiz = -dx * sy + dz * cy;

    // Pitch transformation:
    let y_cam = dy * cp + z_horiz * sp;
    let z_forward = z_horiz * cp - dy * sp;

    // Discard points behind the camera:
    if z_forward <= 0.1 {
        return None;
    }

    let aspect = screen_width / screen_height.max(1.0);
    let tan_half_fov = ((fov_degrees * 0.5) as f32).to_radians().tan();

    let ndc_x = x_cam / (z_forward * tan_half_fov * aspect);
    let ndc_y = y_cam / (z_forward * tan_half_fov);

    let sx = (ndc_x + 1.0) * 0.5 * screen_width;
    let sy = (1.0 - ndc_y) * 0.5 * screen_height;

    Some([sx, sy])
}

/// Top-left position of the minimap box given screen dimensions and settings.
pub fn minimap_rect(corner_idx: usize, size: f32, screen_width: f32, screen_height: f32) -> (f32, f32) {
    const MARGIN: f32 = 2.0;
    match corner_idx {
        0 => (MARGIN, MARGIN), // Top Left
        1 => (screen_width - MARGIN - size, MARGIN), // Top Right
        2 => (MARGIN, screen_height - MARGIN - size), // Bottom Left
        _ => (screen_width - MARGIN - size, screen_height - MARGIN - size), // Bottom Right
    }
}

/// Vanilla Minecraft MapColor palette (MapColor.col by id).
pub const MAP_COLORS: [u32; 62] = [
    0, 8368696, 16247203, 13092807, 16711680, 10526975, 10987431, 31744, 16777215, 10791096,
    9923917, 7368816, 4210943, 9402184, 16776437, 14188339, 11685080, 6724056, 15066419, 8375321,
    15892389, 5000268, 10066329, 5013401, 8339378, 3361970, 6704179, 6717235, 10040115, 1644825,
    16445005, 6085589, 4882687, 55610, 8476209, 7340544, 13742497, 10441252, 9787244, 7367818,
    12223780, 6780213, 10505550, 3746083, 8874850, 5725276, 8014168, 4996700, 4993571, 5001770,
    9321518, 2430480, 12398641, 9715553, 6035741, 1474182, 3837580, 5647422, 1356933, 6579300,
    14200723, 8365974,
];

pub fn packed_rgba(id: u8, brightness: u8) -> [u8; 4] {
    let id = usize::from(id);
    if id == 0 || id >= MAP_COLORS.len() {
        return [0, 0, 0, 0];
    }
    let modifier = [180u32, 220, 255, 135][usize::from(brightness & 3)];
    let col = MAP_COLORS[id];
    let ch = |s: u32| ((col >> s & 255) * modifier / 255) as u8;
    [ch(16), ch(8), ch(0), 255]
}

pub fn block_brightness(color: u8, height: i16, north: Option<i16>, depth: u8, parity: i32) -> u8 {
    let parity = f64::from(parity & 1);
    if color == 12 { // Water
        let diff = f64::from(depth) * 0.1 + parity * 0.2;
        return if diff < 0.5 { 2 } else if diff > 0.9 { 0 } else { 1 };
    }
    let north = north.unwrap_or(height);
    let diff = f64::from(height - north) * 4.0 / 5.0 + (parity - 0.5) * 0.4;
    if diff > 0.6 {
        2
    } else if diff < -0.6 {
        0
    } else {
        1
    }
}

pub fn compose_world_map_quadrants(
    dim: &str,
    pan_x: f64,
    pan_z: f64,
    bpp: f32,
    explored: &ExploredWorld,
) -> [Vec<u8>; 4] {
    let mut quad0 = vec![0u8; 128 * 128 * 4];
    let mut quad1 = vec![0u8; 128 * 128 * 4];
    let mut quad2 = vec![0u8; 128 * 128 * 4];
    let mut quad3 = vec![0u8; 128 * 128 * 4];

    let half = 128.0;
    let bpp_f64 = f64::from(bpp);

    for j in 0..256 {
        for i in 0..256 {
            let u = (i as f64 + 0.5 - half) * bpp_f64;
            let v = (j as f64 + 0.5 - half) * bpp_f64;
            let wx = (pan_x + u).floor() as i32;
            let wz = (pan_z + v).floor() as i32;

            let rgba = match explored.get_block(dim, wx, wz) {
                Some((color, height, depth)) if color & 0x7F != 0 => {
                    let north = explored.get_block(dim, wx, wz - 1).map(|c| c.1);
                    let b = block_brightness(color & 0x7F, height, north, depth, wx + wz);
                    packed_rgba(color & 0x7F, b)
                }
                _ => [16, 16, 20, 255], // Fog of War: scuro non esplorato
            };

            let (target, qi, qj) = if j < 128 {
                if i < 128 {
                    (&mut quad0, i, j)
                } else {
                    (&mut quad1, i - 128, j)
                }
            } else {
                if i < 128 {
                    (&mut quad2, i, j - 128)
                } else {
                    (&mut quad3, i - 128, j - 128)
                }
            };

            let o = (qj * 128 + qi) * 4;
            target[o..o + 4].copy_from_slice(&rgba);
        }
    }

    [quad0, quad1, quad2, quad3]
}

pub fn take_world_map_quadrants_to_upload(dimension: &str) -> Option<[Vec<u8>; 4]> {
    STATE.with_borrow_mut(|s| {
        if !s.world_map.is_open {
            return None;
        }

        let bpp = WORLD_MAP_ZOOMS[s.world_map.zoom_index];
        let current_key = (
            s.world_map.pan_x.round() as i32,
            s.world_map.pan_z.round() as i32,
            s.world_map.zoom_index,
            s.explored_world.revision,
        );

        let dirty = s.map_texture_cache.map_or(true, |k| k != current_key);
        if dirty {
            s.map_texture_cache = Some(current_key);
            let quads = compose_world_map_quadrants(
                dimension,
                s.world_map.pan_x,
                s.world_map.pan_z,
                bpp,
                &s.explored_world,
            );
            s.map_quadrants = Some(quads.clone());
            Some(quads)
        } else {
            None
        }
    })
}

/// Operations to draw on HUD.
#[derive(Clone, Debug, PartialEq)]
pub enum HudDrawOp {
    Fill { rect: [f32; 4], argb: u32 },
    Text { text: String, at: [f32; 2], rgb: u32, shadow: bool },
    MenuBackground { rect: [f32; 4] },
    ListBackground { top: f32, bottom: f32 },
    Separators { top: f32, bottom: f32 },
    Button { rect: [f32; 4], label: String, active: bool, hover: bool },
    CenteredText { text: String, y: f32, color: u32 },
    MapTexture { rect: [f32; 4] },
    MapTextureGrid { rect: [f32; 4], slots: [usize; 4] },
    PlayerMarker { center: [f32; 2], yaw: f32 },
}

pub fn draw_world_map(
    screen_width: f32,
    screen_height: f32,
    cursor: (f32, f32),
    player: &PlayerSnapshot,
    ops: &mut Vec<HudDrawOp>,
) {
    STATE.with_borrow_mut(|s| {
        if !s.world_map.is_open {
            return;
        }

        s.world_map.screen_width = screen_width;
        s.world_map.screen_height = screen_height;

        if let Some((_, ref mut ticks)) = s.world_map.message {
            if *ticks > 0 {
                *ticks -= 1;
            } else {
                s.world_map.message = None;
            }
        }

        // 1. Fullscreen Menu Background
        ops.push(HudDrawOp::MenuBackground {
            rect: [0.0, 0.0, screen_width, screen_height],
        });

        // 2. Top Header Bar
        ops.push(HudDrawOp::Fill {
            rect: [0.0, 0.0, screen_width, 28.0],
            argb: 0xDD00_0000,
        });

        ops.push(HudDrawOp::Text {
            text: "World Map".to_string(),
            at: [12.0, 9.0],
            rgb: 0x00FF_FFAA,
            shadow: true,
        });

        let map_size = (screen_width - 24.0).min(screen_height - 62.0).min(512.0);
        let map_x = ((screen_width - map_size) / 2.0).round();
        let map_y = (30.0 + (screen_height - 62.0 - map_size) / 2.0).round();
        let map_rect = [map_x, map_y, map_size, map_size];
        let map_cx = map_x + map_size / 2.0;
        let map_cy = map_y + map_size / 2.0;
        let bpp = WORLD_MAP_ZOOMS[s.world_map.zoom_index];
        let pixels_per_block = map_size / (256.0 * bpp);

        // Cursor coordinates in world blocks (if hovering map)
        if is_inside(cursor, map_rect) {
            let cur_wx = (s.world_map.pan_x + f64::from((cursor.0 - map_cx) / pixels_per_block)).round() as i32;
            let cur_wz = (s.world_map.pan_z + f64::from((cursor.1 - map_cy) / pixels_per_block)).round() as i32;
            ops.push(HudDrawOp::Text {
                text: format!("Cursor: [{}, {}]", cur_wx, cur_wz),
                at: [80.0, 9.0],
                rgb: 0x00AA_FFFF,
                shadow: true,
            });
        }

        // Player Coordinates on right of header
        let p_info = format!(
            "Player: [{}, {}, {}] ({})",
            player.x.floor() as i32,
            player.y.floor() as i32,
            player.z.floor() as i32,
            cardinal_facing(player.yaw)
        );
        ops.push(HudDrawOp::Text {
            text: p_info,
            at: [screen_width - 200.0, 9.0],
            rgb: 0x00DD_DDDD,
            shadow: true,
        });

        // 3. Map Viewport Border
        ops.push(HudDrawOp::Fill {
            rect: [map_x - 2.0, map_y - 2.0, map_size + 4.0, map_size + 4.0],
            argb: 0xFF2A_2A2A,
        });

        // 4. Map Texture Grid (2x2 slots 256x256 HD)
        ops.push(HudDrawOp::MapTextureGrid {
            rect: map_rect,
            slots: [250, 251, 252, 253],
        });

        // 5. Center Crosshair
        ops.push(HudDrawOp::Fill {
            rect: [map_cx - 4.0, map_cy, 9.0, 1.0],
            argb: 0x80FF_FFFF,
        });
        ops.push(HudDrawOp::Fill {
            rect: [map_cx, map_cy - 4.0, 1.0, 9.0],
            argb: 0x80FF_FFFF,
        });

        // 6. Waypoints
        let mut hovered_wp: Option<(String, [f32; 2], u32)> = None;
        for wp in &s.waypoints {
            if wp.dimension != player.dimension || !wp.enabled {
                continue;
            }
            let dx = wp.x - s.world_map.pan_x;
            let dz = wp.z - s.world_map.pan_z;
            let wx_screen = map_cx + (dx as f32) * pixels_per_block;
            let wz_screen = map_cy + (dz as f32) * pixels_per_block;

            if is_inside((wx_screen, wz_screen), map_rect) {
                // Waypoint marker square with border
                ops.push(HudDrawOp::Fill {
                    rect: [wx_screen - 3.0, wz_screen - 3.0, 7.0, 7.0],
                    argb: 0xFF00_0000,
                });
                ops.push(HudDrawOp::Fill {
                    rect: [wx_screen - 2.0, wz_screen - 2.0, 5.0, 5.0],
                    argb: 0xFF00_0000 | (wp.rgb & 0x00FF_FFFF),
                });
                ops.push(HudDrawOp::Fill {
                    rect: [wx_screen, wz_screen, 1.0, 1.0],
                    argb: 0xFFFF_FFFF,
                });

                // Check hover
                if (cursor.0 - wx_screen).abs() <= 6.0 && (cursor.1 - wz_screen).abs() <= 6.0 {
                    let dist = wp.distance_to(player.x, player.y, player.z).round() as i32;
                    let label = format!("{} ({}m) [{}, {}, {}]", wp.name, dist, wp.x as i32, wp.y as i32, wp.z as i32);
                    hovered_wp = Some((label, [cursor.0, cursor.1 - 18.0], wp.rgb));
                }
            }
        }

        // 6b. Active Pings
        for ping in &s.pings {
            if ping.waypoint.dimension != player.dimension {
                continue;
            }
            let ping_dx = ping.waypoint.x - s.world_map.pan_x;
            let ping_dz = ping.waypoint.z - s.world_map.pan_z;
            let ping_sx = map_cx + (ping_dx as f32) * pixels_per_block;
            let ping_sy = map_cy + (ping_dz as f32) * pixels_per_block;
            if is_inside((ping_sx, ping_sy), map_rect) {
                ops.push(HudDrawOp::Fill {
                    rect: [ping_sx - 3.0, ping_sy - 3.0, 7.0, 7.0],
                    argb: 0xFF00_0000,
                });
                ops.push(HudDrawOp::Fill {
                    rect: [ping_sx - 2.0, ping_sy - 2.0, 5.0, 5.0],
                    argb: 0xFFFF_3333,
                });
                ops.push(HudDrawOp::Fill {
                    rect: [ping_sx, ping_sy, 1.0, 1.0],
                    argb: 0xFFFF_FFFF,
                });
            }
        }

        // 7. Player Marker
        let p_dx = player.x - s.world_map.pan_x;
        let p_dz = player.z - s.world_map.pan_z;
        let p_sx = map_cx + (p_dx as f32) * pixels_per_block;
        let p_sy = map_cy + (p_dz as f32) * pixels_per_block;

        if is_inside((p_sx, p_sy), map_rect) {
            ops.push(HudDrawOp::PlayerMarker {
                center: [p_sx, p_sy],
                yaw: player.yaw,
            });
        } else {
            // Player is outside view: draw clamped edge indicator
            let clamped_x = p_sx.clamp(map_x + 6.0, map_x + map_size - 6.0);
            let clamped_y = p_sy.clamp(map_y + 6.0, map_y + map_size - 6.0);
            ops.push(HudDrawOp::Fill {
                rect: [clamped_x - 3.0, clamped_y - 3.0, 7.0, 7.0],
                argb: 0xFFFF_FFFF,
            });
            ops.push(HudDrawOp::Fill {
                rect: [clamped_x - 2.0, clamped_y - 2.0, 5.0, 5.0],
                argb: 0xFFFF_3333,
            });
        }

        // 8. Hovered Waypoint Tooltip
        if let Some((text, at, col)) = hovered_wp {
            let tw = (text.len() as f32) * 6.0 + 8.0;
            let tx = at[0].clamp(map_x + 4.0, map_x + map_size - tw - 4.0);
            ops.push(HudDrawOp::Fill {
                rect: [tx, at[1], tw, 14.0],
                argb: 0xEE11_1111,
            });
            ops.push(HudDrawOp::Text {
                text,
                at: [tx + 4.0, at[1] + 3.0],
                rgb: col,
                shadow: true,
            });
        }

        // 9. Bottom Control Bar
        ops.push(HudDrawOp::Fill {
            rect: [0.0, screen_height - 30.0, screen_width, 30.0],
            argb: 0xDD00_0000,
        });

        let btn_y = screen_height - 26.0;

        // Button 1: Zoom [-]
        let btn_zoom_out = [map_x, btn_y, 48.0, 20.0];
        ops.push(HudDrawOp::Button {
            rect: btn_zoom_out,
            label: "Zoom -".to_string(),
            active: s.world_map.zoom_index + 1 < WORLD_MAP_ZOOMS.len(),
            hover: is_inside(cursor, btn_zoom_out),
        });

        // Button 2: Zoom [+]
        let btn_zoom_in = [map_x + 52.0, btn_y, 48.0, 20.0];
        ops.push(HudDrawOp::Button {
            rect: btn_zoom_in,
            label: "Zoom +".to_string(),
            active: s.world_map.zoom_index > 0,
            hover: is_inside(cursor, btn_zoom_in),
        });

        // Zoom info label
        let zoom_str = format!("Zoom: {:.1}x", 1.0 / bpp);
        ops.push(HudDrawOp::Text {
            text: zoom_str,
            at: [map_x + 106.0, btn_y + 6.0],
            rgb: 0x00FF_FFAA,
            shadow: true,
        });

        // Button 3: Center
        let btn_center = [map_x + 168.0, btn_y, 56.0, 20.0];
        ops.push(HudDrawOp::Button {
            rect: btn_center,
            label: "Center".to_string(),
            active: true,
            hover: is_inside(cursor, btn_center),
        });

        // Controls info helper text
        ops.push(HudDrawOp::Text {
            text: "[B] Add Waypoint".to_string(),
            at: [map_x + 232.0, btn_y + 6.0],
            rgb: 0x00AA_FFAA,
            shadow: true,
        });

        // Notification message if active
        if let Some((ref msg, _)) = s.world_map.message {
            ops.push(HudDrawOp::Text {
                text: msg.clone(),
                at: [map_x + 330.0, btn_y + 6.0],
                rgb: 0x00FF_DD55,
                shadow: true,
            });
        }

        // Button 4: Waypoints
        let btn_waypoints = [map_x + map_size - 144.0, btn_y, 76.0, 20.0];
        ops.push(HudDrawOp::Button {
            rect: btn_waypoints,
            label: "Waypoints".to_string(),
            active: true,
            hover: is_inside(cursor, btn_waypoints),
        });

        // Button 5: Close
        let btn_close = [map_x + map_size - 64.0, btn_y, 64.0, 20.0];
        ops.push(HudDrawOp::Button {
            rect: btn_close,
            label: "Close".to_string(),
            active: true,
            hover: is_inside(cursor, btn_close),
        });
    });
}

/// Draw HUD and Minimap markers.
/// Computes drawing operations into a Vec to avoid multiple mutable closures in native_hud.
pub fn draw_hud(
    screen_width: f32,
    screen_height: f32,
    cursor: (f32, f32),
    player: &PlayerSnapshot,
    settings: &Settings,
    text_width: impl Fn(&str) -> f32,
) -> Vec<HudDrawOp> {
    let mut ops = Vec::new();

    if is_world_map_open() {
        draw_world_map(screen_width, screen_height, cursor, player, &mut ops);
        return ops;
    }

    let (map_size, blocks_per_pixel) = settings.view();
    let (map_x, map_y) = minimap_rect(settings.corner, map_size, screen_width, screen_height);
    let map_center = [map_x + map_size / 2.0, map_y + map_size / 2.0];
    let map_radius = map_size / 2.0 - 2.0;

    STATE.with_borrow(|s| {
        // 1. Draw Active Pings
        for ping in &s.pings {
            if ping.waypoint.dimension != player.dimension {
                continue;
            }
            let dist = ping.waypoint.distance_to(player.x, player.y, player.z);
            let alpha_byte = ((ping.alpha() * 255.0) as u32).clamp(0, 255);
            let color_argb = (alpha_byte << 24) | (ping.waypoint.rgb & 0x00FF_FFFF);

            // Minimap marker
            if settings.show_minimap_pings {
                let (mx, my, _clamped) = project_to_minimap(
                    ping.waypoint.x,
                    ping.waypoint.z,
                    player.x,
                    player.z,
                    player.yaw,
                    settings.rotate,
                    blocks_per_pixel,
                    map_radius,
                );
                let px = map_center[0] + mx;
                let py = map_center[1] + my;
                ops.push(HudDrawOp::Fill {
                    rect: [px - 2.0, py - 2.0, 4.0, 4.0],
                    argb: color_argb,
                });
            }

            // In-World HUD marker
            if settings.show_hud_pings {
                if let Some([sx, sy]) = project_in_world_hud(
                    [ping.waypoint.x, ping.waypoint.y, ping.waypoint.z],
                    [player.eye_x, player.eye_y, player.eye_z],
                    player.yaw,
                    player.pitch,
                    screen_width,
                    screen_height,
                    player.fov,
                ) {
                    ops.push(HudDrawOp::Fill {
                        rect: [sx - 3.0, sy - 3.0, 6.0, 6.0],
                        argb: color_argb,
                    });
                    let label = format!("[Ping: {}m]", dist.round() as i32);
                    let lw = text_width(&label);
                    ops.push(HudDrawOp::Fill {
                        rect: [sx - lw / 2.0 - 2.0, sy + 5.0, lw + 4.0, 10.0],
                        argb: 0x7800_0000,
                    });
                    ops.push(HudDrawOp::Text {
                        text: label,
                        at: [sx - lw / 2.0, sy + 6.0],
                        rgb: ping.waypoint.rgb,
                        shadow: true,
                    });
                }
            }
        }

        // 2. Draw Waypoints
        for wp in &s.waypoints {
            if !wp.enabled || wp.dimension != player.dimension {
                continue;
            }
            let dist = wp.distance_to(player.x, player.y, player.z);
            let color_argb = 0xFF00_0000 | (wp.rgb & 0x00FF_FFFF);

            // Minimap marker
            if settings.show_minimap_pings {
                let (mx, my, _clamped) = project_to_minimap(
                    wp.x,
                    wp.z,
                    player.x,
                    player.z,
                    player.yaw,
                    settings.rotate,
                    blocks_per_pixel,
                    map_radius,
                );
                let px = map_center[0] + mx;
                let py = map_center[1] + my;
                ops.push(HudDrawOp::Fill {
                    rect: [px - 2.0, py - 2.0, 4.0, 4.0],
                    argb: color_argb,
                });
            }

            // In-World HUD marker & Beacon Beam
            if settings.show_hud_pings {
                let p_eye = [player.eye_x, player.eye_y, player.eye_z];

                // Vertical Beacon Beam
                if settings.show_beacons {
                    let beam_bottom_y = wp.y;
                    let beam_top_y = (wp.y + 250.0).min(320.0);
                    let proj_bot = project_in_world_hud(
                        [wp.x, beam_bottom_y, wp.z],
                        p_eye,
                        player.yaw,
                        player.pitch,
                        screen_width,
                        screen_height,
                        player.fov,
                    );
                    let proj_top = project_in_world_hud(
                        [wp.x, beam_top_y, wp.z],
                        p_eye,
                        player.yaw,
                        player.pitch,
                        screen_width,
                        screen_height,
                        player.fov,
                    );

                    if let (Some([bx_bot, by_bot]), Some([_bx_top, by_top])) = (proj_bot, proj_top) {
                        let y_min = by_top.min(by_bot).max(0.0);
                        let y_max = by_top.max(by_bot).min(screen_height);
                        let beam_h = y_max - y_min;
                        if beam_h > 1.0 {
                            // Beam width scales slightly with proximity (1px to 4px)
                            let beam_w = (40.0 / (dist as f32).max(10.0)).clamp(2.0, 5.0);
                            let beam_alpha = 0x5500_0000; // Semi-transparent glow
                            let beam_argb = beam_alpha | (wp.rgb & 0x00FF_FFFF);
                            ops.push(HudDrawOp::Fill {
                                rect: [bx_bot - beam_w * 0.5, y_min, beam_w, beam_h],
                                argb: beam_argb,
                            });
                            // Central bright core
                            let core_w = (beam_w * 0.5).max(1.0);
                            let core_argb = 0xAA00_0000 | (wp.rgb & 0x00FF_FFFF);
                            ops.push(HudDrawOp::Fill {
                                rect: [bx_bot - core_w * 0.5, y_min, core_w, beam_h],
                                argb: core_argb,
                            });
                        }
                    }
                }

                if let Some([sx, sy]) = project_in_world_hud(
                    [wp.x, wp.y, wp.z],
                    p_eye,
                    player.yaw,
                    player.pitch,
                    screen_width,
                    screen_height,
                    player.fov,
                ) {
                    ops.push(HudDrawOp::Fill {
                        rect: [sx - 3.0, sy - 3.0, 6.0, 6.0],
                        argb: color_argb,
                    });
                    let label = format!("{} ({}m)", wp.name, dist.round() as i32);
                    let lw = text_width(&label);
                    ops.push(HudDrawOp::Fill {
                        rect: [sx - lw / 2.0 - 2.0, sy + 5.0, lw + 4.0, 10.0],
                        argb: 0x7800_0000,
                    });
                    ops.push(HudDrawOp::Text {
                        text: label,
                        at: [sx - lw / 2.0, sy + 6.0],
                        rgb: wp.rgb,
                        shadow: true,
                    });
                }
            }
        }
    });

    // 3. Quick Info Overlay (Coordinates & Facing Direction)
    if settings.show_coords {
        let (bx, by, bz) = (player.x.floor() as i32, player.y.floor() as i32, player.z.floor() as i32);
        let coords_text = format!("XYZ: {}, {}, {}", bx, by, bz);
        let facing_text = format!("Facing: {}", cardinal_facing(player.yaw));

        let cw = text_width(&coords_text);
        let fw = text_width(&facing_text);
        let box_w = cw.max(fw) + 6.0;
        let box_h = 21.0;

        // Position below or above the minimap
        let is_top = settings.corner <= 1;
        let is_left = settings.corner == 0 || settings.corner == 2;

        let ox = if is_left {
            map_x
        } else {
            map_x + map_size - box_w
        };
        let oy = if is_top {
            map_y + map_size + 3.0
        } else {
            map_y - box_h - 3.0
        };

        ops.push(HudDrawOp::Fill {
            rect: [ox, oy, box_w, box_h],
            argb: 0x7800_0000,
        });
        ops.push(HudDrawOp::Text {
            text: coords_text,
            at: [ox + 3.0, oy + 2.0],
            rgb: 0x00FF_FFFF, // White
            shadow: true,
        });
        ops.push(HudDrawOp::Text {
            text: facing_text,
            at: [ox + 3.0, oy + 11.0],
            rgb: 0x00AA_AAAA, // Muted grey
            shadow: true,
        });
    }

    // Waypoint Manager GUI Modal (rendered on top when open)
    draw_waypoints_gui(screen_width, screen_height, cursor, player, &mut ops);

    ops
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_waypoint_serialization() {
        let wp1 = Waypoint {
            x: 123.45,
            y: 64.0,
            z: -789.12,
            dimension: "minecraft:overworld".into(),
            name: "Village Base".into(),
            rgb: 0x33CCFF,
            enabled: true,
        };
        let s = wp1.serialize();
        let wp2 = Waypoint::deserialize(&s).expect("deserialization failed");
        assert_eq!(wp1.name, wp2.name);
        assert!((wp1.x - wp2.x).abs() < 0.01);
        assert!((wp1.y - wp2.y).abs() < 0.01);
        assert!((wp1.z - wp2.z).abs() < 0.01);
        assert_eq!(wp1.dimension, wp2.dimension);
        assert_eq!(wp1.rgb, wp2.rgb);
        assert_eq!(wp1.enabled, wp2.enabled);
    }

    #[test]
    fn test_settings_read_and_write() {
        let none = |_: &str| None;
        let s = Settings::read(none);
        assert_eq!(
            (s.corner, s.size, s.round, s.zoom, s.rotate, s.show_hud_pings, s.show_minimap_pings, s.show_coords, s.show_beacons),
            (1, 1, false, 1, false, true, true, true, true)
        );

        let files = [
            ("corner", "bottom_left"),
            ("size", "128\n"),
            ("shape", "round"),
            ("zoom", "0.5"),
            ("rotate", "true"),
            ("hud_pings", "false"),
            ("minimap_pings", "true"),
            ("show_coords", "true"),
            ("show_beacons", "true"),
        ];
        let get = |k: &str| files.iter().find(|f| f.0 == k).map(|f| f.1.to_owned());
        let mut s = Settings::read(get);
        assert_eq!(s.view(), (128.0, 0.5));
        assert!(!s.show_hud_pings);
        assert!(s.show_minimap_pings);
        assert!(s.show_coords);
        assert!(s.show_beacons);

        s.click(5); // toggle hud_pings
        assert!(s.show_hud_pings);
        s.click(7); // toggle show_coords
        assert!(!s.show_coords);
        s.click(8); // toggle show_beacons
        assert!(!s.show_beacons);
        let stored = s.stored();
        assert_eq!(stored[5], ("hud_pings", "true".to_owned()));
        assert_eq!(stored[7], ("show_coords", "false".to_owned()));
        assert_eq!(stored[8], ("show_beacons", "false".to_owned()));
    }

    #[test]
    fn test_cardinal_facing() {
        assert_eq!(cardinal_facing(0.0), "South (+Z)");
        assert_eq!(cardinal_facing(90.0), "West (-X)");
        assert_eq!(cardinal_facing(180.0), "North (-Z)");
        assert_eq!(cardinal_facing(-90.0), "East (+X)");
        assert_eq!(cardinal_facing(270.0), "East (+X)");
    }

    #[test]
    fn test_waypoint_distance() {
        let wp = Waypoint {
            x: 0.0,
            y: 64.0,
            z: 0.0,
            dimension: "minecraft:overworld".into(),
            name: "Spawn".into(),
            rgb: 0xFF_FFFF,
            enabled: true,
        };
        let dist = wp.distance_to(3.0, 64.0, 4.0);
        assert!((dist - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_ping_tick_and_fade() {
        let mut ping = Ping::new(
            10.0,
            64.0,
            20.0,
            "minecraft:overworld".into(),
            "Ping".into(),
            0xFFAA00,
            200,
        );
        assert_eq!(ping.alpha(), 1.0);
        ping.remaining_ticks = 100;
        assert!((ping.alpha() - 0.5).abs() < 1e-4);
        ping.remaining_ticks = 0;
        assert_eq!(ping.alpha(), 0.0);
    }

    #[test]
    fn test_view_vector() {
        // Yaw 0 faces south (+Z), Pitch 0 is horizontal
        let dir = view_vector(0.0, 0.0);
        assert!((dir[0]).abs() < 1e-6);
        assert!((dir[1]).abs() < 1e-6);
        assert!((dir[2] - 1.0).abs() < 1e-6);

        // Yaw 90 faces west (-X)
        let dir_west = view_vector(90.0, 0.0);
        assert!((dir_west[0] + 1.0).abs() < 1e-6);
        assert!((dir_west[1]).abs() < 1e-6);
        assert!((dir_west[2]).abs() < 1e-6);
    }

    #[test]
    fn test_raycast_target() {
        let player_pos = [100.0, 70.0, 100.0];
        let target = raycast_target(player_pos, 0.0, 0.0, 1.62, 10.0);
        assert!((target[0] - 100.0).abs() < 1e-6);
        assert!((target[1] - 71.62).abs() < 1e-6);
        assert!((target[2] - 110.0).abs() < 1e-6);
    }

    #[test]
    fn test_project_to_minimap() {
        // Player at (0, 0), Target at (10, 0) (10 blocks East = +X)
        // blocks_per_pixel = 1.0, radius = 50.0, not rotated
        let (rx, ry, clamped) = project_to_minimap(10.0, 0.0, 0.0, 0.0, 0.0, false, 1.0, 50.0);
        assert_eq!(rx, 10.0);
        assert_eq!(ry, 0.0);
        assert!(!clamped);

        // Target at (100, 0) should be clamped to radius 50.0
        let (rx_clamped, _, is_clamped) =
            project_to_minimap(100.0, 0.0, 0.0, 0.0, 0.0, false, 1.0, 50.0);
        assert_eq!(rx_clamped, 50.0);
        assert!(is_clamped);
    }

    #[test]
    fn test_project_in_world_hud() {
        let target = [0.0, 65.0, 10.0];
        let player_eye = [0.0, 65.0, 0.0];
        // Facing South (+Z), pitch 0. Target is straight ahead.
        let proj = project_in_world_hud(target, player_eye, 0.0, 0.0, 800.0, 600.0, 70.0);
        assert!(proj.is_some());
        let [sx, sy] = proj.unwrap();
        // Should be at screen center (400, 300)
        assert!((sx - 400.0).abs() < 1e-2);
        assert!((sy - 300.0).abs() < 1e-2);

        // Target to the West (-X): facing South, West is to the RIGHT (sx > 400)
        let target_right = [-5.0, 65.0, 10.0];
        let proj_right =
            project_in_world_hud(target_right, player_eye, 0.0, 0.0, 800.0, 600.0, 70.0).unwrap();
        assert!(proj_right[0] > 400.0);

        // Target behind player
        let target_behind = [0.0, 65.0, -10.0];
        let proj_behind =
            project_in_world_hud(target_behind, player_eye, 0.0, 0.0, 800.0, 600.0, 70.0);
        assert!(proj_behind.is_none());
    }

    #[test]
    fn test_settings_page_buttons_and_click() {
        STATE.with_borrow_mut(|s| {
            s.waypoints = vec![Waypoint {
                name: "Base".to_string(),
                x: 10.0,
                y: 64.0,
                z: 20.0,
                dimension: "minecraft:overworld".to_string(),
                rgb: 0x0033_CCFF, // Cyan
                enabled: true,
            }];
        });

        let dummy_get = |_k: &str| None;
        let buttons = settings_page_buttons(dummy_get);
        assert!(buttons.iter().any(|b| b.contains("WAYPOINTS LIST (1)")));
        assert!(buttons.iter().any(|b| b.contains("[VISIBLE] Base (10, 64, 20) [Cyan]")));

        let base_labels_len = settings(dummy_get).labels().len();
        // Index for first waypoint button: base_labels_len + 1 (0 is header)
        let wp_btn_idx = base_labels_len + 1;
        
        // Clicking should cycle color to Lime
        on_settings_click(wp_btn_idx, dummy_get);
        STATE.with_borrow(|s| {
            assert_eq!(color_name(s.waypoints[0].rgb), "Lime");
            assert!(s.waypoints[0].enabled);
        });

        // Test delete last button
        let delete_last_idx = base_labels_len + 2;
        on_settings_click(delete_last_idx, dummy_get);
        STATE.with_borrow(|s| {
            assert!(s.waypoints.is_empty());
        });
    }

    #[test]
    fn test_in_game_waypoints_gui() {
        // Initialize state
        STATE.with_borrow_mut(|s| {
            s.waypoints = vec![Waypoint {
                name: "Home".to_string(),
                x: 100.0,
                y: 64.0,
                z: -200.0,
                dimension: "minecraft:overworld".to_string(),
                rgb: 0x0033_CCFF, // Cyan
                enabled: true,
            }];
            do_close_gui(&mut s.gui);
        });

        // 1. Toggle GUI open
        assert!(!is_gui_open());
        toggle_gui(None, false, 0.0, 0.0);
        assert!(is_gui_open());
        STATE.with_borrow(|s| {
            assert_eq!(s.gui.selected_index, 0);
            assert_eq!(s.gui.name_buffer, "Home");
            assert_eq!(s.gui.active_tab, GuiTab::List);
        });

        // 2. Switch tab to EditName (press 'e')
        on_gui_key("key.keyboard.e", true, "key.keyboard.u", None);
        STATE.with_borrow(|s| {
            assert_eq!(s.gui.active_tab, GuiTab::EditName);
        });

        // 3. Edit name: backspace, type '2', enter
        on_gui_key("key.keyboard.space", true, "key.keyboard.u", None);
        on_gui_key("key.keyboard.2", true, "key.keyboard.u", None);
        on_gui_key("key.keyboard.enter", true, "key.keyboard.u", None);
        STATE.with_borrow(|s| {
            assert_eq!(s.gui.active_tab, GuiTab::List);
            assert_eq!(s.waypoints[0].name, "Home 2");
            assert!(s.waypoints_dirty);
        });

        // 4. Color grid (press 'c')
        on_gui_key("key.keyboard.c", true, "key.keyboard.u", None);
        STATE.with_borrow(|s| {
            assert_eq!(s.gui.active_tab, GuiTab::ColorGrid);
            assert_eq!(s.gui.color_cursor, 0); // Cyan
        });

        // Move right in grid (Lime) and pick
        on_gui_key("key.keyboard.right", true, "key.keyboard.u", None);
        STATE.with_borrow(|s| {
            assert_eq!(s.gui.color_cursor, 1); // Lime
        });
        on_gui_key("key.keyboard.enter", true, "key.keyboard.u", None);
        STATE.with_borrow(|s| {
            assert_eq!(s.gui.active_tab, GuiTab::List);
            assert_eq!(s.waypoints[0].rgb, COLOR_PALETTE[1].1);
        });

        // 5. Toggle visibility (press 'v')
        on_gui_key("key.keyboard.v", true, "key.keyboard.u", None);
        STATE.with_borrow(|s| {
            assert!(!s.waypoints[0].enabled);
        });

        // 6. Close GUI (press 'u')
        on_gui_key("key.keyboard.u", true, "key.keyboard.u", None);
        assert!(!is_gui_open());

        // 7. Test Mouse Click Interactions
        toggle_gui(None, false, 0.0, 0.0);
        assert!(is_gui_open());
        // Set dimensions for layout
        STATE.with_borrow_mut(|s| {
            s.gui.screen_width = 800.0;
            s.gui.screen_height = 600.0;
        });

        // Click color swatch 2 (Amber) at y = 30.0
        let pal_x = ((800.0_f32 - 192.0) / 2.0).round();
        let swatch_amber_x = pal_x + 2.0 * 24.0 + 5.0;
        let closed = on_gui_click(swatch_amber_x, 35.0, None);
        assert!(!closed);
        STATE.with_borrow(|s| {
            assert_eq!(s.waypoints[0].rgb, COLOR_PALETTE[2].1);
        });

        // Click Done button: cx + 10.0, sh - 20.0
        let done_closed = on_gui_click(410.0, 580.0, None);
        assert!(done_closed);
        assert!(!is_gui_open());

        // 8. Test mouse suppression & frozen look preservation
        toggle_gui(None, false, 45.0, -15.0);
        assert!(is_gui_open());
        assert_eq!(get_frozen_look(), Some((45.0, -15.0)));
        close_gui();
        assert!(!is_gui_open());
        assert_eq!(take_suppress_mouse_look(), Some((45.0, -15.0)));
        assert_eq!(take_suppress_mouse_look(), Some((45.0, -15.0)));
        assert_eq!(take_suppress_mouse_look(), Some((45.0, -15.0)));
        assert_eq!(take_suppress_mouse_look(), None);
    }

    #[test]
    fn test_world_map_interactions() {
        let snapshot = PlayerSnapshot {
            x: 500.0,
            y: 64.0,
            z: -1200.0,
            eye_x: 500.0,
            eye_y: 65.62,
            eye_z: -1200.0,
            yaw: 90.0,
            pitch: 0.0,
            fov: 70.0,
            dimension: "minecraft:overworld".into(),
        };

        // 1. Toggle open
        assert!(!is_world_map_open());
        toggle_world_map(Some(snapshot.clone()), false, 90.0, 0.0);
        assert!(is_world_map_open());
        assert_eq!(get_world_map_view(), Some((500.0, -1200.0, 1.0)));

        // 2. Zoom in and out
        world_map_zoom_in();
        assert_eq!(get_world_map_view(), Some((500.0, -1200.0, 0.5)));
        world_map_zoom_in(); // Clamped at min
        assert_eq!(get_world_map_view(), Some((500.0, -1200.0, 0.5)));
        world_map_zoom_out();
        assert_eq!(get_world_map_view(), Some((500.0, -1200.0, 1.0)));
        world_map_zoom_out();
        assert_eq!(get_world_map_view(), Some((500.0, -1200.0, 2.0)));
        world_map_zoom_out();
        assert_eq!(get_world_map_view(), Some((500.0, -1200.0, 4.0)));
        world_map_zoom_out(); // Clamped at max
        assert_eq!(get_world_map_view(), Some((500.0, -1200.0, 4.0)));

        // Reset to 1.0 bpp
        world_map_zoom_in();
        world_map_zoom_in();
        assert_eq!(get_world_map_view(), Some((500.0, -1200.0, 1.0)));

        // 3. Pan with keys
        on_world_map_key("key.keyboard.w", true, "key.keyboard.m", "key.keyboard.u", Some(snapshot.clone()));
        assert_eq!(get_world_map_view(), Some((500.0, -1232.0, 1.0)));
        on_world_map_key("key.keyboard.d", true, "key.keyboard.m", "key.keyboard.u", Some(snapshot.clone()));
        assert_eq!(get_world_map_view(), Some((532.0, -1232.0, 1.0)));

        // 4. Center on player
        world_map_center(Some(snapshot.clone()));
        assert_eq!(get_world_map_view(), Some((500.0, -1200.0, 1.0)));

        // 5. Test mouse drag
        STATE.with_borrow_mut(|s| {
            s.world_map.screen_width = 800.0;
            s.world_map.screen_height = 600.0;
        });
        start_world_map_drag((400.0, 300.0));
        // Drag mouse 20px right, 10px down
        update_world_map_drag((420.0, 310.0));
        end_world_map_drag();
        // Since mouse moved right/down, map pan should move left/up (negative dx/dz)
        STATE.with_borrow(|s| {
            assert!(s.world_map.pan_x < 500.0);
            assert!(s.world_map.pan_z < -1200.0);
        });

        // 6. Right-click does NOT create waypoint anymore (mouse creation disabled)
        let right_clicked = on_world_map_right_click(400.0, 300.0, Some(snapshot.clone()));
        assert!(!right_clicked);

        // 6b. Dedicated waypoint key creates waypoint
        let created = create_world_map_waypoint(400.0, 300.0, Some(snapshot.clone()));
        assert!(created);
        STATE.with_borrow(|s| {
            assert!(!s.waypoints.is_empty());
        });

        // 7. Draw HUD while World Map open produces World Map ops
        let settings = Settings::read(|_| None);
        let ops = draw_hud(800.0, 600.0, (400.0, 300.0), &snapshot, &settings, |_| 10.0);
        assert!(ops.iter().any(|op| matches!(op, HudDrawOp::MapTextureGrid { .. })));
        assert!(ops.iter().any(|op| matches!(op, HudDrawOp::PlayerMarker { .. })));

        // 8. Close with key
        let act = on_world_map_key("key.keyboard.m", true, "key.keyboard.m", "key.keyboard.u", Some(snapshot.clone()));
        assert_eq!(act, WorldMapKeyAction::Close);
        assert!(!is_world_map_open());
        assert_eq!(take_suppress_mouse_look(), Some((90.0, 0.0)));
    }

    #[test]
    fn test_explored_world_cache_and_fog_of_war() {
        let mut world = ExploredWorld::default();
        let dim = "minecraft:overworld";

        // Unexplored chunk block
        assert_eq!(world.get_block(dim, 100, 200), None);

        // Record a chunk
        let mut color = [0u8; 256];
        let mut height = [64i16; 256];
        let depth = [0u8; 256];
        color[0] = 1; // Grass
        height[0] = 70;
        world.record_chunk(dim, 0, 0, color, height, depth);

        assert_eq!(world.get_block(dim, 0, 0), Some((1, 70, 0)));
        assert_eq!(world.get_block(dim, 16, 0), None); // Neighbor chunk unexplored

        // Serialization & Deserialization
        let bytes = world.save_to_bytes();
        let loaded = ExploredWorld::load_from_bytes(&bytes).expect("Failed to load ExploredWorld");
        assert_eq!(loaded.get_block(dim, 0, 0), Some((1, 70, 0)));
        assert_eq!(loaded.get_block(dim, 16, 0), None);

        // Quadrants composition includes block and fog of war
        let quads = compose_world_map_quadrants(dim, 0.0, 0.0, 1.0, &loaded);
        assert_eq!(quads.len(), 4);
        assert_eq!(quads[0].len(), 128 * 128 * 4);
    }

    #[test]
    fn test_consume_escape_and_1_to_1_drag_scale() {
        assert!(!take_consume_escape());
        request_consume_escape();
        assert!(take_consume_escape());
        assert!(!take_consume_escape());

        STATE.with_borrow_mut(|s| {
            s.world_map.screen_width = 800.0;
            s.world_map.screen_height = 600.0;
            s.world_map.zoom_index = 1; // bpp = 1.0
            s.world_map.pan_x = 500.0;
            s.world_map.pan_z = -1200.0;
        });

        // Map size = (800-24).min(600-62).min(512) = 512.0
        // pixels_per_block = 512.0 / (256.0 * 1.0) = 2.0 px/block
        start_world_map_drag((400.0, 300.0));
        update_world_map_drag((420.0, 310.0));
        end_world_map_drag();

        // Moving mouse +20px X, +10px Y should move pan by -10 blocks X, -5 blocks Z
        STATE.with_borrow(|s| {
            assert_eq!(s.world_map.pan_x, 490.0);
            assert_eq!(s.world_map.pan_z, -1205.0);
        });
    }
}
