//! What this suite knows about corela, and where each fact comes from.
//!
//! The suite has no access to corela's source. Everything below comes
//! from the shipped files (the binary, `regina.toml`, the SVG asset) or
//! from behavior observed at the program's boundary: exit status, log
//! output on stderr, and the window. If corela changes one of these on
//! purpose, this file is the one place to update.

use std::path::{Path, PathBuf};

// --- shipped artifacts ---------------------------------------------------

/// Where the product under test lives. `CORELA_HOME` overrides the default,
/// which is the directory above this suite.
#[must_use]
pub fn home() -> PathBuf {
    if let Some(home) = std::env::var_os("CORELA_HOME") {
        PathBuf::from(home)
    } else {
        let relative = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        relative.canonicalize().unwrap_or(relative)
    }
}

/// The corela executable. `CORELA_BIN` overrides the default debug build.
///
/// # Panics
/// If the executable does not exist; build corela first.
#[must_use]
pub fn binary() -> PathBuf {
    let path = match std::env::var_os("CORELA_BIN") {
        Some(path) => PathBuf::from(path),
        None => home().join("target/debug/corela_bpmn"),
    };
    assert!(
        path.is_file(),
        "corela binary not found at {} (build corela first, or set CORELA_BIN)",
        path.display()
    );
    path
}

/// The configuration file corela ships with.
pub const CONFIG_FILE_NAME: &str = "regina.toml";

/// The shipped configuration's text.
///
/// # Panics
/// If the shipped configuration cannot be read.
#[must_use]
pub fn shipped_config() -> String {
    let path = home().join(CONFIG_FILE_NAME);
    match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => panic!("cannot read {}: {error}", path.display()),
    }
}

// --- startup contract ----------------------------------------------------

/// corela looks for `regina.toml` next to its executable and in up to two
/// parent directories, and accepts only a file whose `[meta]` names this
/// product and version.
pub const PRODUCT_NAME: &str = "corela_bpmn";
pub const PRODUCT_VERSION: &str = "0.1.0";

/// Exit status when startup fails (bad or missing configuration).
pub const EXIT_STARTUP_ERROR: i32 = 1;
/// Exit status after the user quits.
pub const EXIT_OK: i32 = 0;

/// Printed to stderr once the window and its OpenGL context exist.
pub const READY_LINE: &str = "System: ";
/// Every startup failure is logged as one line containing this.
pub const ERROR_TAG: &str = "[ERROR";
/// Rust's panic message. It must never appear.
pub const PANIC_TEXT: &str = "panicked";

// --- window and layout (measured from the running application) -----------

pub const WINDOW_TITLE: &str = "corela_bpmn";
/// The board is a hexagon of flat-top hex cells, `BOARD_RADIUS` cells from
/// its center to each edge, centered in the window.
pub const BOARD_RADIUS: i32 = 4;
/// Horizontal distance between neighbouring cell centers (pixels).
pub const CELL_PITCH_X: f64 = 48.0;
/// Vertical distance between cells in the same column (pixels).
pub const CELL_PITCH_Y: f64 = 55.425_625;
/// Half the board's width and height, rounded up, plus a margin. Comparing
/// only this area keeps other desktop windows near the screen edges from
/// affecting results.
pub const BOARD_HALF_WIDTH: i32 = 224;
pub const BOARD_HALF_HEIGHT: i32 = 262;

/// The start marker's fill color in the default (dark) theme: the
/// `[theme_dark_color] text_value` of the shipped config.
pub const START_MARKER_FILL: [u8; 3] = [0xCA, 0xD3, 0xF5];

// --- keyboard (default bindings in the shipped regina.toml) ----------------

/// xdotool key names for the default bindings.
pub mod keys {
    pub const NORTH: &str = "w";
    pub const NORTH_EAST: &str = "e";
    pub const SOUTH_EAST: &str = "d";
    pub const SOUTH: &str = "s";
    pub const SOUTH_WEST: &str = "a";
    pub const NORTH_WEST: &str = "q";
    pub const PLACE_START: &str = "0";
    pub const PLACE_ARROW: &str = "1";
    pub const REMOVE: &str = "x";
    /// Steps the input mode: none, place, undo, redo.
    pub const CYCLE: &str = "i";
    /// Runs the current input mode, when it is legal.
    pub const RUN: &str = "r";
    pub const THEME: &str = "m";
    pub const QUIT: &str = "Escape";
}

/// Valid key code values in `[input]` (the `sokol_app` range).
pub const MIN_KEY_CODE: i64 = 1;
pub const MAX_KEY_CODE: i64 = 511;

// --- log contract (stderr, with RUST_LOG=info) -----------------------------

/// Gameplay events are logged as `core: <event>`.
pub const EVENT_PREFIX: &str = "core: ";

/// Every event names the action the user is on, as the counter at the top
/// of the window shows it: `action 3/24: <note>`. Moves and edits each
/// count as one action; undo, redo, input mode changes, the theme switch
/// and quitting leave the counter unchanged.
#[must_use]
pub fn action(number: i32, last: i32, note: &str) -> String {
    format!("action {number}/{last}: {note}")
}

/// The `[replay] replay_last_action` of the shipped config: after this
/// many actions corela saves the replay and quits.
pub const LAST_ACTION: i32 = 24;
/// Where replays are saved, relative to `regina.toml`.
pub const REPLAY_DIR: &str = "corela_replay";
/// Replays are saved as `replay_1000.json`, `replay_1001.json`, ..., each
/// to the first number not taken yet.
#[must_use]
pub fn replay_file(number: u32) -> String {
    format!("replay_{number}.json")
}
/// Logged, with the replay file's path, after the last action.
pub const SAVED_REPLAY: &str = "saved replay ";

#[must_use]
pub fn selected(q: i32, r: i32) -> String {
    format!("selected ({q}, {r})")
}
#[must_use]
pub fn out_of_bounds(q: i32, r: i32) -> String {
    format!("({q}, {r}) is out of bounds")
}
#[must_use]
pub fn placed_start(q: i32, r: i32) -> String {
    format!("placed start node at ({q}, {r})")
}
pub const SECOND_START: &str = "the project already has a start node";
#[must_use]
pub fn placed_arrow(q: i32, r: i32) -> String {
    format!("placed arrow at ({q}, {r})")
}
#[must_use]
pub fn turned_arrow(q: i32, r: i32, direction: u8) -> String {
    format!("turned arrow at ({q}, {r}) to {direction}")
}
#[must_use]
pub fn removed(q: i32, r: i32) -> String {
    format!("removed node at ({q}, {r})")
}
#[must_use]
pub fn nothing_to_remove(q: i32, r: i32) -> String {
    format!("nothing to remove at ({q}, {r})")
}
#[must_use]
pub fn undo(undo_left: usize, redo_left: usize) -> String {
    format!("undo, {undo_left} more to undo, {redo_left} to redo")
}
#[must_use]
pub fn redo(undo_left: usize, redo_left: usize) -> String {
    format!("redo, {undo_left} to undo, {redo_left} more to redo")
}
pub const UNDO_NOT_LEGAL: &str = "undo is not legal now";
pub const REDO_NOT_LEGAL: &str = "redo is not legal now";
/// The input modes in the order the cycle key visits them.
pub const MODES: [&str; 4] = ["none", "place", "undo", "redo"];
#[must_use]
pub fn input_mode(name: &str) -> String {
    format!("input mode: {name}")
}
pub const ENTERED_NORTH: &str = "entered North region";
pub const ENTERED_CENTER: &str = "entered Center region";
pub const LIGHT_THEME: &str = "light theme";
pub const DARK_THEME: &str = "dark theme";
pub const QUIT: &str = "quit";

/// Moving north from Center (0, -4) enters North at (0, 4), and moving south
/// from North (0, 4) returns to Center (0, -4).
pub const CENTER_GATE: (i32, i32) = (0, -BOARD_RADIUS);
pub const NORTH_GATE: (i32, i32) = (0, BOARD_RADIUS);
