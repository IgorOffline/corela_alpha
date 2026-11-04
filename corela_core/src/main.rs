mod config;
mod dto;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use corela_gfx::{App, Frame, HexLayout, Key, Window};
use corela_helena::History;
use corela_historia::Replay;
use corela_model::entity::{Board, Node};
use corela_model::message::{Command, Edit};

use crate::config::{CONFIG_FILE_NAME, KEY_NAME, KEY_VERSION, SECTION_META, load_config};
use dto::{Config, Theme};

const NAME: &str = env!("CARGO_BIN_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");
const WINDOW_TITLE: &str = "corela_bpmn";
const DEFAULT_LOG_LEVEL: &str = "info";
const CONFIG_SEARCH_DEPTH: usize = 3;

const NORTH: (i32, i32) = (0, -1);
const NORTH_EAST: (i32, i32) = (1, -1);
const SOUTH_EAST: (i32, i32) = (1, 0);
const SOUTH: (i32, i32) = (0, 1);
const SOUTH_WEST: (i32, i32) = (-1, 1);
const NORTH_WEST: (i32, i32) = (-1, 0);

fn meta_string(table: &toml::Table, key: &str) -> Option<String> {
    let section = table.get(SECTION_META)?;
    let value = section.get(key)?;
    let text = value.as_str()?;
    Some(String::from(text))
}

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

fn find_regina() -> Result<PathBuf, String> {
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(error) => return Err(format!("cannot_locate_the_corela_executable: {error}")),
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
        "no_{CONFIG_FILE_NAME}_for_{NAME}_{VERSION}_within_{CONFIG_SEARCH_DEPTH}_directories_of_{}",
        exe.display()
    ))
}

const fn move_command(direction: (i32, i32)) -> Command {
    let (dq, dr) = direction;
    Command::Edit(Edit::Move { dq, dr })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InputMode {
    None,
    Place,
    Undo,
    Redo,
}

impl InputMode {
    const fn next(self) -> Self {
        match self {
            Self::None => Self::Place,
            Self::Place => Self::Undo,
            Self::Undo => Self::Redo,
            Self::Redo => Self::None,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Place => "place",
            Self::Undo => "undo",
            Self::Redo => "redo",
        }
    }

    const fn command(self) -> Option<Command> {
        match self {
            Self::None => None,
            Self::Place => Some(Command::Edit(Edit::PlaceStart)),
            Self::Undo => Some(Command::Undo),
            Self::Redo => Some(Command::Redo),
        }
    }
}

struct Corela {
    config: Config,
    history: History,
    replay: Replay,
    layout: HexLayout,
    geometry: corela_gfx::Geometry,
    dark: bool,
    mode: InputMode,
}

impl Corela {
    const fn theme(&self) -> &Theme {
        if self.dark {
            &self.config.dark_theme
        } else {
            &self.config.light_theme
        }
    }

    fn log_action(&self, text: &str) {
        log::info!(
            "core: action_{}_of_{}: {text}",
            self.replay.current_action(),
            self.replay.last_action()
        );
    }

    fn command_for(&self, key: Key) -> Option<Command> {
        let config = &self.config;
        if key == config.key_start {
            Some(Command::Edit(Edit::PlaceStart))
        } else if key == config.key_arrow {
            Some(Command::Edit(Edit::PlaceArrow))
        } else if key == config.key_remove {
            Some(Command::Edit(Edit::Remove))
        } else if key == config.key_north {
            Some(move_command(NORTH))
        } else if key == config.key_south {
            Some(move_command(SOUTH))
        } else if key == config.key_north_east {
            Some(move_command(NORTH_EAST))
        } else if key == config.key_south_east {
            Some(move_command(SOUTH_EAST))
        } else if key == config.key_north_west {
            Some(move_command(NORTH_WEST))
        } else if key == config.key_south_west {
            Some(move_command(SOUTH_WEST))
        } else {
            None
        }
    }

    fn finish(&self) {
        let config = &self.config;
        match self
            .replay
            .save(&config.replay_dir, &config.name, &config.version)
        {
            Ok(path) => self.log_action(&format!("saved_replay_{}", path.display())),
            Err(error) => log::error!("core: replay_not_saved: {error}"),
        }
        self.log_action("quit");
        corela_gfx::quit();
    }
}

impl App for Corela {
    fn frame(&mut self, frame: &mut Frame<'_>) {
        let config = &self.config;
        let theme = self.theme();
        let board = self.history.board();
        frame.clear(theme.base_color);
        frame.footer(board.region().name(), theme.label_text_color);
        let counter = format!(
            "Action {} / {}",
            self.replay.current_action(),
            self.replay.last_action()
        );
        frame.header(&counter, theme.label_text_color);
        let selected = board.selected();
        let cells = board.cells();
        let radius = self.layout.size - self.geometry.hex_tile_inset;
        for cell in &cells {
            let (q, r) = *cell;
            let at = self.layout.to_pixel(q, r);
            let active = *cell == selected;
            let color = if active {
                theme.selected_color
            } else {
                theme.tile_color
            };
            frame.hexagon(at, radius, color);
            if board.node_at(*cell).is_none() {
                let text_color = if active {
                    theme.selected_label_text_color
                } else {
                    theme.label_text_color
                };
                frame.label(&board.label(*cell), at, text_color, active);
            }
        }

        for cell in cells {
            let at = self.layout.to_pixel(cell.0, cell.1);
            match board.node_at(cell) {
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
                        theme.selected_chevron_color
                    } else {
                        theme.chevron_color
                    };
                    frame.icon(&config.chevron_icon, at, angle, color);
                }
                None => {}
            }
        }
    }

    fn key_down(&mut self, key: Key) {
        if self.replay.is_complete() {
            return;
        }
        let config = &self.config;
        if key == config.key_theme {
            self.dark = !self.dark;
            self.log_action(if self.dark {
                "dark_theme"
            } else {
                "light_theme"
            });
            return;
        }
        if key == config.key_escape {
            self.log_action("quit");
            corela_gfx::quit();
            return;
        }
        if key == config.key_cycle {
            self.mode = self.mode.next();
            self.log_action(&format!("input_mode_{}", self.mode.name()));
            return;
        }
        let command = if key == config.key_run {
            match self.mode.command() {
                Some(command) if self.history.is_legal(command) => command,
                Some(_) => {
                    self.log_action(&format!("{}_is_not_legal_now", self.mode.name()));
                    return;
                }
                None => return,
            }
        } else {
            match self.command_for(key) {
                Some(command) => command,
                None => return,
            }
        };
        let note = self.history.execute(command);
        self.log_action(&note);
        self.replay.record(command, note, self.history.snapshot());
        if self.replay.is_complete() {
            self.finish();
        }
    }
}

fn main() -> ExitCode {
    let log_environment = env_logger::Env::default().default_filter_or(DEFAULT_LOG_LEVEL);
    env_logger::Builder::from_env(log_environment).init();

    match start() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            log::error!("corela: {error}");
            ExitCode::FAILURE
        }
    }
}

fn start() -> Result<(), String> {
    let path = find_regina()?;
    let config = load_config(&path)?;
    log::info!("{}_v{}", config.name, config.version);
    log::debug!("configuration_{}", path.display());

    let window = Window {
        title: WINDOW_TITLE,
        width: config.window_width,
        height: config.window_height,
    };
    let geometry = corela_gfx::geometry();
    let layout = HexLayout {
        origin: (
            corela_gfx::small_to_f32(config.window_width) / 2.0,
            corela_gfx::small_to_f32(config.window_height) / 2.0,
        ),
        size: geometry.hex_size,
    };
    let history = History::new(Board::new(geometry.hex_radius));
    let replay = Replay::new(config.last_action);
    let app = Corela {
        config,
        history,
        replay,
        layout,
        geometry,
        dark: true,
        mode: InputMode::None,
    };
    corela_gfx::run(&window, Box::new(app));
    Ok(())
}
