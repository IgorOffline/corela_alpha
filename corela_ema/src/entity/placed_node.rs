use crate::entity::{Node, Region};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlacedNode {
    region: Region,
    cell: (i32, i32),
    node: Node,
}

impl PlacedNode {
    #[must_use]
    pub const fn new(region: Region, cell: (i32, i32), node: Node) -> Self {
        Self { region, cell, node }
    }

    #[must_use]
    pub const fn region(&self) -> Region {
        self.region
    }

    #[must_use]
    pub const fn cell(&self) -> (i32, i32) {
        self.cell
    }

    #[must_use]
    pub const fn node(&self) -> Node {
        self.node
    }

    #[must_use]
    pub fn is_at(&self, region: Region, cell: (i32, i32)) -> bool {
        self.region == region && self.cell == cell
    }
}
