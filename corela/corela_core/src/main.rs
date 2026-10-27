mod config;
mod model;

use std::path::{Path, PathBuf};

use corela_gfx::{App, Frame, HexLayout, Window};

use crate::config::load_config;
use model::{Config, Event, Flow, Gfx};

const NAME: &str = env!("CARGO_BIN_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");

fn belongs(path: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    let Ok(table) = text.parse::<toml::Table>() else {
        return false;
    };
    let meta = |key: &str| {
        table
            .get("meta")
            .and_then(|t| t.get(key))
            .and_then(|v| v.as_str())
    };
    meta("name") == Some(NAME) && meta("version") == Some(VERSION)
}

fn find_regina() -> PathBuf {
    let exe = std::env::current_exe().expect("cannot locate the corela executable");
    let found = exe
        .ancestors()
        .skip(1)
        .take(3)
        .map(|dir| dir.join("regina.toml"))
        .find(|path| belongs(path));
    found.unwrap_or_else(|| {
        log::error!(
            "corela: no regina.toml for {NAME} {VERSION} within 3 directories of {}",
            exe.display()
        );
        std::process::exit(1)
    })
}

struct Corela {
    config: Config,
    gfx: Gfx,
    layout: HexLayout,
}

impl App for Corela {
    fn frame(&mut self, frame: &mut Frame) {
        let config = &self.config;
        let (cell_q, cell_r) = config.cell;
        let selected = self.gfx.selected();
        for (label, (q, r)) in (0..).zip(self.gfx.cells()) {
            let at = self.layout.to_pixel(q, r);
            let active = (q, r) == selected;
            let color = if active { config.selected } else { config.tile };
            frame.hexagon(at, self.layout.size - 2.0, color);
            let occupied = (q, r) == config.cell
                || config
                    .chevron_offsets
                    .iter()
                    .any(|&(dq, dr)| (q, r) == (cell_q + dq, cell_r + dr));
            if !occupied {
                frame.label(label, at, color, active);
            }
        }

        let center = self.layout.to_pixel(cell_q, cell_r);
        frame.circle(
            center,
            config.cell_marker_outer_radius,
            config.cell_marker_border_color,
        );
        frame.circle(
            center,
            config.cell_marker_inner_radius,
            config.cell_marker_fill_color,
        );
        for &(dq, dr) in &config.chevron_offsets {
            let at = self.layout.to_pixel(cell_q + dq, cell_r + dr);
            frame.double_chevron_right(
                at,
                (at.0 - center.0, at.1 - center.1),
                config.chevron_color,
            );
        }
    }

    fn key_down(&mut self, key_code: i32) {
        let config = &self.config;
        let event = match key_code {
            k if k == config.key_w => Event::Move { dq: 0, dr: -1 },
            k if k == config.key_s => Event::Move { dq: 0, dr: 1 },
            k if k == config.key_e => Event::Move { dq: 1, dr: -1 },
            k if k == config.key_d => Event::Move { dq: 1, dr: 0 },
            k if k == config.key_q => Event::Move { dq: -1, dr: 0 },
            k if k == config.key_a => Event::Move { dq: -1, dr: 1 },
            k if k == config.key_escape => Event::Quit,
            _ => return,
        };
        if self.gfx.handle(event) == Flow::Quit {
            corela_gfx::quit();
        }
    }
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let path = find_regina();
    let config = load_config(&path);
    log::info!("{} {} ({})", config.name, config.version, path.display());

    let window = Window {
        title: c"corela",
        width: config.window_width,
        height: config.window_height,
        clear: config.base,
    };
    let layout = HexLayout {
        origin: (
            config.window_width as f32 / 2.0,
            config.window_height as f32 / 2.0,
        ),
        size: config.size,
    };
    let gfx = Gfx::new(config.radius);
    corela_gfx::run(
        &window,
        Corela {
            config,
            gfx,
            layout,
        },
    );
}
