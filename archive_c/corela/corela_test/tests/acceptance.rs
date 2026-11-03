//! Acceptance tests for corela, driven through its real window.
//!
//! Each scenario runs a private copy of corela whose `regina.toml` ends the
//! session after a few actions. The When steps press keys; corela then
//! saves its replay and quits by itself, and the Then steps check that
//! replay. Nothing is inspected while corela runs.
//!
//! Needs a display, a debug build of corela, and xdotool (`XDOTOOL` names
//! the executable when it is not on `PATH`).

mod session;

use std::path::Path;

use cucumber::gherkin::Step;
use cucumber::{World as _, given, then, when};
use serde_json::Value;

use session::{NORTH_KEY, SOUTH_KEY, Session};

#[derive(Debug, Default, cucumber::World)]
struct Acceptance {
    session: Option<Session>,
    /// The `actions` array of the saved replay.
    actions: Vec<Value>,
}

impl Acceptance {
    fn session(&mut self) -> &mut Session {
        self.session.as_mut().expect("Given must start a session")
    }
}

#[given(expr = "a new session that ends after {int} actions")]
fn new_session(world: &mut Acceptance, last_action: u32) {
    world.session = Some(Session::start(last_action));
}

#[when(expr = "I move {word} {int} time(s)")]
fn move_times(world: &mut Acceptance, direction: String, count: usize) {
    let key = match direction.as_str() {
        "north" => NORTH_KEY,
        "south" => SOUTH_KEY,
        other => panic!("no key for direction {other:?}"),
    };
    for _ in 0..count {
        world.session().press(key);
    }
}

#[then(expr = "corela saves a replay of {int} actions")]
fn replay_saved(world: &mut Acceptance, count: usize) {
    world.actions = world.session().finish();
    assert_eq!(world.actions.len(), count, "{:#?}", world.actions);
}

#[then("the replay shows")]
fn replay_shows(world: &mut Acceptance, step: &Step) {
    let table = step.table.as_ref().expect("the step needs a table");
    // row 0 is the header: action | region | note
    for row in table.rows.iter().skip(1) {
        let number: usize = row[0].parse().expect("action number");
        let action = &world.actions[number - 1];
        assert_eq!(action["action"], number, "replay out of order: {action}");
        assert_eq!(
            action["region"],
            row[1].as_str(),
            "action {number}: {action}"
        );
        assert_eq!(action["note"], row[2].as_str(), "action {number}: {action}");
    }
}

fn main() {
    let features = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/features");
    futures::executor::block_on(
        Acceptance::cucumber()
            .max_concurrent_scenarios(1)
            .fail_on_skipped()
            .run_and_exit(features),
    );
}
