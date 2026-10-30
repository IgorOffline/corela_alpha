use crate::model::{Config, Theme};
use corela_gfx::Icon;
use std::path::{Path, PathBuf};

pub(crate) const CONFIG_FILE_NAME: &str = "regina.toml";

pub(crate) const SECTION_META: &str = "meta";
const SECTION_ASSET: &str = "asset";
const SECTION_GFX: &str = "gfx";
const SECTION_INPUT: &str = "input";
const SECTION_THEME_DARK: &str = "theme_dark";
const SECTION_THEME_LIGHT: &str = "theme_light";
const THEME_COLOR_SUFFIX: &str = "_color";
const THEME_ROLE_SUFFIX: &str = "_role";

pub(crate) const KEY_NAME: &str = "name";
pub(crate) const KEY_VERSION: &str = "version";

/// `[theme_dark_role] color_tile = "blue"` resolves to `[theme_dark_color] blue_value`.
const COLOR_VALUE_SUFFIX: &str = "_value";

/// `sokol_app` keycodes: 0 is `SAPP_KEYCODE_INVALID`, and all of them are below
/// `SAPP_MAX_KEYCODES` (512).
const MIN_KEY_CODE: i32 = 1;
const MAX_KEY_CODE: i32 = 511;
/// X11 window sizes are 16-bit.
const MIN_WINDOW_SIDE: i32 = 1;
const MAX_WINDOW_SIDE: i32 = 32767;
/// Colors are packed as 0xRRGGBB.
const MAX_COLOR: u32 = 0x00FF_FFFF;

/// Reads typed values from a parsed config file. Anything missing, of the
/// wrong type or out of range is an error naming the file, section and key.
struct ConfigReader {
    path: PathBuf,
    table: toml::Table,
}

impl ConfigReader {
    /// # Errors
    /// If the file cannot be read or is not valid TOML.
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

    /// # Errors
    /// If `[section] key` is missing.
    fn value(&self, section: &str, key: &str) -> Result<&toml::Value, String> {
        let found = match self.table.get(section) {
            Some(section_table) => section_table.get(key),
            None => None,
        };
        match found {
            Some(value) => Ok(value),
            None => Err(format!(
                "{}: missing [{section}] {key}",
                self.path.display()
            )),
        }
    }

    /// # Errors
    /// If `[section] key` is missing or not an integer.
    fn int(&self, section: &str, key: &str) -> Result<i64, String> {
        match self.value(section, key)?.as_integer() {
            Some(number) => Ok(number),
            None => Err(format!(
                "{}: [{section}] {key} must be an integer",
                self.path.display()
            )),
        }
    }

    /// An integer within `min..=max`.
    ///
    /// # Errors
    /// If `[section] key` is missing, not an integer, or outside the range.
    fn int_in(&self, section: &str, key: &str, min: i32, max: i32) -> Result<i32, String> {
        let number = self.int(section, key)?;
        match i32::try_from(number) {
            Ok(narrowed) if (min..=max).contains(&narrowed) => Ok(narrowed),
            _ => Err(format!(
                "{}: [{section}] {key} = {number} must be within {min}..={max}",
                self.path.display()
            )),
        }
    }

    /// A `sokol_app` keycode from `[input] key`.
    ///
    /// # Errors
    /// If the key is missing, not an integer, or not a valid keycode.
    fn key_code(&self, key: &str) -> Result<i32, String> {
        self.int_in(SECTION_INPUT, key, MIN_KEY_CODE, MAX_KEY_CODE)
    }

    /// A window width or height in pixels from `[gfx] key`.
    ///
    /// # Errors
    /// If the key is missing, not an integer, or not a valid X11 size.
    fn window_side(&self, key: &str) -> Result<i32, String> {
        self.int_in(SECTION_GFX, key, MIN_WINDOW_SIDE, MAX_WINDOW_SIDE)
    }

    /// # Errors
    /// If `[section] key` is missing or not a string.
    fn string(&self, section: &str, key: &str) -> Result<&str, String> {
        match self.value(section, key)?.as_str() {
            Some(text) => Ok(text),
            None => Err(format!(
                "{}: [{section}] {key} must be a string",
                self.path.display()
            )),
        }
    }

    /// Looks up the palette color named by `[<theme>_role] role`.
    ///
    /// # Errors
    /// If the role or the color it names is missing, or the color is not a
    /// 0xRRGGBB integer.
    fn color(&self, theme: &str, role: &str) -> Result<u32, String> {
        let name = self.string(&format!("{theme}{THEME_ROLE_SUFFIX}"), role)?;
        let section = format!("{theme}{THEME_COLOR_SUFFIX}");
        let key = format!("{name}{COLOR_VALUE_SUFFIX}");
        let number = self.int(&section, &key)?;
        match u32::try_from(number) {
            Ok(color) if color <= MAX_COLOR => Ok(color),
            _ => Err(format!(
                "{}: [{section}] {key} = {number} must be a 0xRRGGBB color",
                self.path.display()
            )),
        }
    }

    /// # Errors
    /// If any of the theme's colors fails as described in `color`.
    fn theme(&self, theme: &str) -> Result<Theme, String> {
        Ok(Theme {
            base_color: self.color(theme, "color_base")?,
            tile_color: self.color(theme, "color_tile")?,
            selected_color: self.color(theme, "color_selected")?,
            cell_marker_border_color: self.color(theme, "color_cell_marker_border")?,
            cell_marker_fill_color: self.color(theme, "color_cell_marker_fill")?,
            chevron_color: self.color(theme, "color_chevron")?,
            label_text_color: self.color(theme, "color_label_text")?,
        })
    }

    /// Loads the SVG named by `[asset] key`, relative to the config file.
    ///
    /// # Errors
    /// If the key is missing, or the SVG cannot be read or parsed.
    fn icon(&self, key: &str) -> Result<Icon, String> {
        let root = match self.path.parent() {
            Some(root) => root,
            None => return Err(format!("{CONFIG_FILE_NAME} has no parent directory")),
        };
        let svg = root.join(self.string(SECTION_ASSET, key)?);
        let text = match std::fs::read_to_string(&svg) {
            Ok(text) => text,
            Err(error) => return Err(format!("{}: {error}", svg.display())),
        };
        match Icon::parse(&text) {
            Ok(icon) => Ok(icon),
            Err(error) => Err(format!("{}: {error}", svg.display())),
        }
    }
}

/// Reads and validates the whole config file.
///
/// # Errors
/// A message naming the file, and the section and key where there is one,
/// if the file cannot be read or parsed, or a value is missing, has the
/// wrong type or is out of range.
pub(crate) fn load_config(path: &Path) -> Result<Config, String> {
    let reader = ConfigReader::open(path)?;
    Ok(Config {
        name: reader.string(SECTION_META, KEY_NAME)?.to_owned(),
        version: reader.string(SECTION_META, KEY_VERSION)?.to_owned(),
        key_start: reader.key_code("input_key_start")?,
        key_arrow: reader.key_code("input_key_arrow")?,
        key_remove: reader.key_code("input_key_remove")?,
        key_escape: reader.key_code("input_key_escape")?,
        key_north_west: reader.key_code("input_key_north_west")?,
        key_north: reader.key_code("input_key_north")?,
        key_north_east: reader.key_code("input_key_north_east")?,
        key_south_west: reader.key_code("input_key_south_west")?,
        key_south: reader.key_code("input_key_south")?,
        key_south_east: reader.key_code("input_key_south_east")?,
        window_width: reader.window_side("window_width")?,
        window_height: reader.window_side("window_height")?,
        key_theme: reader.key_code("input_key_theme")?,
        dark_theme: reader.theme(SECTION_THEME_DARK)?,
        light_theme: reader.theme(SECTION_THEME_LIGHT)?,
        chevron_icon: reader.icon("icon_chevron")?,
    })
}
