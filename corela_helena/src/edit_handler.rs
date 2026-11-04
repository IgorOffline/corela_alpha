use corela_model::entity::{Board, Node, PlacedNode, Region};
use corela_model::message::Edit;

const CENTER_GATE: (i32, i32) = (0, -4);
const NORTH_GATE: (i32, i32) = (0, 4);
const GATE_NORTH_STEP: (i32, i32) = (0, -1);
const GATE_SOUTH_STEP: (i32, i32) = (0, 1);
const ARROW_DIRECTION_COUNT: u8 = 8;

pub(crate) struct Outcome {
    pub(crate) board: Board,
    pub(crate) note: String,
}

pub(crate) fn apply(board: &Board, edit: Edit) -> Result<Outcome, String> {
    match edit {
        Edit::Move { dq, dr } => moved(board, (dq, dr)),
        Edit::PlaceStart => with_start(board),
        Edit::PlaceArrow => with_arrow(board),
        Edit::Remove => without_selected(board),
    }
}

fn moved(board: &Board, step: (i32, i32)) -> Result<Outcome, String> {
    let region = board.region();
    let selected = board.selected();
    if region == Region::Center && selected == CENTER_GATE && step == GATE_NORTH_STEP {
        return Ok(Outcome {
            board: board.with_selection(Region::North, NORTH_GATE),
            note: String::from("entered_north_region"),
        });
    }
    if region == Region::North && selected == NORTH_GATE && step == GATE_SOUTH_STEP {
        return Ok(Outcome {
            board: board.with_selection(Region::Center, CENTER_GATE),
            note: String::from("entered_center_region"),
        });
    }
    let (q, r) = (selected.0 + step.0, selected.1 + step.1);
    if !board.in_bounds((q, r)) {
        return Err(format!("{q}_{r}_is_out_of_bounds"));
    }
    Ok(Outcome {
        board: board.with_selection(region, (q, r)),
        note: format!("selected_{q}_{r}"),
    })
}

fn with_start(board: &Board) -> Result<Outcome, String> {
    let selected = board.selected();
    let (q, r) = selected;
    if board.node_at(selected).is_some() {
        return Err(format!("{q}_{r}_is_taken_no_start_node"));
    }
    if board.has_start_node() {
        return Err(String::from("the_project_already_has_a_start_node"));
    }
    let mut nodes = board.nodes().to_vec();
    nodes.push(PlacedNode::new(
        board.region(),
        selected,
        Node::BpmnStartNote,
    ));
    Ok(Outcome {
        board: board.with_nodes(nodes),
        note: format!("placed_start_node_at_{q}_{r}"),
    })
}

fn with_arrow(board: &Board) -> Result<Outcome, String> {
    let selected = board.selected();
    let (q, r) = selected;
    let direction = match board.node_at(selected) {
        Some(Node::Arrow(direction)) => direction,
        Some(Node::BpmnStartNote) => {
            return Err(format!("{q}_{r}_holds_the_start_node_no_arrow"));
        }
        None => {
            let mut nodes = board.nodes().to_vec();
            nodes.push(PlacedNode::new(board.region(), selected, Node::Arrow(0)));
            return Ok(Outcome {
                board: board.with_nodes(nodes),
                note: format!("placed_arrow_at_{q}_{r}"),
            });
        }
    };
    let turned = (direction + 1) % ARROW_DIRECTION_COUNT;
    let mut nodes = Vec::new();
    for placed in board.nodes() {
        if placed.is_at(board.region(), selected) {
            nodes.push(PlacedNode::new(
                placed.region(),
                placed.cell(),
                Node::Arrow(turned),
            ));
        } else {
            nodes.push(*placed);
        }
    }
    Ok(Outcome {
        board: board.with_nodes(nodes),
        note: format!("turned_arrow_at_{q}_{r}_to_{turned}"),
    })
}

fn without_selected(board: &Board) -> Result<Outcome, String> {
    let selected = board.selected();
    let (q, r) = selected;
    if board.node_at(selected).is_none() {
        return Err(format!("nothing_to_remove_at_{q}_{r}"));
    }
    let mut kept = Vec::new();
    for placed in board.nodes() {
        if !placed.is_at(board.region(), selected) {
            kept.push(*placed);
        }
    }
    Ok(Outcome {
        board: board.with_nodes(kept),
        note: format!("removed_node_at_{q}_{r}"),
    })
}
