//! Undo and redo, in a pseudo-CQRS way.
//!
//! - Commands (the write side) go in through `History::execute`. They are
//!   the only way to change what is on screen.
//! - Queries (the read side) come out through `History::board`, a shared
//!   reference to an immutable `Board`, which the renderer and the replay
//!   read but cannot change.
//!
//! Because a `Board` never changes, undo and redo copy nothing back: they
//! only move boards between three places.
//!
//! ```text
//!   past (oldest .. newest)    present    future (newest .. oldest)
//!   [b1, b2, b3]               b4         [b6, b5]
//!
//!   undo:  b3 becomes present, b4 goes to the end of future
//!   redo:  b5 becomes present, b4 goes to the end of past
//!   edit:  the new board becomes present, b4 goes to past, future is cleared
//! ```

use crate::model::{Board, Edit};

/// What the user can ask for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Command {
    /// Make a new board from the present one.
    Edit(Edit),
    /// Go back to the board before the present one.
    Undo,
    /// Go forward again, after an undo.
    Redo,
}

pub(crate) struct History {
    /// Boards before `present`; the last one is the newest.
    past: Vec<Board>,
    /// The board on screen.
    present: Board,
    /// Boards that undo stepped back from; the last one is the next redo.
    future: Vec<Board>,
}

impl History {
    pub(crate) const fn new(first: Board) -> Self {
        Self {
            past: Vec::new(),
            present: first,
            future: Vec::new(),
        }
    }

    // --- queries ---

    /// The board on screen.
    pub(crate) const fn board(&self) -> &Board {
        &self.present
    }

    /// How many undos are possible right now.
    pub(crate) const fn undo_count(&self) -> usize {
        self.past.len()
    }

    /// How many redos are possible right now.
    pub(crate) const fn redo_count(&self) -> usize {
        self.future.len()
    }

    // --- commands ---

    /// Carries out `command` and returns a note for the log saying what
    /// happened.
    pub(crate) fn execute(&mut self, command: Command) -> String {
        match command {
            Command::Edit(edit) => self.edit(edit),
            Command::Undo => self.undo(),
            Command::Redo => self.redo(),
        }
    }

    fn edit(&mut self, edit: Edit) -> String {
        let outcome = self.present.apply(edit);
        // an edit that changes nothing leaves nothing to undo
        if outcome.board == self.present {
            return outcome.note;
        }
        let previous = std::mem::replace(&mut self.present, outcome.board);
        self.past.push(previous);
        // a new edit starts a new future: the old one can no longer be redone
        self.future.clear();
        outcome.note
    }

    fn undo(&mut self) -> String {
        let board = match self.past.pop() {
            Some(board) => board,
            None => return String::from("nothing to undo"),
        };
        let undone = std::mem::replace(&mut self.present, board);
        self.future.push(undone);
        format!(
            "undo, {} more to undo, {} to redo",
            self.past.len(),
            self.future.len()
        )
    }

    fn redo(&mut self) -> String {
        let board = match self.future.pop() {
            Some(board) => board,
            None => return String::from("nothing to redo"),
        };
        let redone = std::mem::replace(&mut self.present, board);
        self.past.push(redone);
        format!(
            "redo, {} to undo, {} more to redo",
            self.past.len(),
            self.future.len()
        )
    }
}
