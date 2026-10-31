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
}

#[derive(Default)]
pub struct RuntimeState {
    pub settings: Option<Settings>,
    pub pings: Vec<Ping>,
    pub waypoints: Vec<Waypoint>,
    pub waypoints_loaded: bool,
    pub waypoints_dirty: bool,
    pub gui: WaypointsGuiState,
}

thread_local! {
    static STATE: RefCell<RuntimeState> = RefCell::new(RuntimeState::default());
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

pub fn close_gui() {
    STATE.with_borrow_mut(|s| s.gui.is_open = false);
}

pub fn toggle_gui(player: Option<PlayerSnapshot>) {
    let _ = player;
    STATE.with_borrow_mut(|s| {
        s.gui.is_open = !s.gui.is_open;
        if s.gui.is_open {
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
    });
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
                s.gui.is_open = false;
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

pub fn draw_waypoints_gui(
    screen_width: f32,
    screen_height: f32,
    ops: &mut Vec<HudDrawOp>,
) {
    STATE.with_borrow(|s| {
        if !s.gui.is_open {
            return;
        }

        let modal_w = 400.0_f32.min(screen_width - 16.0);
        let modal_h = 220.0_f32.min(screen_height - 16.0);
        let mx = ((screen_width - modal_w) / 2.0).round();
        let my = ((screen_height - modal_h) / 2.0).round();

        // Dark dim backdrop
        ops.push(HudDrawOp::Fill {
            rect: [0.0, 0.0, screen_width, screen_height],
            argb: 0x8800_0000,
        });

        // Main modal container
        ops.push(HudDrawOp::Fill {
            rect: [mx, my, modal_w, modal_h],
            argb: 0xF012_1722,
        });

        // Frame borders
        ops.push(HudDrawOp::Fill {
            rect: [mx, my, modal_w, 1.0],
            argb: 0xFF33_CCFF,
        });
        ops.push(HudDrawOp::Fill {
            rect: [mx, my + modal_h - 1.0, modal_w, 1.0],
            argb: 0xFF33_CCFF,
        });
        ops.push(HudDrawOp::Fill {
            rect: [mx, my, 1.0, modal_h],
            argb: 0xFF33_CCFF,
        });
        ops.push(HudDrawOp::Fill {
            rect: [mx + modal_w - 1.0, my, 1.0, modal_h],
            argb: 0xFF33_CCFF,
        });

        // Header bar
        ops.push(HudDrawOp::Fill {
            rect: [mx + 1.0, my + 1.0, modal_w - 2.0, 22.0],
            argb: 0xFF1B_2436,
        });
        ops.push(HudDrawOp::Text {
            text: "WAYPOINTS MANAGER".to_string(),
            at: [mx + 10.0, my + 6.0],
            rgb: 0x0033_CCFF,
            shadow: true,
        });
        ops.push(HudDrawOp::Text {
            text: "[Tab] Switch  [U]/[Esc] Close".to_string(),
            at: [mx + modal_w - 170.0, my + 6.0],
            rgb: 0x0099_AABB,
            shadow: true,
        });

        // Vertical divider
        let left_w = 185.0_f32.min(modal_w / 2.0 - 5.0);
        let div_x = mx + left_w + 10.0;
        ops.push(HudDrawOp::Fill {
            rect: [div_x, my + 24.0, 1.0, modal_h - 26.0],
            argb: 0xFF2A_364F,
        });

        // Left Pane: Waypoints list
        ops.push(HudDrawOp::Text {
            text: format!("LIST ({})", s.waypoints.len()),
            at: [mx + 10.0, my + 28.0],
            rgb: 0x00FF_AA00,
            shadow: true,
        });

        let max_visible = 6;
        let start_idx = s.gui.scroll_offset;
        let end_idx = (start_idx + max_visible).min(s.waypoints.len());

        if s.waypoints.is_empty() {
            ops.push(HudDrawOp::Text {
                text: "No waypoints yet.".to_string(),
                at: [mx + 12.0, my + 60.0],
                rgb: 0x0088_99AA,
                shadow: true,
            });
            ops.push(HudDrawOp::Text {
                text: "Press [N] to add one!".to_string(),
                at: [mx + 12.0, my + 76.0],
                rgb: 0x0055_FF55,
                shadow: true,
            });
        } else {
            for i in start_idx..end_idx {
                let wp = &s.waypoints[i];
                let item_y = my + 44.0 + ((i - start_idx) as f32) * 22.0;

                // Selection highlight
                if i == s.gui.selected_index {
                    let hl_col = if s.gui.active_tab == GuiTab::List {
                        0x6033_CCFF
                    } else {
                        0x3033_CCFF
                    };
                    ops.push(HudDrawOp::Fill {
                        rect: [mx + 8.0, item_y, left_w, 20.0],
                        argb: hl_col,
                    });
                }

                // Visibility pill
                let vis_text = if wp.enabled { "[V]" } else { "[H]" };
                let vis_col = if wp.enabled { 0x0055_FF55 } else { 0x00FF_5555 };
                ops.push(HudDrawOp::Text {
                    text: vis_text.to_string(),
                    at: [mx + 12.0, item_y + 4.0],
                    rgb: vis_col,
                    shadow: true,
                });

                // Color badge
                ops.push(HudDrawOp::Fill {
                    rect: [mx + 34.0, item_y + 5.0, 9.0, 9.0],
                    argb: 0xFF00_0000 | wp.rgb,
                });

                // Waypoint Name (truncated to 13 chars if needed)
                let display_name: String = wp.name.chars().take(13).collect();
                ops.push(HudDrawOp::Text {
                    text: display_name,
                    at: [mx + 48.0, item_y + 4.0],
                    rgb: if i == s.gui.selected_index {
                        0x00FF_FFFF
                    } else {
                        0x00CC_DDEE
                    },
                    shadow: true,
                });
            }
        }

        // Left pane footer
        ops.push(HudDrawOp::Text {
            text: "[N] Add  [D] Del  [V] Vis".to_string(),
            at: [mx + 10.0, my + modal_h - 18.0],
            rgb: 0x0088_99AA,
            shadow: true,
        });

        // Right Pane: Editor & Color Grid
        let right_x = div_x + 10.0;
        let right_w = modal_w - left_w - 25.0;

        ops.push(HudDrawOp::Text {
            text: "DETAILS & EDIT".to_string(),
            at: [right_x, my + 28.0],
            rgb: 0x00FF_AA00,
            shadow: true,
        });

        if let Some(wp) = s.waypoints.get(s.gui.selected_index) {
            // 1. Name Input Box
            ops.push(HudDrawOp::Text {
                text: "Name: (Press [E] to edit)".to_string(),
                at: [right_x, my + 44.0],
                rgb: 0x00BB_CCDD,
                shadow: true,
            });

            let box_y = my + 56.0;
            let is_editing_name = s.gui.active_tab == GuiTab::EditName;
            let box_bg = if is_editing_name {
                0x8022_3858
            } else {
                0x8018_2030
            };
            let box_border = if is_editing_name {
                0xFF33_CCFF
            } else {
                0xFF3A_4760
            };

            ops.push(HudDrawOp::Fill {
                rect: [right_x, box_y, right_w, 18.0],
                argb: box_bg,
            });
            ops.push(HudDrawOp::Fill {
                rect: [right_x, box_y, right_w, 1.0],
                argb: box_border,
            });
            ops.push(HudDrawOp::Fill {
                rect: [right_x, box_y + 17.0, right_w, 1.0],
                argb: box_border,
            });
            ops.push(HudDrawOp::Fill {
                rect: [right_x, box_y, 1.0, 18.0],
                argb: box_border,
            });
            ops.push(HudDrawOp::Fill {
                rect: [right_x + right_w - 1.0, box_y, 1.0, 18.0],
                argb: box_border,
            });

            let input_text = if is_editing_name {
                format!("{}_", s.gui.name_buffer)
            } else {
                wp.name.clone()
            };
            ops.push(HudDrawOp::Text {
                text: input_text,
                at: [right_x + 6.0, box_y + 4.0],
                rgb: if is_editing_name {
                    0x0055_FFFF
                } else {
                    0x00FF_FFFF
                },
                shadow: true,
            });

            // 2. Position
            ops.push(HudDrawOp::Text {
                text: format!("Pos: {:.0}, {:.0}, {:.0}", wp.x, wp.y, wp.z),
                at: [right_x, my + 80.0],
                rgb: 0x0099_AABB,
                shadow: true,
            });

            // 3. Color Grid
            ops.push(HudDrawOp::Text {
                text: "Color Palette: (Press [C] or Grid)".to_string(),
                at: [right_x, my + 94.0],
                rgb: 0x00BB_CCDD,
                shadow: true,
            });

            let grid_start_x = right_x;
            let grid_start_y = my + 107.0;
            let swatch_w = 38.0;
            let swatch_h = 16.0;

            for (idx, (_cname, rgb)) in COLOR_PALETTE.iter().enumerate() {
                let row = (idx / 4) as f32;
                let col = (idx % 4) as f32;
                let bx = grid_start_x + col * 44.0;
                let by = grid_start_y + row * 21.0;

                // Color swatch
                ops.push(HudDrawOp::Fill {
                    rect: [bx, by, swatch_w, swatch_h],
                    argb: 0xFF00_0000 | rgb,
                });

                // Active color indicator
                if (*rgb & 0x00FF_FFFF) == (wp.rgb & 0x00FF_FFFF) {
                    ops.push(HudDrawOp::Fill {
                        rect: [
                            bx + swatch_w / 2.0 - 2.0,
                            by + swatch_h / 2.0 - 2.0,
                            4.0,
                            4.0,
                        ],
                        argb: 0xFF00_0000,
                    });
                }

                // Grid cursor highlight
                if s.gui.active_tab == GuiTab::ColorGrid && idx == s.gui.color_cursor {
                    // Glowing white border
                    ops.push(HudDrawOp::Fill {
                        rect: [bx - 2.0, by - 2.0, swatch_w + 4.0, 2.0],
                        argb: 0xFFFF_FFFF,
                    });
                    ops.push(HudDrawOp::Fill {
                        rect: [bx - 2.0, by + swatch_h, swatch_w + 4.0, 2.0],
                        argb: 0xFFFF_FFFF,
                    });
                    ops.push(HudDrawOp::Fill {
                        rect: [bx - 2.0, by - 2.0, 2.0, swatch_h + 4.0],
                        argb: 0xFFFF_FFFF,
                    });
                    ops.push(HudDrawOp::Fill {
                        rect: [bx + swatch_w, by - 2.0, 2.0, swatch_h + 4.0],
                        argb: 0xFFFF_FFFF,
                    });
                }
            }

            // Current / hovered color label
            let cur_col_idx = if s.gui.active_tab == GuiTab::ColorGrid {
                s.gui.color_cursor
            } else {
                COLOR_PALETTE
                    .iter()
                    .position(|(_, c)| *c == (wp.rgb & 0x00FF_FFFF))
                    .unwrap_or(0)
            };
            ops.push(HudDrawOp::Text {
                text: format!("Color: {}", COLOR_PALETTE[cur_col_idx].0),
                at: [right_x, my + 154.0],
                rgb: COLOR_PALETTE[cur_col_idx].1,
                shadow: true,
            });

            // Right pane footer
            let footer_text = match s.gui.active_tab {
                GuiTab::EditName => "[Enter] Save Name | [Esc] Cancel",
                GuiTab::ColorGrid => "[Arrows] Move | [Enter] Select Color",
                GuiTab::List => "[E] Edit Name | [C] Color Grid",
            };
            let footer_col = match s.gui.active_tab {
                GuiTab::EditName | GuiTab::ColorGrid => 0x0055_FF55,
                GuiTab::List => 0x0088_99AA,
            };
            ops.push(HudDrawOp::Text {
                text: footer_text.to_string(),
                at: [right_x, my + modal_h - 18.0],
                rgb: footer_col,
                shadow: true,
            });
        }
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

/// Operations to draw on HUD.
#[derive(Clone, Debug)]
pub enum HudDrawOp {
    Fill { rect: [f32; 4], argb: u32 },
    Text { text: String, at: [f32; 2], rgb: u32, shadow: bool },
}

/// Draw HUD and Minimap markers.
/// Computes drawing operations into a Vec to avoid multiple mutable closures in native_hud.
pub fn draw_hud(
    screen_width: f32,
    screen_height: f32,
    player: &PlayerSnapshot,
    settings: &Settings,
    text_width: impl Fn(&str) -> f32,
) -> Vec<HudDrawOp> {
    let mut ops = Vec::new();
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
    draw_waypoints_gui(screen_width, screen_height, &mut ops);

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
            s.gui.is_open = false;
        });

        // 1. Toggle GUI open
        assert!(!is_gui_open());
        toggle_gui(None);
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
    }
}
