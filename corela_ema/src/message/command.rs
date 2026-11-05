use crate::message::Edit;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Edit(Edit),
    Undo,
    Redo,
}
