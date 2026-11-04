use crate::edit::Edit;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Edit(Edit),
    Undo,
    Redo,
}
