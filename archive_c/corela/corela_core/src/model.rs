mod config;
mod edits;

pub(crate) use config::{Config, Theme};

const CENTER_CELL: (i32, i32) = (0, 0);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Region {
    Center,
    North,
}

impl Region {
    pub(crate) const fn name(self) -> &'static std::ffi::CStr {
        match self {
            Self::Center => c"Center",
            Self::North => c"North",
        }
    }

    /// `name` as a Rust string, for logs and the replay file.
    pub(crate) const fn text(self) -> &'static str {
        match self {
            Self::Center => "Center",
            Self::North => "North",
        }
    }
}

/// A project may contain only one start node across all regions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Node {
    BpmnStartNote,
    Arrow(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PlacedNode {
    region: Region,
    cell: (i32, i32),
    node: Node,
}

impl PlacedNode {
    pub(crate) const fn region(&self) -> Region {
        self.region
    }

    pub(crate) const fn cell(&self) -> (i32, i32) {
        self.cell
    }

    pub(crate) const fn node(&self) -> Node {
        self.node
    }

    fn is_at(&self, region: Region, cell: (i32, i32)) -> bool {
        self.region == region && self.cell == cell
    }
}

/// A change the user can make to a board.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Edit {
    Move { dq: i32, dr: i32 },
    PlaceStart,
    PlaceArrow,
    Remove,
}

/// What `Board::apply` returns: the new board, and a note for the log
/// saying what happened. When nothing could be done, `board` equals the
/// board `apply` was called on.
pub(crate) struct Outcome {
    pub(crate) board: Board,
    pub(crate) note: String,
}

/// Everything on screen that undo and redo bring back.
///
/// A `Board` never changes after it is made: it has no `&mut self` method.
/// `apply` makes a new board instead, so an old board stays exactly as it
/// was, and undo is just "show the previous board again" (see `History`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Board {
    radius: i32,
    region: Region,
    selected: (i32, i32),
    nodes: Vec<PlacedNode>,
}

impl Board {
    /// An empty board with the center cell selected.
    pub(crate) const fn new(radius: i32) -> Self {
        Self {
            radius,
            region: Region::Center,
            selected: CENTER_CELL,
            nodes: Vec::new(),
        }
    }

    // --- queries: read the board, never change it ---

    pub(crate) const fn region(&self) -> Region {
        self.region
    }

    pub(crate) const fn selected(&self) -> (i32, i32) {
        self.selected
    }

    /// The nodes of every region, in the order they were placed.
    pub(crate) fn nodes(&self) -> &[PlacedNode] {
        &self.nodes
    }

    /// The node on `cell` of the current region.
    pub(crate) fn node_at(&self, cell: (i32, i32)) -> Option<Node> {
        for placed in &self.nodes {
            if placed.is_at(self.region, cell) {
                return Some(placed.node);
            }
        }
        None
    }

    /// Whether the start node is placed, in any region.
    fn has_start_node(&self) -> bool {
        for placed in &self.nodes {
            if placed.node == Node::BpmnStartNote {
                return true;
            }
        }
        false
    }

    pub(crate) fn in_bounds(&self, cell: (i32, i32)) -> bool {
        let (q, r) = cell;
        q.abs().max(r.abs()).max((q + r).abs()) <= self.radius
    }

    /// `q`-major.
    pub(crate) fn cells(&self) -> Vec<(i32, i32)> {
        let mut cells = Vec::new();
        for q in -self.radius..=self.radius {
            for r in -self.radius..=self.radius {
                if self.in_bounds((q, r)) {
                    cells.push((q, r));
                }
            }
        }
        cells
    }
}
