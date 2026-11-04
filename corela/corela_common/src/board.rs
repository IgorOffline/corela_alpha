use crate::node::Node;
use crate::placed_node::PlacedNode;
use crate::region::Region;

const CENTER_CELL: (i32, i32) = (0, 0);
const COLUMN_LETTERS: u8 = 26;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Board {
    radius: i32,
    region: Region,
    selected: (i32, i32),
    nodes: Vec<PlacedNode>,
}

impl Board {
    #[must_use]
    pub const fn new(radius: i32) -> Self {
        Self {
            radius,
            region: Region::Center,
            selected: CENTER_CELL,
            nodes: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_selection(&self, region: Region, selected: (i32, i32)) -> Self {
        Self {
            radius: self.radius,
            region,
            selected,
            nodes: self.nodes.clone(),
        }
    }

    #[must_use]
    pub const fn with_nodes(&self, nodes: Vec<PlacedNode>) -> Self {
        Self {
            radius: self.radius,
            region: self.region,
            selected: self.selected,
            nodes,
        }
    }

    #[must_use]
    pub const fn region(&self) -> Region {
        self.region
    }

    #[must_use]
    pub const fn selected(&self) -> (i32, i32) {
        self.selected
    }

    #[must_use]
    pub fn nodes(&self) -> &[PlacedNode] {
        &self.nodes
    }

    #[must_use]
    pub fn node_at(&self, cell: (i32, i32)) -> Option<Node> {
        for placed in &self.nodes {
            if placed.is_at(self.region, cell) {
                return Some(placed.node());
            }
        }
        None
    }

    #[must_use]
    pub fn has_start_node(&self) -> bool {
        for placed in &self.nodes {
            if placed.node() == Node::BpmnStartNote {
                return true;
            }
        }
        false
    }

    #[must_use]
    pub fn in_bounds(&self, cell: (i32, i32)) -> bool {
        let (q, r) = cell;
        q.abs().max(r.abs()).max((q + r).abs()) <= self.radius
    }

    #[must_use]
    pub fn label(&self, cell: (i32, i32)) -> String {
        let (q, r) = cell;
        let column = match u8::try_from(q + self.radius) {
            Ok(column) if column < COLUMN_LETTERS => char::from(b'A' + column),
            _ => '?',
        };
        let bottom = self.radius.min(self.radius - q);
        format!("{column}{}", bottom - r + 1)
    }

    #[must_use]
    pub fn cells(&self) -> Vec<(i32, i32)> {
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
