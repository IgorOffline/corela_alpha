use crate::model::{Config, Theme};
use corela_gfx::Icon;
use std::path::{Path, PathBuf};

pub const CONFIG_FILE_NAME: &str = "regina.toml";

pub const SECTION_META: &str = "meta";
const SECTION_ASSET: &str = "asset";
const SECTION_GFX: &str = "gfx";
const SECTION_INPUT: &str = "input";
const SECTION_THEME_DARK: &str = "theme_dark";
const SECTION_THEME_LIGHT: &str = "theme_light";
const THEME_COLOR_SUFFIX: &str = "_color";
const THEME_ROLE_SUFFIX: &str = "_role";

pub const KEY_NAME: &str = "name";
pub const KEY_VERSION: &str = "version";

/// `[theme_dark_role] color_tile = "blue"` resolves to `[theme_dark_color] blue_value`.
const COLOR_VALUE_SUFFIX: &str = "_value";

/// Reads typed values from a parsed config file. Anything missing or of the
/// wrong type panics with the file path, since corela cannot run without it.
struct ConfigReader {
    path: PathBuf,
    table: toml::Table,
}

impl ConfigReader {
    fn open(path: &Path) -> ConfigReader {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) => panic!("{}: {error}", path.display()),
        };
        let table = match text.parse::<toml::Table>() {
            Ok(table) => table,
            Err(error) => panic!("{}: {error}", path.display()),
        };
        ConfigReader {
            path: path.to_path_buf(),
            table,
        }
    }

    fn value(&self, section: &str, key: &str) -> &toml::Value {
        let found = match self.table.get(section) {
            Some(section_table) => section_table.get(key),
            None => None,
        };
        match found {
            Some(value) => value,
            None => panic!("{}: missing [{section}] {key}", self.path.display()),
        }
    }

    fn int(&self, section: &str, key: &str) -> i64 {
        match self.value(section, key).as_integer() {
            Some(number) => number,
            None => panic!(
                "{}: [{section}] {key} must be an integer",
                self.path.display()
            ),
        }
    }

    fn string(&self, section: &str, key: &str) -> &str {
        match self.value(section, key).as_str() {
            Some(text) => text,
            None => panic!(
                "{}: [{section}] {key} must be a string",
                self.path.display()
            ),
        }
    }

    /// Looks up the palette color named by `[<theme>_role] role`.
    fn color(&self, theme: &str, role: &str) -> u32 {
        let name = self.string(&format!("{theme}{THEME_ROLE_SUFFIX}"), role);
        let key = format!("{name}{COLOR_VALUE_SUFFIX}");
        self.int(&format!("{theme}{THEME_COLOR_SUFFIX}"), &key) as u32
    }

    fn theme(&self, theme: &str) -> Theme {
        Theme {
            base_color: self.color(theme, "color_base"),
            tile_color: self.color(theme, "color_tile"),
            selected_color: self.color(theme, "color_selected"),
            cell_marker_border_color: self.color(theme, "color_cell_marker_border"),
            cell_marker_fill_color: self.color(theme, "color_cell_marker_fill"),
            chevron_color: self.color(theme, "color_chevron"),
            label_text_color: self.color(theme, "color_label_text"),
        }
    }

    /// Loads the SVG named by `[asset] key`, relative to the config file.
    fn icon(&self, key: &str) -> Icon {
        let root = match self.path.parent() {
            Some(root) => root,
            None => panic!("{CONFIG_FILE_NAME} has no parent directory"),
        };
        let svg = root.join(self.string(SECTION_ASSET, key));
        let text = match std::fs::read_to_string(&svg) {
            Ok(text) => text,
            Err(error) => panic!("{}: {error}", svg.display()),
        };
        match Icon::parse(&text) {
            Ok(icon) => icon,
            Err(error) => panic!("{}: {error}", svg.display()),
        }
    }
}

pub fn load_config(path: &Path) -> Config {
    let reader = ConfigReader::open(path);
    Config {
        name: reader.string(SECTION_META, KEY_NAME).to_owned(),
        version: reader.string(SECTION_META, KEY_VERSION).to_owned(),
        key_start: reader.int(SECTION_INPUT, "input_key_start") as i32,
        key_arrow: reader.int(SECTION_INPUT, "input_key_arrow") as i32,
        key_remove: reader.int(SECTION_INPUT, "input_key_remove") as i32,
        key_escape: reader.int(SECTION_INPUT, "input_key_escape") as i32,
        key_north_west: reader.int(SECTION_INPUT, "input_key_north_west") as i32,
        key_north: reader.int(SECTION_INPUT, "input_key_north") as i32,
        key_north_east: reader.int(SECTION_INPUT, "input_key_north_east") as i32,
        key_south_west: reader.int(SECTION_INPUT, "input_key_south_west") as i32,
        key_south: reader.int(SECTION_INPUT, "input_key_south") as i32,
        key_south_east: reader.int(SECTION_INPUT, "input_key_south_east") as i32,
        window_width: reader.int(SECTION_GFX, "window_width") as i32,
        window_height: reader.int(SECTION_GFX, "window_height") as i32,
        key_theme: reader.int(SECTION_INPUT, "input_key_theme") as i32,
        dark_theme: reader.theme(SECTION_THEME_DARK),
        light_theme: reader.theme(SECTION_THEME_LIGHT),
        chevron_icon: reader.icon("icon_chevron"),
    }
}
