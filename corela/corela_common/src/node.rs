#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Node {
    BpmnStartNote,
    Arrow(u8),
}
