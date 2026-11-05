use std::fs::OpenOptions;
use std::io::{Error, ErrorKind, Write};
use std::path::{Path, PathBuf};

use corela_ema::entity::{Node, PlacedNode};
use corela_ema::message::{Command, Edit};
use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};

use crate::action::Action;
use crate::replay::Replay;
use crate::xml_format::{
    ACTION_TAG, ARROW_NODE, COMMAND_TAG, DIRECTION_ATTRIBUTE, DQ_ATTRIBUTE, DR_ATTRIBUTE,
    LAST_ACTION_ATTRIBUTE, MOVE_COMMAND, NAME_ATTRIBUTE, NODE_TAG, NOTE_ATTRIBUTE,
    NUMBER_ATTRIBUTE, PLACE_ARROW_COMMAND, PLACE_START_COMMAND, Q_ATTRIBUTE, R_ATTRIBUTE,
    REDO_COMMAND, REDO_COUNT_ATTRIBUTE, REGION_ATTRIBUTE, REMOVE_COMMAND, REPLAY_FILE_PREFIX,
    REPLAY_FILE_SUFFIX, REPLAY_TAG, SELECTED_Q_ATTRIBUTE, SELECTED_R_ATTRIBUTE, START_NODE,
    TYPE_ATTRIBUTE, UNDO_COMMAND, UNDO_COUNT_ATTRIBUTE, VERSION_ATTRIBUTE,
};

const FIRST_REPLAY_NUMBER: u32 = 1000;
const LAST_REPLAY_NUMBER: u32 = 9999;
const XML_INDENT_CHARACTER: u8 = b' ';
const XML_INDENT_SIZE: usize = 2;

pub fn save_replay(
    replay: &Replay,
    dir: &Path,
    name: &str,
    version: &str,
) -> Result<PathBuf, String> {
    if let Err(error) = std::fs::create_dir_all(dir) {
        return Err(format!("{}: {error}", dir.display()));
    }
    let xml = match replay_to_xml(replay, name, version) {
        Ok(xml) => xml,
        Err(error) => return Err(format!("{}: {error}", dir.display())),
    };
    let mut number = FIRST_REPLAY_NUMBER;
    while number <= LAST_REPLAY_NUMBER {
        let path = dir.join(replay_file_name(number));
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
        "{}: {}_to_{}_are_all_taken",
        dir.display(),
        replay_file_name(FIRST_REPLAY_NUMBER),
        replay_file_name(LAST_REPLAY_NUMBER)
    ))
}

fn replay_file_name(number: u32) -> String {
    format!("{REPLAY_FILE_PREFIX}{number}{REPLAY_FILE_SUFFIX}")
}

fn replay_to_xml(replay: &Replay, name: &str, version: &str) -> Result<Vec<u8>, Error> {
    let mut writer = Writer::new_with_indent(Vec::new(), XML_INDENT_CHARACTER, XML_INDENT_SIZE);
    let declaration = BytesDecl::new("1.0", Some("UTF-8"), None);
    writer.write_event(Event::Decl(declaration))?;
    let last_action = replay.last_action().to_string();
    let mut root = BytesStart::new(REPLAY_TAG);
    root.push_attribute((NAME_ATTRIBUTE, name));
    root.push_attribute((VERSION_ATTRIBUTE, version));
    root.push_attribute((LAST_ACTION_ATTRIBUTE, last_action.as_str()));
    writer.write_event(Event::Start(root))?;
    for action in replay.actions() {
        write_action(&mut writer, action)?;
    }
    writer.write_event(Event::End(BytesEnd::new(REPLAY_TAG)))?;
    let mut xml = writer.into_inner();
    xml.push(b'\n');
    Ok(xml)
}

fn write_action(writer: &mut Writer<Vec<u8>>, action: &Action) -> Result<(), Error> {
    let snapshot = action.snapshot();
    let board = action.board();
    let (q, r) = board.selected();
    let mut element = BytesStart::new(ACTION_TAG);
    element.push_attribute((NUMBER_ATTRIBUTE, action.number().to_string().as_str()));
    element.push_attribute((NOTE_ATTRIBUTE, action.note()));
    element.push_attribute((REGION_ATTRIBUTE, board.region().code()));
    element.push_attribute((SELECTED_Q_ATTRIBUTE, q.to_string().as_str()));
    element.push_attribute((SELECTED_R_ATTRIBUTE, r.to_string().as_str()));
    element.push_attribute((
        UNDO_COUNT_ATTRIBUTE,
        snapshot.undo_count.to_string().as_str(),
    ));
    element.push_attribute((
        REDO_COUNT_ATTRIBUTE,
        snapshot.redo_count.to_string().as_str(),
    ));
    writer.write_event(Event::Start(element))?;
    writer.write_event(Event::Empty(command_element(action.command())))?;
    for placed in board.nodes() {
        writer.write_event(Event::Empty(node_element(placed)))?;
    }
    writer.write_event(Event::End(BytesEnd::new(ACTION_TAG)))
}

fn command_element(command: Command) -> BytesStart<'static> {
    let mut element = BytesStart::new(COMMAND_TAG);
    match command {
        Command::Edit(Edit::Move { dq, dr }) => {
            element.push_attribute((TYPE_ATTRIBUTE, MOVE_COMMAND));
            element.push_attribute((DQ_ATTRIBUTE, dq.to_string().as_str()));
            element.push_attribute((DR_ATTRIBUTE, dr.to_string().as_str()));
        }
        Command::Edit(Edit::PlaceStart) => {
            element.push_attribute((TYPE_ATTRIBUTE, PLACE_START_COMMAND));
        }
        Command::Edit(Edit::PlaceArrow) => {
            element.push_attribute((TYPE_ATTRIBUTE, PLACE_ARROW_COMMAND));
        }
        Command::Edit(Edit::Remove) => element.push_attribute((TYPE_ATTRIBUTE, REMOVE_COMMAND)),
        Command::Undo => element.push_attribute((TYPE_ATTRIBUTE, UNDO_COMMAND)),
        Command::Redo => element.push_attribute((TYPE_ATTRIBUTE, REDO_COMMAND)),
    }
    element
}

fn node_element(placed: &PlacedNode) -> BytesStart<'static> {
    let (q, r) = placed.cell();
    let mut element = BytesStart::new(NODE_TAG);
    element.push_attribute((REGION_ATTRIBUTE, placed.region().code()));
    element.push_attribute((Q_ATTRIBUTE, q.to_string().as_str()));
    element.push_attribute((R_ATTRIBUTE, r.to_string().as_str()));
    match placed.node() {
        Node::BpmnStartNote => element.push_attribute((TYPE_ATTRIBUTE, START_NODE)),
        Node::Arrow(direction) => {
            element.push_attribute((TYPE_ATTRIBUTE, ARROW_NODE));
            element.push_attribute((DIRECTION_ATTRIBUTE, direction.to_string().as_str()));
        }
    }
    element
}
