use std::path::PathBuf;

use corela_gfx::{Icon, Key};

#[expect(
    clippy::struct_field_names,
    reason = "the suffix matches every other *_color name"
)]
pub(crate) struct Theme {
    pub(crate) base_color: u32,
    pub(crate) tile_color: u32,
    pub(crate) selected_color: u32,
    pub(crate) selected_label_text_color: u32,
    pub(crate) cell_marker_border_color: u32,
    pub(crate) cell_marker_fill_color: u32,
    pub(crate) chevron_color: u32,
    pub(crate) selected_chevron_color: u32,
    pub(crate) label_text_color: u32,
}

pub(crate) struct Config {
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) key_start: Key,
    pub(crate) key_arrow: Key,
    pub(crate) key_remove: Key,
    pub(crate) key_cycle: Key,
    pub(crate) key_run: Key,
    pub(crate) key_escape: Key,
    pub(crate) key_north_west: Key,
    pub(crate) key_north: Key,
    pub(crate) key_north_east: Key,
    pub(crate) key_south_west: Key,
    pub(crate) key_south: Key,
    pub(crate) key_south_east: Key,
    pub(crate) window_width: i32,
    pub(crate) window_height: i32,
    pub(crate) key_theme: Key,
    pub(crate) dark_theme: Theme,
    pub(crate) light_theme: Theme,
    pub(crate) chevron_icon: Icon,
    pub(crate) replay_dir: PathBuf,
    pub(crate) last_action: i32,
}
