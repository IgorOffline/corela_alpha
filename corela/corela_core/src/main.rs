mod model;

use std::path::{Path, PathBuf};

use corela_gfx::{App, Frame, HexLayout, Window};

use model::{Event, Flow, Gfx};

const CHEVRON_OFFSETS: [(i32, i32); 6] = [(1, -1), (1, 0), (0, 1), (-1, 1), (-1, 0), (0, -1)];

struct Config {
    name: String,
    version: String,
    cell: (i32, i32),
    key_escape: i32,
    key_q: i32,
    key_w: i32,
    key_e: i32,
    key_a: i32,
    key_s: i32,
    key_d: i32,
    window_width: i32,
    window_height: i32,
    radius: i32,
    size: f32,
    base: u32,
    tile: u32,
    selected: u32,
}

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

fn load_config(path: &Path) -> Config {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let table: toml::Table = text
        .parse()
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let value = |section: &str, key: &str| {
        table
            .get(section)
            .and_then(|t| t.get(key))
            .unwrap_or_else(|| panic!("{}: missing [{section}] {key}", path.display()))
    };
    let int = |section: &str, key: &str| {
        value(section, key)
            .as_integer()
            .unwrap_or_else(|| panic!("{}: [{section}] {key} must be an integer", path.display()))
    };
    let str = |section: &str, key: &str| {
        value(section, key)
            .as_str()
            .unwrap_or_else(|| panic!("{}: [{section}] {key} must be a string", path.display()))
    };
    let color = |role: &str| int("color", &format!("{}_value", str("gfx", role))) as u32;
    Config {
        name: str("meta", "name").to_owned(),
        version: str("meta", "version").to_owned(),
        cell: (int("bpmn", "cell_x") as i32, int("bpmn", "cell_y") as i32),
        key_escape: int("gfx", "input_key_escape") as i32,
        key_q: int("gfx", "input_key_q") as i32,
        key_w: int("gfx", "input_key_w") as i32,
        key_e: int("gfx", "input_key_e") as i32,
        key_a: int("gfx", "input_key_a") as i32,
        key_s: int("gfx", "input_key_s") as i32,
        key_d: int("gfx", "input_key_d") as i32,
        window_width: int("gfx", "window_width") as i32,
        window_height: int("gfx", "window_height") as i32,
        radius: int("gfx", "hex_radius") as i32,
        size: int("gfx", "hex_size") as f32,
        base: color("color_base"),
        tile: color("color_tile"),
        selected: color("color_selected"),
    }
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
                || CHEVRON_OFFSETS
                    .iter()
                    .any(|&(dq, dr)| (q, r) == (cell_q + dq, cell_r + dr));
            if !occupied {
                frame.label(label, at, color, active);
            }
        }

        let center = self.layout.to_pixel(cell_q, cell_r);
        frame.circle(center, 15.75, 0x232634);
        frame.circle(center, 14.25, 0xffffff);
        for (dq, dr) in CHEVRON_OFFSETS {
            let at = self.layout.to_pixel(cell_q + dq, cell_r + dr);
            frame.double_chevron_right(at, (at.0 - center.0, at.1 - center.1), 0xef9f76);
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
