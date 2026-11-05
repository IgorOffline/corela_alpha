use corela_ema::dto::Snapshot;
use corela_ema::entity::Board;
use corela_ema::message::{Command, Edit};

use crate::edit_handler;

pub struct History {
    past: Vec<Board>,
    present: Board,
    future: Vec<Board>,
}

impl History {
    #[must_use]
    pub const fn new(first: Board) -> Self {
        Self {
            past: Vec::new(),
            present: first,
            future: Vec::new(),
        }
    }

    #[must_use]
    pub const fn board(&self) -> &Board {
        &self.present
    }

    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            board: self.present.clone(),
            undo_count: self.past.len(),
            redo_count: self.future.len(),
        }
    }

    pub fn execute(&mut self, command: Command) -> Result<String, String> {
        match command {
            Command::Edit(edit) => self.edit(edit),
            Command::Undo => self.undo(),
            Command::Redo => self.redo(),
        }
    }

    fn edit(&mut self, edit: Edit) -> Result<String, String> {
        let outcome = edit_handler::apply(&self.present, edit)?;
        let previous = std::mem::replace(&mut self.present, outcome.board);
        self.past.push(previous);
        self.future.clear();
        Ok(outcome.note)
    }

    fn undo(&mut self) -> Result<String, String> {
        let board = match self.past.pop() {
            Some(board) => board,
            None => return Err(String::from("nothing_to_undo")),
        };
        let undone = std::mem::replace(&mut self.present, board);
        self.future.push(undone);
        Ok(format!(
            "undo_{}_more_to_undo_{}_to_redo",
            self.past.len(),
            self.future.len()
        ))
    }

    fn redo(&mut self) -> Result<String, String> {
        let board = match self.future.pop() {
            Some(board) => board,
            None => return Err(String::from("nothing_to_redo")),
        };
        let redone = std::mem::replace(&mut self.present, board);
        self.past.push(redone);
        Ok(format!(
            "redo_{}_to_undo_{}_more_to_redo",
            self.past.len(),
            self.future.len()
        ))
    }
}
