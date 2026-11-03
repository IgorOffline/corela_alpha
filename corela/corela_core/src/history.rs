use crate::model::{Board, Edit};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Command {
    Edit(Edit),
    Undo,
    Redo,
}

pub(crate) struct History {
    past: Vec<Board>,
    present: Board,
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

    pub(crate) const fn board(&self) -> &Board {
        &self.present
    }

    pub(crate) const fn undo_count(&self) -> usize {
        self.past.len()
    }

    pub(crate) const fn redo_count(&self) -> usize {
        self.future.len()
    }

    pub(crate) fn execute(&mut self, command: Command) -> String {
        match command {
            Command::Edit(edit) => self.edit(edit),
            Command::Undo => self.undo(),
            Command::Redo => self.redo(),
        }
    }

    fn edit(&mut self, edit: Edit) -> String {
        let outcome = self.present.apply(edit);
        if outcome.board == self.present {
            return outcome.note;
        }
        let previous = std::mem::replace(&mut self.present, outcome.board);
        self.past.push(previous);
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
