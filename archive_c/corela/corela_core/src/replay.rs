//! Counts the user's actions and, after the last one, saves them all as
//! `replay_NNNN.json`.

use std::fs::OpenOptions;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use crate::history::{Command, History};
use crate::model::{Board, Edit, Node, PlacedNode};

const FIRST_ACTION: i32 = 1;
/// Replay files are numbered from here, like `report_1000.txt`.
const FIRST_REPLAY_NUMBER: u32 = 1000;
const LAST_REPLAY_NUMBER: u32 = 9999;

/// One action, as it happened. Never changed after it is recorded.
struct Action {
    number: i32,
    command: Command,
    note: String,
    /// The board on screen after the action.
    board: Board,
    undo_count: usize,
    redo_count: usize,
}

pub(crate) struct Replay {
    last_action: i32,
    /// The action the user is on: the next one to be recorded.
    current_action: i32,
    actions: Vec<Action>,
}

impl Replay {
    pub(crate) const fn new(last_action: i32) -> Self {
        Self {
            last_action,
            current_action: FIRST_ACTION,
            actions: Vec::new(),
        }
    }

    /// The action the user is on, from 1 to `last_action`.
    pub(crate) fn current_action(&self) -> i32 {
        self.current_action.min(self.last_action)
    }

    pub(crate) const fn last_action(&self) -> i32 {
        self.last_action
    }

    /// Whether the last action has been recorded.
    pub(crate) const fn is_complete(&self) -> bool {
        self.current_action > self.last_action
    }

    /// Records the current action, and moves on to the next one.
    pub(crate) fn record(&mut self, command: Command, note: String, history: &History) {
        self.actions.push(Action {
            number: self.current_action,
            command,
            note,
            board: history.board().clone(),
            undo_count: history.undo_count(),
            redo_count: history.redo_count(),
        });
        self.current_action += 1;
    }

    /// Writes the replay to the first free `replay_NNNN.json` in `dir`,
    /// creating `dir` if needed, and returns that file's path.
    ///
    /// # Errors
    /// If `dir` cannot be created, a file cannot be written, or every
    /// number up to `LAST_REPLAY_NUMBER` is taken.
    pub(crate) fn save(&self, dir: &Path, name: &str, version: &str) -> Result<PathBuf, String> {
        if let Err(error) = std::fs::create_dir_all(dir) {
            return Err(format!("{}: {error}", dir.display()));
        }
        let json = self.to_json(name, version);
        let mut number = FIRST_REPLAY_NUMBER;
        while number <= LAST_REPLAY_NUMBER {
            let path = dir.join(format!("replay_{number}.json"));
            // create_new never overwrites an earlier replay
            let opened = OpenOptions::new().write(true).create_new(true).open(&path);
            match opened {
                Ok(mut file) => {
                    return match file.write_all(json.as_bytes()) {
                        Ok(()) => Ok(path),
                        Err(error) => Err(format!("{}: {error}", path.display())),
                    };
                }
                Err(error) if error.kind() == ErrorKind::AlreadyExists => number += 1,
                Err(error) => return Err(format!("{}: {error}", path.display())),
            }
        }
        Err(format!(
            "{}: replay_{FIRST_REPLAY_NUMBER}.json to replay_{LAST_REPLAY_NUMBER}.json are all taken",
            dir.display()
        ))
    }

    fn to_json(&self, name: &str, version: &str) -> String {
        let mut json = format!(
            "{{\n  \"name\": {},\n  \"version\": {},\n  \"last_action\": {},\n  \"actions\": [",
            json_string(name),
            json_string(version),
            self.last_action
        );
        let mut first = true;
        for action in &self.actions {
            if !first {
                json.push(',');
            }
            first = false;
            json.push_str("\n    ");
            json.push_str(&action_json(action));
        }
        json.push_str("\n  ]\n}\n");
        json
    }
}

fn action_json(action: &Action) -> String {
    let board = &action.board;
    let (q, r) = board.selected();
    let mut nodes = String::new();
    let mut first = true;
    for placed in board.nodes() {
        if !first {
            nodes.push_str(", ");
        }
        first = false;
        nodes.push_str(&node_json(placed));
    }
    format!(
        "{{\"action\": {}, \"command\": {}, \"note\": {}, \"region\": {}, \"selected\": [{q}, {r}], \"undo_count\": {}, \"redo_count\": {}, \"nodes\": [{nodes}]}}",
        action.number,
        command_json(action.command),
        json_string(&action.note),
        json_string(board.region().text()),
        action.undo_count,
        action.redo_count
    )
}

fn command_json(command: Command) -> String {
    match command {
        Command::Edit(Edit::Move { dq, dr }) => {
            format!("{{\"type\": \"move\", \"dq\": {dq}, \"dr\": {dr}}}")
        }
        Command::Edit(Edit::PlaceStart) => String::from("{\"type\": \"place_start\"}"),
        Command::Edit(Edit::PlaceArrow) => String::from("{\"type\": \"place_arrow\"}"),
        Command::Edit(Edit::Remove) => String::from("{\"type\": \"remove\"}"),
        Command::Undo => String::from("{\"type\": \"undo\"}"),
        Command::Redo => String::from("{\"type\": \"redo\"}"),
    }
}

fn node_json(placed: &PlacedNode) -> String {
    let (q, r) = placed.cell();
    let region = json_string(placed.region().text());
    match placed.node() {
        Node::BpmnStartNote => {
            format!("{{\"region\": {region}, \"cell\": [{q}, {r}], \"node\": \"start\"}}")
        }
        Node::Arrow(direction) => format!(
            "{{\"region\": {region}, \"cell\": [{q}, {r}], \"node\": \"arrow\", \"direction\": {direction}}}"
        ),
    }
}

/// `text` as a quoted JSON string.
fn json_string(text: &str) -> String {
    let mut quoted = String::from("\"");
    for character in text.chars() {
        match character {
            '"' => quoted.push_str("\\\""),
            '\\' => quoted.push_str("\\\\"),
            '\n' => quoted.push_str("\\n"),
            other if other.is_control() => {
                let escaped = format!("\\u{:04x}", u32::from(other));
                quoted.push_str(&escaped);
            }
            other => quoted.push(other),
        }
    }
    quoted.push('"');
    quoted
}
