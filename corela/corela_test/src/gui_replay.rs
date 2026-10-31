//! Undo, redo and the replay: the shipped configuration ends the session
//! after action 24, saves every action to the first free
//! `corela_replay/replay_NNNN.json` and quits.

use crate::product::{self, keys, selected};
use crate::sandbox::Sandbox;
use crate::session::Session;
use crate::x11;

/// Already in the replay directory when corela starts; it must be kept.
const EARLIER_REPLAY: &str = "{\"earlier\": true}\n";

#[must_use]
pub fn undo_redo_and_replay() -> i32 {
    let _desktop = x11::desktop();
    let sandbox = Sandbox::new("undo_redo_and_replay");
    let replay_dir = sandbox.dir().join(product::REPLAY_DIR);
    let earlier = replay_dir.join(product::replay_file(1000));
    match std::fs::create_dir_all(&replay_dir) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("cannot create replay dir: {e}");
            return 1;
        }
    }
    match std::fs::write(&earlier, EARLIER_REPLAY) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("cannot write earlier replay: {e}");
            return 1;
        }
    }
    let mut session = Session::start(&sandbox, product::LAST_ACTION);
    let empty = session.capture();

    // undo and redo step between whole boards
    session.act(keys::PLACE_ARROW, &product::placed_arrow(0, 0), "arrow");
    let arrow = session.capture();
    session.select_mode("undo");
    session.view(keys::RUN, &product::undo(0, 1), "undo the arrow");
    let undone = session.capture();
    session.select_mode("redo");
    session.view(keys::RUN, &product::redo(1, 0), "redo the arrow");
    let redone = session.capture();
    session.checks.check(
        arrow.board_difference(&empty) > 0,
        "placing the arrow changes the board",
    );
    session.checks.check(
        undone.board_difference(&empty) == 0,
        "undo brings back exactly the empty board",
    );
    session.checks.check(
        redone.board_difference(&arrow) == 0,
        "redo brings back exactly the arrow",
    );

    // several undos in a row, until there is nothing left
    session.act(
        keys::PLACE_ARROW,
        &product::turned_arrow(0, 0, 1),
        "turn the arrow",
    );
    session.select_mode("undo");
    session.view(keys::RUN, &product::undo(1, 1), "undo the turn");
    let unturned = session.capture();
    session.select_mode("undo");
    session.view(keys::RUN, &product::undo(0, 2), "undo the arrow");
    session.select_mode("undo");
    session.view(keys::RUN, product::UNDO_NOT_LEGAL, "undo at the start");
    let first = session.capture();
    session.checks.check(
        unturned.board_difference(&arrow) == 0,
        "undoing the turn brings back the unturned arrow",
    );
    session.checks.check(
        first.board_difference(&empty) == 0,
        "undo with nothing left changes nothing",
    );

    // a new action after an undo drops what could have been redone
    session.select_mode("redo");
    session.view(keys::RUN, &product::redo(1, 1), "redo the arrow");
    session.act(keys::NORTH, &selected(0, -1), "north");
    session.select_mode("redo");
    session.view(keys::RUN, product::REDO_NOT_LEGAL, "redo after a move");
    // the selection is part of the board, so a move is undone like an edit
    session.select_mode("undo");
    session.view(keys::RUN, &product::undo(1, 1), "undo the move");
    let back = session.capture();
    session.checks.check(
        back.board_difference(&arrow) == 0,
        "undoing the move selects the arrow's cell again",
    );

    // undo a removal
    session.act(keys::SOUTH, &selected(0, 1), "south");
    session.act(keys::PLACE_START, &product::placed_start(0, 1), "start");
    let start = session.capture();
    session.act(keys::REMOVE, &product::removed(0, 1), "remove the start");
    session.select_mode("undo");
    session.view(keys::RUN, &product::undo(3, 1), "undo the removal");
    let restored = session.capture();
    session.checks.check(
        restored.board_difference(&start) == 0,
        "undoing the removal brings the start node back",
    );

    // Six moves/edits have advanced the action counter. Undo, redo and
    // mode changes leave it unchanged. Actions 7 to 9 revisit the arrow.
    session.act(keys::NORTH, &selected(0, 0), "north");
    session.act(
        keys::PLACE_ARROW,
        &product::turned_arrow(0, 0, 1),
        "turn the restored arrow",
    );
    session.act(keys::SOUTH, &selected(0, 1), "south");

    // Actions 10 to 23, ending at (0, 1).
    for _ in 0..7 {
        session.act(keys::NORTH, &selected(0, 0), "north");
        session.act(keys::SOUTH, &selected(0, 1), "south");
    }

    // action 24 is the last: save the replay, then quit
    let removed = session.event(&product::removed(0, 1));
    let saved = session.event(product::SAVED_REPLAY);
    let quit = session.event(product::QUIT);
    let events = session.press_raw(keys::REMOVE);
    let replay = replay_dir.join(product::replay_file(1001));
    let saved_line = format!("{saved}{}", replay.display());
    let wanted = vec![removed, saved_line, quit];
    session.checks.check(
        events == wanted,
        format!("action 24 saves and quits: expected {wanted:?}, got {events:?}"),
    );
    session.check_exit("the last action");

    let text = std::fs::read_to_string(&replay).unwrap_or_default();
    session.checks.check(
        text.matches("{\"action\": ").count() == 24,
        format!("{} holds all 24 actions", replay.display()),
    );
    session.checks.check(
        text.contains("{\"action\": 24, \"command\": {\"type\": \"remove\"}"),
        "the replay ends with action 24, the removal",
    );
    // Undo and redo are not replay commands. Their effects are captured
    // in the board snapshots attached to the following moves and edits.
    session.checks.check(
        !text.contains("\"type\": \"undo\"") && !text.contains("\"type\": \"redo\""),
        "undo and redo do not consume replay actions",
    );
    let after_redo = replay_action(&text, 3);
    session.checks.check(
        after_redo.contains("\"node\": \"arrow\", \"direction\": 0")
            && after_redo.contains("\"undo_count\": 2, \"redo_count\": 0"),
        "the replay preserves the redone arrow and the discarded redo history",
    );
    let after_undo = replay_action(&text, 7);
    session.checks.check(
        after_undo.contains("\"cell\": [0, 1], \"node\": \"start\"")
            && after_undo.contains("\"undo_count\": 4, \"redo_count\": 0"),
        "the replay preserves the start node restored by undo",
    );
    session.checks.check(
        std::fs::read_to_string(&earlier).unwrap_or_default() == EARLIER_REPLAY,
        "the earlier replay_1000.json is left alone",
    );

    session.checks.finish()
}

/// Each action occupies one line in the product's replay format.
fn replay_action(text: &str, number: i32) -> &str {
    let prefix = format!("{{\"action\": {number},");
    text.lines()
        .find(|line| line.trim_start().starts_with(&prefix))
        .unwrap_or_default()
}
