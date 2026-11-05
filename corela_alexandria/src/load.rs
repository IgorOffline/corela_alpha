use std::fs::Metadata;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::SystemTime;

use corela_ema::dto::Snapshot;
use corela_ema::entity::{Board, Node, PlacedNode, Region};
use corela_ema::message::{Command, Edit};
use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};

use crate::action::Action;
use crate::replay::Replay;
use crate::xml_format::{
    ACTION_TAG, ARROW_NODE, COMMAND_TAG, DIRECTION_ATTRIBUTE, DQ_ATTRIBUTE, DR_ATTRIBUTE,
    LAST_ACTION_ATTRIBUTE, MOVE_COMMAND, NODE_TAG, NOTE_ATTRIBUTE, NUMBER_ATTRIBUTE,
    PLACE_ARROW_COMMAND, PLACE_START_COMMAND, Q_ATTRIBUTE, R_ATTRIBUTE, REDO_COMMAND,
    REDO_COUNT_ATTRIBUTE, REGION_ATTRIBUTE, REMOVE_COMMAND, REPLAY_FILE_PREFIX, REPLAY_FILE_SUFFIX,
    REPLAY_TAG, SELECTED_Q_ATTRIBUTE, SELECTED_R_ATTRIBUTE, START_NODE, TYPE_ATTRIBUTE,
    UNDO_COMMAND, UNDO_COUNT_ATTRIBUTE,
};

struct ReplayFile {
    modified: SystemTime,
    path: PathBuf,
}

struct OpenAction {
    number: i32,
    note: String,
    region: Region,
    selected: (i32, i32),
    undo_count: usize,
    redo_count: usize,
    command: Option<Command>,
    nodes: Vec<PlacedNode>,
}

pub fn find_latest_replay(dir: &Path) -> Result<Option<PathBuf>, String> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("{}: {error}", dir.display())),
    };
    let mut latest: Option<ReplayFile> = None;
    for entry in entries {
        let found = match entry {
            Ok(found) => found,
            Err(error) => return Err(format!("{}: {error}", dir.display())),
        };
        let path = found.path();
        if !has_replay_file_name(&path) {
            continue;
        }
        let metadata = match found.metadata() {
            Ok(metadata) => metadata,
            Err(error) => return Err(format!("{}: {error}", path.display())),
        };
        if !metadata.is_file() {
            continue;
        }
        let candidate = ReplayFile {
            modified: modified_time(&metadata, &path)?,
            path,
        };
        latest = Some(newer_replay_file(latest, candidate));
    }
    match latest {
        Some(file) => Ok(Some(file.path)),
        None => Ok(None),
    }
}

fn has_replay_file_name(path: &Path) -> bool {
    let name = match path.file_name() {
        Some(name) => name.to_string_lossy(),
        None => return false,
    };
    name.starts_with(REPLAY_FILE_PREFIX) && name.ends_with(REPLAY_FILE_SUFFIX)
}

fn modified_time(metadata: &Metadata, path: &Path) -> Result<SystemTime, String> {
    match metadata.modified() {
        Ok(modified) => Ok(modified),
        Err(error) => Err(format!("{}: {error}", path.display())),
    }
}

fn newer_replay_file(latest: Option<ReplayFile>, candidate: ReplayFile) -> ReplayFile {
    let current = match latest {
        Some(current) => current,
        None => return candidate,
    };
    if candidate.modified > current.modified {
        return candidate;
    }
    if candidate.modified == current.modified && candidate.path > current.path {
        return candidate;
    }
    current
}

pub fn load_replay(path: &Path, empty_board: &Board) -> Result<Replay, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => return Err(format!("{}: {error}", path.display())),
    };
    match parse_replay(&text, empty_board) {
        Ok(replay) => Ok(replay),
        Err(error) => Err(format!("{}: {error}", path.display())),
    }
}

fn parse_replay(text: &str, empty_board: &Board) -> Result<Replay, String> {
    let mut reader = Reader::from_str(text);
    let mut last_action: Option<i32> = None;
    let mut open_action: Option<OpenAction> = None;
    let mut actions = Vec::new();
    loop {
        let event = match reader.read_event() {
            Ok(event) => event,
            Err(error) => return Err(error.to_string()),
        };
        match event {
            Event::Start(element) => {
                if has_tag(&element, REPLAY_TAG) {
                    last_action = Some(number_attribute(&element, LAST_ACTION_ATTRIBUTE)?);
                } else if has_tag(&element, ACTION_TAG) {
                    open_action = Some(read_open_action(&element)?);
                }
            }
            Event::Empty(element) => {
                if let Some(action) = &mut open_action {
                    read_action_child(&element, action)?;
                }
            }
            Event::End(element) => {
                if element.name().as_ref() == ACTION_TAG
                    && let Some(action) = open_action.take()
                {
                    actions.push(finish_action(action, empty_board)?);
                }
            }
            Event::Eof => break,
            Event::Text(_)
            | Event::CData(_)
            | Event::Comment(_)
            | Event::Decl(_)
            | Event::PI(_)
            | Event::DocType(_)
            | Event::GeneralRef(_) => {}
        }
    }
    match last_action {
        Some(last) => Ok(Replay::from_actions(last, actions)),
        None => Err(format!("{REPLAY_TAG}_element_is_missing")),
    }
}

fn has_tag(element: &BytesStart<'_>, tag: &str) -> bool {
    element.name().as_ref() == tag
}

fn text_attribute(element: &BytesStart<'_>, name: &str) -> Result<String, String> {
    let found = match element.try_get_attribute(name) {
        Ok(found) => found,
        Err(error) => return Err(error.to_string()),
    };
    let attribute = match found {
        Some(attribute) => attribute,
        None => return Err(format!("{name}_attribute_is_missing")),
    };
    match attribute.normalized_value(XmlVersion::Implicit1_0) {
        Ok(value) => Ok(value.into_owned()),
        Err(error) => Err(error.to_string()),
    }
}

fn number_attribute<T: FromStr>(element: &BytesStart<'_>, name: &str) -> Result<T, String> {
    let text = text_attribute(element, name)?;
    match text.parse::<T>() {
        Ok(number) => Ok(number),
        Err(_) => Err(format!("{name}_{text}_is_not_a_valid_number")),
    }
}

fn region_attribute(element: &BytesStart<'_>) -> Result<Region, String> {
    let code = text_attribute(element, REGION_ATTRIBUTE)?;
    match Region::from_code(&code) {
        Some(region) => Ok(region),
        None => Err(format!("{code}_is_not_a_region")),
    }
}

fn read_open_action(element: &BytesStart<'_>) -> Result<OpenAction, String> {
    Ok(OpenAction {
        number: number_attribute(element, NUMBER_ATTRIBUTE)?,
        note: text_attribute(element, NOTE_ATTRIBUTE)?,
        region: region_attribute(element)?,
        selected: (
            number_attribute(element, SELECTED_Q_ATTRIBUTE)?,
            number_attribute(element, SELECTED_R_ATTRIBUTE)?,
        ),
        undo_count: number_attribute(element, UNDO_COUNT_ATTRIBUTE)?,
        redo_count: number_attribute(element, REDO_COUNT_ATTRIBUTE)?,
        command: None,
        nodes: Vec::new(),
    })
}

fn read_action_child(element: &BytesStart<'_>, action: &mut OpenAction) -> Result<(), String> {
    if has_tag(element, COMMAND_TAG) {
        action.command = Some(read_command(element)?);
    } else if has_tag(element, NODE_TAG) {
        action.nodes.push(read_placed_node(element)?);
    }
    Ok(())
}

fn read_command(element: &BytesStart<'_>) -> Result<Command, String> {
    let command_type = text_attribute(element, TYPE_ATTRIBUTE)?;
    if command_type == MOVE_COMMAND {
        Ok(Command::Edit(Edit::Move {
            dq: number_attribute(element, DQ_ATTRIBUTE)?,
            dr: number_attribute(element, DR_ATTRIBUTE)?,
        }))
    } else if command_type == PLACE_START_COMMAND {
        Ok(Command::Edit(Edit::PlaceStart))
    } else if command_type == PLACE_ARROW_COMMAND {
        Ok(Command::Edit(Edit::PlaceArrow))
    } else if command_type == REMOVE_COMMAND {
        Ok(Command::Edit(Edit::Remove))
    } else if command_type == UNDO_COMMAND {
        Ok(Command::Undo)
    } else if command_type == REDO_COMMAND {
        Ok(Command::Redo)
    } else {
        Err(format!("{command_type}_is_not_a_command"))
    }
}

fn read_placed_node(element: &BytesStart<'_>) -> Result<PlacedNode, String> {
    let region = region_attribute(element)?;
    let cell = (
        number_attribute(element, Q_ATTRIBUTE)?,
        number_attribute(element, R_ATTRIBUTE)?,
    );
    let node_type = text_attribute(element, TYPE_ATTRIBUTE)?;
    let node = if node_type == START_NODE {
        Node::BpmnStartNote
    } else if node_type == ARROW_NODE {
        Node::Arrow(number_attribute(element, DIRECTION_ATTRIBUTE)?)
    } else {
        return Err(format!("{node_type}_is_not_a_node"));
    };
    Ok(PlacedNode::new(region, cell, node))
}

fn finish_action(action: OpenAction, empty_board: &Board) -> Result<Action, String> {
    let command = match action.command {
        Some(command) => command,
        None => return Err(format!("action_{}_has_no_command", action.number)),
    };
    let board = empty_board
        .with_selection(action.region, action.selected)
        .with_nodes(action.nodes);
    let snapshot = Snapshot {
        board,
        undo_count: action.undo_count,
        redo_count: action.redo_count,
    };
    Ok(Action::new(action.number, command, action.note, snapshot))
}
