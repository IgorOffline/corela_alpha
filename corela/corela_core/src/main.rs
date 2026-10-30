mod config;
mod model;

use std::ffi::CStr;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use corela_gfx::{App, Frame, HexLayout, Window};

use crate::config::{CONFIG_FILE_NAME, KEY_NAME, KEY_VERSION, SECTION_META, load_config};
use model::{Config, Event, Flow, Gfx, Node, Theme};

const NAME: &str = env!("CARGO_BIN_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");
const WINDOW_TITLE: &CStr = c"corela";
const DEFAULT_LOG_LEVEL: &str = "info";
/// How many parent directories of the executable are searched for the config.
const CONFIG_SEARCH_DEPTH: usize = 3;

// Axial (q, r) steps to each neighbor of a flat-top hex; screen y grows downwards.
const NORTH: (i32, i32) = (0, -1);
const NORTH_EAST: (i32, i32) = (1, -1);
const SOUTH_EAST: (i32, i32) = (1, 0);
const SOUTH: (i32, i32) = (0, 1);
const SOUTH_WEST: (i32, i32) = (-1, 1);
const NORTH_WEST: (i32, i32) = (-1, 0);

/// `[meta] key` as a string, if the table has one.
fn meta_string(table: &toml::Table, key: &str) -> Option<String> {
    let section = table.get(SECTION_META)?;
    let value = section.get(key)?;
    let text = value.as_str()?;
    Some(String::from(text))
}

/// Whether `path` is a readable config file written for this name and version.
fn belongs(path: &Path) -> bool {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(_) => return false,
    };
    let table = match text.parse::<toml::Table>() {
        Ok(table) => table,
        Err(_) => return false,
    };
    let name = meta_string(&table, KEY_NAME);
    let version = meta_string(&table, KEY_VERSION);
    name == Some(String::from(NAME)) && version == Some(String::from(VERSION))
}

/// Looks for the config file in the executable's directory and its parents.
///
/// # Errors
/// If the executable's path is unknown, or no matching config is found
/// within `CONFIG_SEARCH_DEPTH` directories.
fn find_regina() -> Result<PathBuf, String> {
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(error) => return Err(format!("cannot locate the corela executable: {error}")),
    };
    let mut directory = exe.parent();
    for _ in 0..CONFIG_SEARCH_DEPTH {
        let current = match directory {
            Some(current) => current,
            None => break,
        };
        let candidate = current.join(CONFIG_FILE_NAME);
        if belongs(&candidate) {
            return Ok(candidate);
        }
        directory = current.parent();
    }
    Err(format!(
        "no {CONFIG_FILE_NAME} for {NAME} {VERSION} within {CONFIG_SEARCH_DEPTH} directories of {}",
        exe.display()
    ))
}

const fn move_event(direction: (i32, i32)) -> Event {
    let (dq, dr) = direction;
    Event::Move { dq, dr }
}

struct Corela {
    config: Config,
    gfx: Gfx,
    layout: HexLayout,
    geometry: corela_gfx::Geometry,
    dark: bool,
}

impl Corela {
    const fn theme(&self) -> &Theme {
        if self.dark {
            &self.config.dark_theme
        } else {
            &self.config.light_theme
        }
    }
}

impl App for Corela {
    fn frame(&mut self, frame: &mut Frame) {
        let config = &self.config;
        let theme = self.theme();
        let inverted_theme = if self.dark {
            &config.light_theme
        } else {
            &config.dark_theme
        };
        frame.clear(theme.base_color);
        frame.footer(self.gfx.region().name(), theme.label_text_color);
        let selected = self.gfx.selected();
        let cells = self.gfx.cells();
        for (index, cell) in cells.iter().enumerate() {
            let (q, r) = *cell;
            // labels are numbered in cells() order; a radius-4 board has 61
            let label = match i32::try_from(index) {
                Ok(label) => label,
                Err(_) => break,
            };
            let at = self.layout.to_pixel(q, r);
            let active = (q, r) == selected;
            let color = if active {
                theme.selected_color
            } else {
                theme.tile_color
            };
            let radius = self.layout.size - self.geometry.hex_tile_inset;
            frame.hexagon(at, radius, color);
            // the marker and its chevrons cover these cells' labels
            if self.gfx.node_at((q, r)).is_none() {
                let text_color = if active {
                    inverted_theme.label_text_color
                } else {
                    theme.label_text_color
                };
                frame.label(label, at, color, text_color, active);
            }
        }

        for cell in cells {
            let at = self.layout.to_pixel(cell.0, cell.1);
            match self.gfx.node_at(cell) {
                Some(Node::BpmnStartNote) => {
                    frame.circle(
                        at,
                        self.geometry.cell_marker_outer_radius,
                        theme.cell_marker_border_color,
                    );
                    frame.circle(
                        at,
                        self.geometry.cell_marker_inner_radius,
                        theme.cell_marker_fill_color,
                    );
                }
                Some(Node::Arrow(direction)) => {
                    let angle = f32::from(direction) * std::f32::consts::FRAC_PI_4;
                    let color = if cell == selected {
                        inverted_theme.chevron_color
                    } else {
                        theme.chevron_color
                    };
                    frame.icon(
                        &config.chevron_icon,
                        at,
                        (angle.cos(), angle.sin()),
                        self.geometry.chevron_size,
                        color,
                    );
                }
                None => {}
            }
        }
    }

    fn key_down(&mut self, key_code: i32) {
        let config = &self.config;
        if key_code == config.key_theme {
            self.dark = !self.dark;
            log::info!("core: {} theme", if self.dark { "dark" } else { "light" });
            return;
        }
        let event = if key_code == config.key_start {
            Event::PlaceStart
        } else if key_code == config.key_arrow {
            Event::PlaceArrow
        } else if key_code == config.key_remove {
            Event::Remove
        } else if key_code == config.key_north {
            move_event(NORTH)
        } else if key_code == config.key_south {
            move_event(SOUTH)
        } else if key_code == config.key_north_east {
            move_event(NORTH_EAST)
        } else if key_code == config.key_south_east {
            move_event(SOUTH_EAST)
        } else if key_code == config.key_north_west {
            move_event(NORTH_WEST)
        } else if key_code == config.key_south_west {
            move_event(SOUTH_WEST)
        } else if key_code == config.key_escape {
            Event::Quit
        } else {
            return;
        };
        if self.gfx.handle(event) == Flow::Quit {
            corela_gfx::quit();
        }
    }
}

fn main() -> ExitCode {
    // RUST_LOG overrides the default level
    let log_settings = env_logger::Env::default().default_filter_or(DEFAULT_LOG_LEVEL);
    env_logger::Builder::from_env(log_settings).init();

    match start() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            log::error!("corela: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Finds and loads the config, then runs the app until its window closes.
///
/// # Errors
/// The first error from `find_regina` or `load_config`; nothing has been
/// shown on screen yet when it returns one.
fn start() -> Result<(), String> {
    let path = find_regina()?;
    let config = load_config(&path)?;
    log::info!("{} v{}", config.name, config.version);
    log::debug!("Configuration: {}", path.display());

    let window = Window {
        title: WINDOW_TITLE,
        width: config.window_width,
        height: config.window_height,
        clear: config.dark_theme.base_color,
    };
    let geometry = corela_gfx::geometry();
    let layout = HexLayout {
        origin: (
            corela_gfx::small_to_f32(config.window_width) / 2.0,
            corela_gfx::small_to_f32(config.window_height) / 2.0,
        ),
        size: geometry.hex_size,
    };
    let gfx = Gfx::new(geometry.hex_radius);
    let app = Corela {
        config,
        gfx,
        layout,
        geometry,
        dark: true,
    };
    corela_gfx::run(&window, Box::new(app));
    Ok(())
}
