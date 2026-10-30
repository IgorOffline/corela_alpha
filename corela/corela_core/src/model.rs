use corela_gfx::Icon;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Event {
    Move { dq: i32, dr: i32 },
    PlaceStart,
    PlaceArrow,
    Remove,
    Quit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Flow {
    Continue,
    Quit,
}

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
}

/// A project may contain only one start node across all regions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Node {
    BpmnStartNote,
    Arrow(u8),
}

#[derive(Clone, Copy, Debug)]
struct PlacedNode {
    region: Region,
    cell: (i32, i32),
    node: Node,
}

impl PlacedNode {
    fn is_at(&self, region: Region, cell: (i32, i32)) -> bool {
        self.region == region && self.cell == cell
    }
}

pub(crate) struct Gfx {
    region: Region,
    radius: i32,
    selected: (i32, i32),
    nodes: Vec<PlacedNode>,
}

impl Gfx {
    pub(crate) const fn new(radius: i32) -> Self {
        Self {
            region: Region::Center,
            radius,
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

    pub(crate) fn node_at(&self, cell: (i32, i32)) -> Option<Node> {
        self.nodes
            .iter()
            .find(|placed| placed.is_at(self.region, cell))
            .map(|placed| placed.node)
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

    pub(crate) fn handle(&mut self, event: Event) -> Flow {
        match event {
            Event::Move { dq, dr } => {
                if self.region == Region::Center && self.selected == (0, -4) && (dq, dr) == (0, -1)
                {
                    self.region = Region::North;
                    self.selected = (0, 4);
                    log::info!("core: entered North region");
                    return Flow::Continue;
                }
                if self.region == Region::North && self.selected == (0, 4) && (dq, dr) == (0, 1) {
                    self.region = Region::Center;
                    self.selected = (0, -4);
                    log::info!("core: entered Center region");
                    return Flow::Continue;
                }
                let cell = (self.selected.0 + dq, self.selected.1 + dr);
                if self.in_bounds(cell) {
                    self.selected = cell;
                    log::info!("core: selected ({}, {})", cell.0, cell.1);
                }
                Flow::Continue
            }
            Event::PlaceStart => {
                if self.node_at(self.selected).is_none()
                    && !self
                        .nodes
                        .iter()
                        .any(|placed| placed.node == Node::BpmnStartNote)
                {
                    self.nodes.push(PlacedNode {
                        region: self.region,
                        cell: self.selected,
                        node: Node::BpmnStartNote,
                    });
                }
                Flow::Continue
            }
            Event::PlaceArrow => {
                if let Some(placed) = self
                    .nodes
                    .iter_mut()
                    .find(|placed| placed.is_at(self.region, self.selected))
                {
                    if let Node::Arrow(direction) = &mut placed.node {
                        *direction = (*direction + 1) % 8;
                    }
                } else {
                    self.nodes.push(PlacedNode {
                        region: self.region,
                        cell: self.selected,
                        node: Node::Arrow(0),
                    });
                }
                Flow::Continue
            }
            Event::Remove => {
                self.nodes
                    .retain(|placed| !placed.is_at(self.region, self.selected));
                Flow::Continue
            }
            Event::Quit => {
                log::info!("core: quit");
                Flow::Quit
            }
        }
    }
}

#[expect(
    clippy::struct_field_names,
    reason = "the suffix matches every other *_color name (report 1000)"
)]
pub(crate) struct Theme {
    pub(crate) base_color: u32,
    pub(crate) tile_color: u32,
    pub(crate) selected_color: u32,
    pub(crate) cell_marker_border_color: u32,
    pub(crate) cell_marker_fill_color: u32,
    pub(crate) chevron_color: u32,
    pub(crate) label_text_color: u32,
}

pub(crate) struct Config {
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) key_start: i32,
    pub(crate) key_arrow: i32,
    pub(crate) key_remove: i32,
    pub(crate) key_escape: i32,
    pub(crate) key_north_west: i32,
    pub(crate) key_north: i32,
    pub(crate) key_north_east: i32,
    pub(crate) key_south_west: i32,
    pub(crate) key_south: i32,
    pub(crate) key_south_east: i32,
    pub(crate) window_width: i32,
    pub(crate) window_height: i32,
    pub(crate) key_theme: i32,
    pub(crate) dark_theme: Theme,
    pub(crate) light_theme: Theme,
    pub(crate) chevron_icon: Icon,
}
