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
