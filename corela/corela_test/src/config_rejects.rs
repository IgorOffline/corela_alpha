//! Startup must reject every invalid configuration with exit status 1 and
//! exactly one error line naming the problem, and must never panic.
//!
//! These cases fail before corela opens a window, so they need no desktop.

use std::time::Duration;

use crate::sandbox::{Sandbox, quoted};

const LIMIT: Duration = Duration::from_secs(10);

fn sandbox(name: &str) -> Sandbox {
    Sandbox::new(name)
}

// --- finding the configuration --------------------------------------------

#[must_use]
pub fn missing_config_file() -> i32 {
    let mut sandbox = sandbox("missing_config_file");
    sandbox.without_config();
    sandbox
        .run(LIMIT)
        .assert_rejected("no regina.toml for corela_bpmn 0.1.0 within 3 directories")
}

#[must_use]
pub fn config_for_another_version_is_ignored() -> i32 {
    let mut sandbox = sandbox("config_for_another_version");
    sandbox.config().set("meta", "version", &quoted("9.9.9"));
    sandbox
        .run(LIMIT)
        .assert_rejected("no regina.toml for corela_bpmn 0.1.0")
}

#[must_use]
pub fn config_for_another_product_is_ignored() -> i32 {
    let mut sandbox = sandbox("config_for_another_product");
    sandbox.config().set("meta", "name", &quoted("not_corela"));
    sandbox
        .run(LIMIT)
        .assert_rejected("no regina.toml for corela_bpmn 0.1.0")
}

#[must_use]
pub fn malformed_toml_is_ignored() -> i32 {
    // a file corela cannot parse cannot claim to belong to it
    let mut sandbox = sandbox("malformed_toml");
    sandbox.config().replace_all("[meta\nname = ");
    sandbox
        .run(LIMIT)
        .assert_rejected("no regina.toml for corela_bpmn 0.1.0")
}

// --- missing values and wrong types -----------------------------------------

#[must_use]
pub fn missing_key() -> i32 {
    let mut sandbox = sandbox("missing_key");
    sandbox.config().remove("gfx", "window_width");
    sandbox
        .run(LIMIT)
        .assert_rejected("missing [gfx] window_width")
}

#[must_use]
pub fn string_where_integer_expected() -> i32 {
    let mut sandbox = sandbox("string_for_integer");
    sandbox.config().set("gfx", "window_width", &quoted("wide"));
    sandbox
        .run(LIMIT)
        .assert_rejected("[gfx] window_width must be an integer")
}

#[must_use]
pub fn integer_where_string_expected() -> i32 {
    let mut sandbox = sandbox("integer_for_string");
    sandbox.config().set("theme_dark_role", "color_tile", "5");
    sandbox
        .run(LIMIT)
        .assert_rejected("[theme_dark_role] color_tile must be a string")
}

#[must_use]
pub fn role_names_a_color_that_does_not_exist() -> i32 {
    let mut sandbox = sandbox("unknown_palette_color");
    sandbox
        .config()
        .set("theme_dark_role", "color_tile", &quoted("no_such_color"));
    sandbox
        .run(LIMIT)
        .assert_rejected("missing [theme_dark_color] no_such_color_value")
}

#[must_use]
pub fn icon_file_does_not_exist() -> i32 {
    let mut sandbox = sandbox("missing_icon_file");
    sandbox
        .config()
        .set("asset", "icon_chevron", &quoted("no_such_icon.svg"));
    sandbox.run(LIMIT).assert_rejected("no_such_icon.svg: ")
}

// --- key codes: 1..=511 -----------------------------------------------------

#[must_use]
pub fn key_code_that_overflows_32_bits() -> i32 {
    // 4294967377 = 2^32 + 81: an unchecked 32-bit conversion would turn it
    // into 81, the code of the Q key
    let mut sandbox = sandbox("key_code_overflow");
    sandbox
        .config()
        .set("input", "input_key_north", "4294967377");
    sandbox
        .run(LIMIT)
        .assert_rejected("[input] input_key_north = 4294967377 must be within 1..=511")
}

#[must_use]
pub fn key_code_zero() -> i32 {
    let mut sandbox = sandbox("key_code_zero");
    sandbox.config().set("input", "input_key_start", "0");
    sandbox
        .run(LIMIT)
        .assert_rejected("[input] input_key_start = 0 must be within 1..=511")
}

#[must_use]
pub fn key_code_negative() -> i32 {
    let mut sandbox = sandbox("key_code_negative");
    sandbox.config().set("input", "input_key_south", "-5");
    sandbox
        .run(LIMIT)
        .assert_rejected("[input] input_key_south = -5 must be within 1..=511")
}

#[must_use]
pub fn key_code_just_above_range() -> i32 {
    let mut sandbox = sandbox("key_code_512");
    sandbox.config().set("input", "input_key_escape", "512");
    sandbox
        .run(LIMIT)
        .assert_rejected("[input] input_key_escape = 512 must be within 1..=511")
}

// --- window size: 1..=32767 -------------------------------------------------

#[must_use]
pub fn window_width_zero() -> i32 {
    let mut sandbox = sandbox("window_width_zero");
    sandbox.config().set("gfx", "window_width", "0");
    sandbox
        .run(LIMIT)
        .assert_rejected("[gfx] window_width = 0 must be within 1..=32767")
}

#[must_use]
pub fn window_height_beyond_x11_limit() -> i32 {
    let mut sandbox = sandbox("window_height_32768");
    sandbox.config().set("gfx", "window_height", "32768");
    sandbox
        .run(LIMIT)
        .assert_rejected("[gfx] window_height = 32768 must be within 1..=32767")
}

// --- colors: 0x000000..=0xFFFFFF ---------------------------------------------

#[must_use]
pub fn negative_color() -> i32 {
    let mut sandbox = sandbox("negative_color");
    sandbox.config().set("theme_dark_color", "base_value", "-1");
    sandbox
        .run(LIMIT)
        .assert_rejected("[theme_dark_color] base_value = -1 must be a 0xRRGGBB color")
}

#[must_use]
pub fn color_with_more_than_24_bits() -> i32 {
    let mut sandbox = sandbox("color_too_big");
    sandbox
        .config()
        .set("theme_dark_color", "base_value", "0x1000000");
    sandbox
        .run(LIMIT)
        .assert_rejected("[theme_dark_color] base_value = 16777216 must be a 0xRRGGBB color")
}

#[must_use]
pub fn light_theme_colors_are_checked_too() -> i32 {
    let mut sandbox = sandbox("light_color_too_big");
    sandbox
        .config()
        .set("theme_light_color", "base_value", "0xFFFFFFFF");
    sandbox
        .run(LIMIT)
        .assert_rejected("[theme_light_color] base_value = 4294967295 must be a 0xRRGGBB color")
}
