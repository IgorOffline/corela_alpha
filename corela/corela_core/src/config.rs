use crate::model::Config;
use std::path::Path;

pub fn load_config(path: &Path) -> Config {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let table: toml::Table = text
        .parse()
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let value = |section: &str, key: &str| {
        table
            .get(section)
            .and_then(|t| t.get(key))
            .unwrap_or_else(|| panic!("{}: missing [{section}] {key}", path.display()))
    };
    let int = |section: &str, key: &str| {
        value(section, key)
            .as_integer()
            .unwrap_or_else(|| panic!("{}: [{section}] {key} must be an integer", path.display()))
    };
    let str = |section: &str, key: &str| {
        value(section, key)
            .as_str()
            .unwrap_or_else(|| panic!("{}: [{section}] {key} must be a string", path.display()))
    };
    let color = |role: &str| int("color", &format!("{}_value", str("gfx", role))) as u32;
    let radius = |key: &str| {
        let number = value("gfx", key);
        let radius = number
            .as_float()
            .or_else(|| number.as_integer().map(|n| n as f64))
            .unwrap_or_else(|| panic!("{}: [gfx] {key} must be a number", path.display()));
        assert!(
            radius.is_finite() && radius > 0.0 && radius <= f32::MAX as f64,
            "{}: [gfx] {key} must be a positive finite radius",
            path.display()
        );
        radius as f32
    };
    let chevron_coordinate = |axis: &str, index: usize| {
        let key = format!("chevron_offsets_{axis}_{index}");
        let coordinate = value("gfx", &key)
            .as_array()
            .filter(|items| items.len() == 1)
            .and_then(|items| items[0].as_integer())
            .and_then(|n| i32::try_from(n).ok());
        coordinate.unwrap_or_else(|| {
            panic!(
                "{}: [gfx] {key} must be an array containing one 32-bit integer",
                path.display()
            )
        })
    };
    let chevron_offsets = (0..6)
        .map(|index| {
            (
                chevron_coordinate("q", index),
                chevron_coordinate("r", index),
            )
        })
        .collect();
    let cell_marker_outer_radius = radius("cell_marker_outer_radius");
    let cell_marker_inner_radius = radius("cell_marker_inner_radius");
    assert!(
        cell_marker_inner_radius < cell_marker_outer_radius,
        "{}: cell marker inner radius must be smaller than its outer radius",
        path.display()
    );
    Config {
        name: str("meta", "name").to_owned(),
        version: str("meta", "version").to_owned(),
        cell: (int("bpmn", "cell_x") as i32, int("bpmn", "cell_y") as i32),
        key_escape: int("gfx", "input_key_escape") as i32,
        key_q: int("gfx", "input_key_q") as i32,
        key_w: int("gfx", "input_key_w") as i32,
        key_e: int("gfx", "input_key_e") as i32,
        key_a: int("gfx", "input_key_a") as i32,
        key_s: int("gfx", "input_key_s") as i32,
        key_d: int("gfx", "input_key_d") as i32,
        window_width: int("gfx", "window_width") as i32,
        window_height: int("gfx", "window_height") as i32,
        radius: int("gfx", "hex_radius") as i32,
        size: int("gfx", "hex_size") as f32,
        base: color("color_base"),
        tile: color("color_tile"),
        selected: color("color_selected"),
        chevron_offsets,
        cell_marker_outer_radius,
        cell_marker_inner_radius,
        cell_marker_border_color: color("color_cell_marker_border"),
        cell_marker_fill_color: color("color_cell_marker_fill"),
        chevron_color: color("color_chevron"),
    }
}
