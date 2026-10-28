use crate::model::Config;
use corela_gfx::Icon;
use std::path::{Path, PathBuf};

pub const CONFIG_FILE_NAME: &str = "regina.toml";

pub const SECTION_META: &str = "meta";
const SECTION_BPMN: &str = "bpmn";
const SECTION_ASSET: &str = "asset";
const SECTION_COLOR: &str = "color";
const SECTION_GFX: &str = "gfx";
const SECTION_INPUT: &str = "input";

pub const KEY_NAME: &str = "name";
pub const KEY_VERSION: &str = "version";

/// `[gfx] color_tile = "blue"` resolves to `[color] blue_value`.
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

    /// Looks up the palette color named by `[gfx] role`.
    fn color(&self, role: &str) -> u32 {
        let name = self.string(SECTION_GFX, role);
        let key = format!("{name}{COLOR_VALUE_SUFFIX}");
        self.int(SECTION_COLOR, &key) as u32
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
        cell: (
            reader.int(SECTION_BPMN, "cell_x") as i32,
            reader.int(SECTION_BPMN, "cell_y") as i32,
        ),
        key_escape: reader.int(SECTION_INPUT, "input_key_escape") as i32,
        key_q: reader.int(SECTION_INPUT, "input_key_q") as i32,
        key_w: reader.int(SECTION_INPUT, "input_key_w") as i32,
        key_e: reader.int(SECTION_INPUT, "input_key_e") as i32,
        key_a: reader.int(SECTION_INPUT, "input_key_a") as i32,
        key_s: reader.int(SECTION_INPUT, "input_key_s") as i32,
        key_d: reader.int(SECTION_INPUT, "input_key_d") as i32,
        window_width: reader.int(SECTION_GFX, "window_width") as i32,
        window_height: reader.int(SECTION_GFX, "window_height") as i32,
        base_color: reader.color("color_base"),
        tile_color: reader.color("color_tile"),
        selected_color: reader.color("color_selected"),
        cell_marker_border_color: reader.color("color_cell_marker_border"),
        cell_marker_fill_color: reader.color("color_cell_marker_fill"),
        chevron_color: reader.color("color_chevron"),
        chevron_icon: reader.icon("icon_chevron"),
    }
}
