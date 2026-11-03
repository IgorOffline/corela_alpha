mod config;
mod edits;

pub(crate) use config::{Config, Theme};

const CENTER_CELL: (i32, i32) = (0, 0);
const COLUMN_LETTERS: u8 = 26;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Region {
    Center,
    North,
}

impl Region {
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Center => "Center",
            Self::North => "North",
        }
    }
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Edit {
    Move { dq: i32, dr: i32 },
    PlaceStart,
    PlaceArrow,
    Remove,
}

pub(crate) struct Outcome {
    pub(crate) board: Board,
    pub(crate) note: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Board {
    radius: i32,
    region: Region,
    selected: (i32, i32),
    nodes: Vec<PlacedNode>,
}

impl Board {
    pub(crate) const fn new(radius: i32) -> Self {
        Self {
            radius,
            region: Region::Center,
            selected: CENTER_CELL,
            nodes: Vec::new(),
        }
    }

    pub(crate) const fn region(&self) -> Region {
        self.region
    }

    pub(crate) const fn selected(&self) -> (i32, i32) {
        self.selected
    }

    pub(crate) fn nodes(&self) -> &[PlacedNode] {
        &self.nodes
    }

    pub(crate) fn node_at(&self, cell: (i32, i32)) -> Option<Node> {
        for placed in &self.nodes {
            if placed.is_at(self.region, cell) {
                return Some(placed.node);
            }
        }
        None
    }

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

    pub(crate) fn label(&self, cell: (i32, i32)) -> String {
        let (q, r) = cell;
        let column = match u8::try_from(q + self.radius) {
            Ok(column) if column < COLUMN_LETTERS => char::from(b'A' + column),
            _ => '?',
        };
        let bottom = self.radius.min(self.radius - q);
        format!("{column}{}", bottom - r + 1)
    }

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
