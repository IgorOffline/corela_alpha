#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Move { dq: i32, dr: i32 },
    Quit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flow {
    Continue,
    Quit,
}

pub struct Gfx {
    radius: i32,
    selected: (i32, i32),
}

impl Gfx {
    pub fn new(radius: i32) -> Self {
        Self {
            radius,
            selected: (0, 0),
        }
    }

    pub fn selected(&self) -> (i32, i32) {
        self.selected
    }

    pub fn in_bounds(&self, (q, r): (i32, i32)) -> bool {
        q.abs().max(r.abs()).max((q + r).abs()) <= self.radius
    }

    /// Every cell of the board, `q`-major.
    pub fn cells(&self) -> impl Iterator<Item = (i32, i32)> + use<'_> {
        let radius = self.radius;
        (-radius..=radius)
            .flat_map(move |q| (-radius..=radius).map(move |r| (q, r)))
            .filter(|&cell| self.in_bounds(cell))
    }

    pub fn handle(&mut self, event: Event) -> Flow {
        match event {
            Event::Move { dq, dr } => {
                let cell = (self.selected.0 + dq, self.selected.1 + dr);
                if self.in_bounds(cell) {
                    self.selected = cell;
                    log::info!("core: selected ({}, {})", cell.0, cell.1);
                }
                Flow::Continue
            }
            Event::Quit => {
                log::info!("core: quit");
                Flow::Quit
            }
        }
    }
}

pub struct Config {
    pub name: String,
    pub version: String,
    pub cell: (i32, i32),
    pub key_escape: i32,
    pub key_q: i32,
    pub key_w: i32,
    pub key_e: i32,
    pub key_a: i32,
    pub key_s: i32,
    pub key_d: i32,
    pub window_width: i32,
    pub window_height: i32,
    pub radius: i32,
    pub size: f32,
    pub base: u32,
    pub tile: u32,
    pub selected: u32,
    pub chevron_offsets: Vec<(i32, i32)>,
    pub cell_marker_outer_radius: f32,
    pub cell_marker_inner_radius: f32,
    pub cell_marker_border_color: u32,
    pub cell_marker_fill_color: u32,
    pub chevron_color: u32,
}
