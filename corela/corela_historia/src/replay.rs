use std::fs::OpenOptions;
use std::io::{Error, ErrorKind, Write};
use std::path::{Path, PathBuf};

use corela_common::{Command, Edit, Node, PlacedNode, Snapshot};
use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};

const FIRST_ACTION: i32 = 1;
const FIRST_REPLAY_NUMBER: u32 = 1000;
const LAST_REPLAY_NUMBER: u32 = 9999;
const XML_INDENT_CHARACTER: u8 = b' ';
const XML_INDENT_SIZE: usize = 2;
const REPLAY_TAG: &str = "replay";
const ACTION_TAG: &str = "action";
const COMMAND_TAG: &str = "command";
const NODE_TAG: &str = "node";

struct Action {
    number: i32,
    command: Command,
    note: String,
    snapshot: Snapshot,
}

pub struct Replay {
    last_action: i32,
    current_action: i32,
    actions: Vec<Action>,
}

impl Replay {
    #[must_use]
    pub const fn new(last_action: i32) -> Self {
        Self {
            last_action,
            current_action: FIRST_ACTION,
            actions: Vec::new(),
        }
    }

    #[must_use]
    pub fn current_action(&self) -> i32 {
        self.current_action.min(self.last_action)
    }

    #[must_use]
    pub const fn last_action(&self) -> i32 {
        self.last_action
    }

    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.current_action > self.last_action
    }

    pub fn record(&mut self, command: Command, note: String, snapshot: Snapshot) {
        if matches!(command, Command::Undo | Command::Redo) {
            return;
        }
        self.actions.push(Action {
            number: self.current_action,
            command,
            note,
            snapshot,
        });
        self.current_action += 1;
    }

    pub fn save(&self, dir: &Path, name: &str, version: &str) -> Result<PathBuf, String> {
        if let Err(error) = std::fs::create_dir_all(dir) {
            return Err(format!("{}: {error}", dir.display()));
        }
        let xml = match self.to_xml(name, version) {
            Ok(xml) => xml,
            Err(error) => return Err(format!("{}: {error}", dir.display())),
        };
        let mut number = FIRST_REPLAY_NUMBER;
        while number <= LAST_REPLAY_NUMBER {
            let path = dir.join(format!("replay_{number}.xml"));
            let opened = OpenOptions::new().write(true).create_new(true).open(&path);
            match opened {
                Ok(mut file) => {
                    return match file.write_all(&xml) {
                        Ok(()) => Ok(path),
                        Err(error) => Err(format!("{}: {error}", path.display())),
                    };
                }
                Err(error) if error.kind() == ErrorKind::AlreadyExists => number += 1,
                Err(error) => return Err(format!("{}: {error}", path.display())),
            }
        }
        Err(format!(
            "{}: replay_{FIRST_REPLAY_NUMBER}.xml_to_replay_{LAST_REPLAY_NUMBER}.xml_are_all_taken",
            dir.display()
        ))
    }

    fn to_xml(&self, name: &str, version: &str) -> Result<Vec<u8>, Error> {
        let mut writer = Writer::new_with_indent(Vec::new(), XML_INDENT_CHARACTER, XML_INDENT_SIZE);
        let declaration = BytesDecl::new("1.0", Some("UTF-8"), None);
        writer.write_event(Event::Decl(declaration))?;
        let mut replay = BytesStart::new(REPLAY_TAG);
        replay.push_attribute(("name", name));
        replay.push_attribute(("version", version));
        replay.push_attribute(("last_action", self.last_action.to_string().as_str()));
        writer.write_event(Event::Start(replay))?;
        for action in &self.actions {
            write_action(&mut writer, action)?;
        }
        writer.write_event(Event::End(BytesEnd::new(REPLAY_TAG)))?;
        let mut xml = writer.into_inner();
        xml.push(b'\n');
        Ok(xml)
    }
}

fn write_action(writer: &mut Writer<Vec<u8>>, action: &Action) -> Result<(), Error> {
    let snapshot = &action.snapshot;
    let board = &snapshot.board;
    let (q, r) = board.selected();
    let mut element = BytesStart::new(ACTION_TAG);
    element.push_attribute(("number", action.number.to_string().as_str()));
    element.push_attribute(("note", action.note.as_str()));
    element.push_attribute(("region", board.region().code()));
    element.push_attribute(("selected_q", q.to_string().as_str()));
    element.push_attribute(("selected_r", r.to_string().as_str()));
    element.push_attribute(("undo_count", snapshot.undo_count.to_string().as_str()));
    element.push_attribute(("redo_count", snapshot.redo_count.to_string().as_str()));
    writer.write_event(Event::Start(element))?;
    writer.write_event(Event::Empty(command_element(action.command)))?;
    for placed in board.nodes() {
        writer.write_event(Event::Empty(node_element(placed)))?;
    }
    writer.write_event(Event::End(BytesEnd::new(ACTION_TAG)))
}

fn command_element(command: Command) -> BytesStart<'static> {
    let mut element = BytesStart::new(COMMAND_TAG);
    match command {
        Command::Edit(Edit::Move { dq, dr }) => {
            element.push_attribute(("type", "move"));
            element.push_attribute(("dq", dq.to_string().as_str()));
            element.push_attribute(("dr", dr.to_string().as_str()));
        }
        Command::Edit(Edit::PlaceStart) => element.push_attribute(("type", "place_start")),
        Command::Edit(Edit::PlaceArrow) => element.push_attribute(("type", "place_arrow")),
        Command::Edit(Edit::Remove) => element.push_attribute(("type", "remove")),
        Command::Undo => element.push_attribute(("type", "undo")),
        Command::Redo => element.push_attribute(("type", "redo")),
    }
    element
}

fn node_element(placed: &PlacedNode) -> BytesStart<'static> {
    let (q, r) = placed.cell();
    let mut element = BytesStart::new(NODE_TAG);
    element.push_attribute(("region", placed.region().code()));
    element.push_attribute(("q", q.to_string().as_str()));
    element.push_attribute(("r", r.to_string().as_str()));
    match placed.node() {
        Node::BpmnStartNote => element.push_attribute(("type", "start")),
        Node::Arrow(direction) => {
            element.push_attribute(("type", "arrow"));
            element.push_attribute(("direction", direction.to_string().as_str()));
        }
    }
    element
}
