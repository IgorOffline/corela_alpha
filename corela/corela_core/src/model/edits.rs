use super::{Board, Edit, Node, Outcome, PlacedNode, Region};

const CENTER_GATE: (i32, i32) = (0, -4);
const NORTH_GATE: (i32, i32) = (0, 4);
const GATE_NORTH_STEP: (i32, i32) = (0, -1);
const GATE_SOUTH_STEP: (i32, i32) = (0, 1);
const ARROW_DIRECTIONS: u8 = 8;

impl Board {
    pub(crate) fn apply(&self, edit: Edit) -> Outcome {
        match edit {
            Edit::Move { dq, dr } => self.moved((dq, dr)),
            Edit::PlaceStart => self.with_start(),
            Edit::PlaceArrow => self.with_arrow(),
            Edit::Remove => self.without_selected(),
        }
    }

    fn with_selection(&self, region: Region, selected: (i32, i32)) -> Self {
        Self {
            radius: self.radius,
            region,
            selected,
            nodes: self.nodes.clone(),
        }
    }

    const fn with_nodes(&self, nodes: Vec<PlacedNode>) -> Self {
        Self {
            radius: self.radius,
            region: self.region,
            selected: self.selected,
            nodes,
        }
    }

    fn unchanged(&self, note: String) -> Outcome {
        Outcome {
            board: self.clone(),
            note,
        }
    }

    fn moved(&self, step: (i32, i32)) -> Outcome {
        if self.region == Region::Center && self.selected == CENTER_GATE && step == GATE_NORTH_STEP
        {
            return Outcome {
                board: self.with_selection(Region::North, NORTH_GATE),
                note: String::from("entered North region"),
            };
        }
        if self.region == Region::North && self.selected == NORTH_GATE && step == GATE_SOUTH_STEP {
            return Outcome {
                board: self.with_selection(Region::Center, CENTER_GATE),
                note: String::from("entered Center region"),
            };
        }
        let (q, r) = (self.selected.0 + step.0, self.selected.1 + step.1);
        if !self.in_bounds((q, r)) {
            return self.unchanged(format!("({q}, {r}) is out of bounds"));
        }
        Outcome {
            board: self.with_selection(self.region, (q, r)),
            note: format!("selected ({q}, {r})"),
        }
    }

    fn with_start(&self) -> Outcome {
        let (q, r) = self.selected;
        if self.node_at(self.selected).is_some() {
            return self.unchanged(format!("({q}, {r}) is taken, no start node"));
        }
        if self.has_start_node() {
            return self.unchanged(String::from("the project already has a start node"));
        }
        let mut nodes = self.nodes.clone();
        nodes.push(PlacedNode {
            region: self.region,
            cell: self.selected,
            node: Node::BpmnStartNote,
        });
        Outcome {
            board: self.with_nodes(nodes),
            note: format!("placed start node at ({q}, {r})"),
        }
    }

    fn with_arrow(&self) -> Outcome {
        let (q, r) = self.selected;
        let direction = match self.node_at(self.selected) {
            Some(Node::Arrow(direction)) => direction,
            Some(Node::BpmnStartNote) => {
                return self.unchanged(format!("({q}, {r}) holds the start node, no arrow"));
            }
            None => {
                let mut nodes = self.nodes.clone();
                nodes.push(PlacedNode {
                    region: self.region,
                    cell: self.selected,
                    node: Node::Arrow(0),
                });
                return Outcome {
                    board: self.with_nodes(nodes),
                    note: format!("placed arrow at ({q}, {r})"),
                };
            }
        };
        let turned = (direction + 1) % ARROW_DIRECTIONS;
        let mut nodes = Vec::new();
        for placed in &self.nodes {
            if placed.is_at(self.region, self.selected) {
                nodes.push(PlacedNode {
                    region: placed.region,
                    cell: placed.cell,
                    node: Node::Arrow(turned),
                });
            } else {
                nodes.push(*placed);
            }
        }
        Outcome {
            board: self.with_nodes(nodes),
            note: format!("turned arrow at ({q}, {r}) to {turned}"),
        }
    }

    fn without_selected(&self) -> Outcome {
        let (q, r) = self.selected;
        if self.node_at(self.selected).is_none() {
            return self.unchanged(format!("nothing to remove at ({q}, {r})"));
        }
        let mut kept = Vec::new();
        for placed in &self.nodes {
            if !placed.is_at(self.region, self.selected) {
                kept.push(*placed);
            }
        }
        Outcome {
            board: self.with_nodes(kept),
            note: format!("removed node at ({q}, {r})"),
        }
    }
}
