use std::path::PathBuf;

use corela_gfx::Icon;

#[expect(
    clippy::struct_field_names,
    reason = "the suffix matches every other *_color name"
)]
pub(crate) struct Theme {
    pub(crate) base_color: u32,
    pub(crate) tile_color: u32,
    pub(crate) selected_color: u32,
    pub(crate) cell_marker_border_color: u32,
    pub(crate) cell_marker_fill_color: u32,
    pub(crate) chevron_color: u32,
    pub(crate) label_text_color: u32,
}

pub(crate) struct Config {
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) key_start: i32,
    pub(crate) key_arrow: i32,
    pub(crate) key_remove: i32,
    pub(crate) key_cycle: i32,
    pub(crate) key_run: i32,
    pub(crate) key_escape: i32,
    pub(crate) key_north_west: i32,
    pub(crate) key_north: i32,
    pub(crate) key_north_east: i32,
    pub(crate) key_south_west: i32,
    pub(crate) key_south: i32,
    pub(crate) key_south_east: i32,
    pub(crate) window_width: i32,
    pub(crate) window_height: i32,
    pub(crate) key_theme: i32,
    pub(crate) dark_theme: Theme,
    pub(crate) light_theme: Theme,
    pub(crate) chevron_icon: Icon,
    /// Where finished replays are saved.
    pub(crate) replay_dir: PathBuf,
    /// The replay is saved, and corela quits, after this action.
    pub(crate) last_action: i32,
}
