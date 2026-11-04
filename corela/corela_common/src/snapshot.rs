use crate::board::Board;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub board: Board,
    pub undo_count: usize,
    pub redo_count: usize,
}
