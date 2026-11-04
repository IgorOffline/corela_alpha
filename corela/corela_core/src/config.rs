use crate::model::{Config, Theme};
use corela_gfx::{Icon, Key};
use std::path::{Path, PathBuf};

pub(crate) const CONFIG_FILE_NAME: &str = "regina.toml";

pub(crate) const SECTION_META: &str = "meta";
const SECTION_ASSET: &str = "asset";
const SECTION_GFX: &str = "gfx";
const SECTION_INPUT: &str = "input";
const SECTION_REPLAY: &str = "replay";
const SECTION_THEME_DARK: &str = "theme_dark";
const SECTION_THEME_LIGHT: &str = "theme_light";
const THEME_COLOR_SUFFIX: &str = "_color";
const THEME_ROLE_SUFFIX: &str = "_role";

pub(crate) const KEY_NAME: &str = "name";
pub(crate) const KEY_VERSION: &str = "version";

const COLOR_VALUE_SUFFIX: &str = "_value";

const MIN_WINDOW_SIDE: i32 = 1;
const MAX_WINDOW_SIDE: i32 = 32767;
const MAX_COLOR: u32 = 0x00FF_FFFF;
const MIN_LAST_ACTION: i32 = 1;
const MAX_LAST_ACTION: i32 = 9999;

struct ConfigReader {
    path: PathBuf,
    table: toml::Table,
}

impl ConfigReader {
    fn open(path: &Path) -> Result<Self, String> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) => return Err(format!("{}: {error}", path.display())),
        };
        let table = match text.parse::<toml::Table>() {
            Ok(table) => table,
            Err(error) => return Err(format!("{}: {error}", path.display())),
        };
        Ok(Self {
            path: path.to_path_buf(),
            table,
        })
    }

    fn value(&self, section: &str, key: &str) -> Result<&toml::Value, String> {
        let found = match self.table.get(section) {
            Some(section_table) => section_table.get(key),
            None => None,
        };
        match found {
            Some(value) => Ok(value),
            None => Err(format!("{}: missing_{section}_{key}", self.path.display())),
        }
    }

    fn int(&self, section: &str, key: &str) -> Result<i64, String> {
        match self.value(section, key)?.as_integer() {
            Some(number) => Ok(number),
            None => Err(format!(
                "{}: {section}_{key}_must_be_an_integer",
                self.path.display()
            )),
        }
    }

    fn int_in(&self, section: &str, key: &str, min: i32, max: i32) -> Result<i32, String> {
        let number = self.int(section, key)?;
        match i32::try_from(number) {
            Ok(narrowed) if (min..=max).contains(&narrowed) => Ok(narrowed),
            _ => Err(format!(
                "{}: {section}_{key}_{number}_must_be_within_{min}_to_{max}",
                self.path.display()
            )),
        }
    }

    fn key(&self, key: &str) -> Result<Key, String> {
        let name = self.string(SECTION_INPUT, key)?;
        match Key::from_name(name) {
            Some(found) => Ok(found),
            None => Err(format!(
                "{}: {SECTION_INPUT}_{key}_{name}_is_not_a_key_name_try_Escape_Key0_or_W",
                self.path.display()
            )),
        }
    }

    fn window_side(&self, key: &str) -> Result<i32, String> {
        self.int_in(SECTION_GFX, key, MIN_WINDOW_SIDE, MAX_WINDOW_SIDE)
    }

    fn string(&self, section: &str, key: &str) -> Result<&str, String> {
        match self.value(section, key)?.as_str() {
            Some(text) => Ok(text),
            None => Err(format!(
                "{}: {section}_{key}_must_be_a_string",
                self.path.display()
            )),
        }
    }

    fn color(&self, theme: &str, role: &str) -> Result<u32, String> {
        let name = self.string(&format!("{theme}{THEME_ROLE_SUFFIX}"), role)?;
        let section = format!("{theme}{THEME_COLOR_SUFFIX}");
        let key = format!("{name}{COLOR_VALUE_SUFFIX}");
        let number = self.int(&section, &key)?;
        match u32::try_from(number) {
            Ok(color) if color <= MAX_COLOR => Ok(color),
            _ => Err(format!(
                "{}: {section}_{key}_{number}_must_be_a_0xRRGGBB_color",
                self.path.display()
            )),
        }
    }

    fn theme(&self, theme: &str) -> Result<Theme, String> {
        Ok(Theme {
            base_color: self.color(theme, "color_base")?,
            tile_color: self.color(theme, "color_tile")?,
            selected_color: self.color(theme, "color_selected")?,
            selected_label_text_color: self.color(theme, "color_selected_label_text")?,
            cell_marker_border_color: self.color(theme, "color_cell_marker_border")?,
            cell_marker_fill_color: self.color(theme, "color_cell_marker_fill")?,
            chevron_color: self.color(theme, "color_chevron")?,
            selected_chevron_color: self.color(theme, "color_selected_chevron")?,
            label_text_color: self.color(theme, "color_label_text")?,
        })
    }

    fn path(&self, section: &str, key: &str) -> Result<PathBuf, String> {
        let root = match self.path.parent() {
            Some(root) => root,
            None => return Err(format!("{CONFIG_FILE_NAME}_has_no_parent_directory")),
        };
        Ok(root.join(self.string(section, key)?))
    }

    fn icon(&self, key: &str) -> Result<Icon, String> {
        let png = self.path(SECTION_ASSET, key)?;
        let bytes = match std::fs::read(&png) {
            Ok(bytes) => bytes,
            Err(error) => return Err(format!("{}: {error}", png.display())),
        };
        match Icon::load_png(&bytes) {
            Ok(icon) => Ok(icon),
            Err(error) => Err(format!("{}: {error}", png.display())),
        }
    }
}

pub(crate) fn load_config(path: &Path) -> Result<Config, String> {
    let reader = ConfigReader::open(path)?;
    Ok(Config {
        name: reader.string(SECTION_META, KEY_NAME)?.to_owned(),
        version: reader.string(SECTION_META, KEY_VERSION)?.to_owned(),
        key_start: reader.key("input_key_start")?,
        key_arrow: reader.key("input_key_arrow")?,
        key_remove: reader.key("input_key_remove")?,
        key_cycle: reader.key("input_key_cycle")?,
        key_run: reader.key("input_key_run")?,
        key_escape: reader.key("input_key_escape")?,
        key_north_west: reader.key("input_key_north_west")?,
        key_north: reader.key("input_key_north")?,
        key_north_east: reader.key("input_key_north_east")?,
        key_south_west: reader.key("input_key_south_west")?,
        key_south: reader.key("input_key_south")?,
        key_south_east: reader.key("input_key_south_east")?,
        window_width: reader.window_side("window_width")?,
        window_height: reader.window_side("window_height")?,
        key_theme: reader.key("input_key_theme")?,
        dark_theme: reader.theme(SECTION_THEME_DARK)?,
        light_theme: reader.theme(SECTION_THEME_LIGHT)?,
        chevron_icon: reader.icon("icon_chevron")?,
        replay_dir: reader.path(SECTION_REPLAY, "replay_dir")?,
        last_action: reader.int_in(
            SECTION_REPLAY,
            "replay_last_action",
            MIN_LAST_ACTION,
            MAX_LAST_ACTION,
        )?,
    })
}
