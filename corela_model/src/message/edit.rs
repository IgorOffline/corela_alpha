#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edit {
    Move { dq: i32, dr: i32 },
    PlaceStart,
    PlaceArrow,
    Remove,
}
