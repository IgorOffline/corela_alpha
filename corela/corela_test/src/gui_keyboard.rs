//! One end-to-end session at the keyboard: moving, placing, rotating and
//! removing nodes, crossing between regions, switching theme and quitting.
//!
//! Each key press is judged by the gameplay event corela logs and, where
//! the effect is only visible, by captures of the window.

use crate::product::{self, keys, selected};
use crate::sandbox::Sandbox;
use crate::session::Session;
use crate::x11;

/// More actions than this session takes, so corela does not save a replay
/// and quit halfway (that is `gui_replay`'s subject).
const LAST_ACTION: i32 = 99;

#[must_use]
pub fn keyboard_session() -> i32 {
    let _desktop = x11::desktop();
    let mut sandbox = Sandbox::new("keyboard_session");
    sandbox
        .config()
        .set("replay", "replay_last_action", &LAST_ACTION.to_string());
    let mut session = Session::start(&sandbox, LAST_ACTION);

    // every direction once, ending back at the center
    session.act(keys::NORTH, &selected(0, -1), "north");
    session.act(keys::NORTH_EAST, &selected(1, -2), "north-east");
    session.act(keys::SOUTH_EAST, &selected(2, -2), "south-east");
    session.act(keys::SOUTH, &selected(2, -1), "south");
    session.act(keys::SOUTH_WEST, &selected(1, 0), "south-west");
    session.act(keys::NORTH_WEST, &selected(0, 0), "north-west");

    // the start marker: one per project
    session.act(
        keys::PLACE_START,
        &product::placed_start(0, 0),
        "place start at (0,0)",
    );
    let start_placed = session.capture();
    session.act(keys::NORTH, &selected(0, -1), "north");
    session.act(
        keys::PLACE_START,
        product::SECOND_START,
        "second start, must be refused",
    );
    let second_refused = session.capture();

    let at_origin = start_placed.cell_center(0, 0);
    let above_origin = start_placed.cell_center(0, -1);
    let fill = Some(product::START_MARKER_FILL);
    session.checks.check(
        start_placed.pixel(at_origin.0, at_origin.1) == fill,
        "start marker is drawn at (0,0)",
    );
    session.checks.check(
        second_refused.pixel(above_origin.0, above_origin.1) != fill,
        "no second start marker at (0,-1)",
    );
    session.checks.check(
        second_refused.pixel(at_origin.0, at_origin.1) == fill,
        "the first start marker stays at (0,0)",
    );

    // an arrow: place, rotate, remove
    session.act(
        keys::PLACE_ARROW,
        &product::placed_arrow(0, -1),
        "place arrow at (0,-1)",
    );
    let arrow = session.capture();
    session.act(
        keys::PLACE_ARROW,
        &product::turned_arrow(0, -1, 1),
        "rotate the arrow",
    );
    let rotated = session.capture();
    session.act(keys::REMOVE, &product::removed(0, -1), "remove the arrow");
    let removed = session.capture();
    session.checks.check(
        arrow.board_difference(&second_refused) > 0,
        "placing the arrow changes the board",
    );
    session.checks.check(
        rotated.board_difference(&arrow) > 0,
        "rotating the arrow changes the board",
    );
    session.checks.check(
        removed.board_difference(&second_refused) == 0,
        "after removal the board is exactly as before the arrow",
    );

    // regions: nodes in North stay in North
    session.act(keys::NORTH, &selected(0, -2), "north");
    session.act(keys::NORTH, &selected(0, -3), "north");
    session.act(keys::NORTH, &selected(0, -4), "north to the gate");
    let center_before = session.capture();
    session.act(keys::NORTH, product::ENTERED_NORTH, "cross into North");
    let north_empty = session.capture();
    // (1, 4) lies outside the board: max(|1|, |4|, |1 + 4|) = 5 > 4
    session.act(
        keys::SOUTH_EAST,
        &product::out_of_bounds(1, 4),
        "out of bounds, ignored",
    );
    session.act(
        keys::PLACE_ARROW,
        &product::placed_arrow(0, 4),
        "place arrow in North (0,4)",
    );
    let north_arrow = session.capture();
    session.act(keys::SOUTH, product::ENTERED_CENTER, "cross back to Center");
    let center_after = session.capture();
    session.act(
        keys::REMOVE,
        &product::nothing_to_remove(0, -4),
        "remove in Center at (0,-4)",
    );
    session.act(
        keys::NORTH,
        product::ENTERED_NORTH,
        "cross into North again",
    );
    let north_again = session.capture();
    session.checks.check(
        north_arrow.board_difference(&north_empty) > 0,
        "the arrow appears in North",
    );
    session.checks.check(
        center_after.board_difference(&center_before) == 0,
        "the North arrow does not show in Center",
    );
    session.checks.check(
        north_again.board_difference(&north_arrow) == 0,
        "removing in Center leaves the North arrow alone",
    );

    // theme, then quit
    session.view(keys::THEME, product::LIGHT_THEME, "light theme");
    let light = session.capture();
    session.checks.check(
        light.board_difference(&north_again) > 0,
        "the light theme changes the board",
    );
    session.view(keys::THEME, product::DARK_THEME, "dark theme");
    session.view(keys::QUIT, product::QUIT, "quit");
    session.check_exit("quit");

    session.checks.finish()
}
