mod config;
mod dto;

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use corela_gfx::{App, Frame, HexLayout, Key, Window};
use corela_helena::History;
use corela_historia::load::{find_latest_replay, load_replay};
use corela_historia::save::save_replay;
use corela_historia::{PlaybackTimer, Replay};
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
    Input,
    Undo,
    Redo,
    Save,
    Load,
}

impl InputMode {
    const fn next(self) -> Self {
        match self {
            Self::None => Self::Input,
            Self::Input => Self::Undo,
            Self::Undo => Self::Redo,
            Self::Redo => Self::Save,
            Self::Save => Self::Load,
            Self::Load => Self::None,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Input => "Input",
            Self::Undo => "Undo",
            Self::Redo => "Redo",
            Self::Save => "Save",
            Self::Load => "Load",
        }
    }

    const fn code(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Input => "input",
            Self::Undo => "undo",
            Self::Redo => "redo",
            Self::Save => "save",
            Self::Load => "load",
        }
    }
}

struct Corela {
    config: Config,
    history: History,
    replay: Replay,
    playback_timer: Option<PlaybackTimer>,
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

    fn log_warning(&self, text: &str) {
        log::warn!(
            "core: action_{}_of_{}: {text}",
            self.replay.current_action(),
            self.replay.last_action()
        );
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

    fn run_mode(&mut self) {
        match self.mode {
            InputMode::None | InputMode::Input => {}
            InputMode::Undo => self.run_command(Command::Undo),
            InputMode::Redo => self.run_command(Command::Redo),
            InputMode::Save => self.save_if_any_action(),
            InputMode::Load => self.start_playback(),
        }
    }

    fn run_command(&mut self, command: Command) {
        let note = match self.history.execute(command) {
            Ok(note) => note,
            Err(warning) => {
                self.log_warning(&warning);
                return;
            }
        };
        self.replay
            .record(command, note.clone(), self.history.snapshot());
        self.log_action(&note);
        if self.replay.is_complete() {
            self.save();
            self.quit();
        }
    }

    fn save_if_any_action(&self) {
        if self.replay.has_actions() {
            self.save();
        } else {
            self.log_warning("nothing_to_save");
        }
    }

    fn save(&self) {
        let config = &self.config;
        match save_replay(
            &self.replay,
            &config.replay_dir,
            &config.name,
            &config.version,
        ) {
            Ok(path) => self.log_action(&format!("saved_replay_{}", path.display())),
            Err(error) => log::error!("core: replay_not_saved: {error}"),
        }
    }

    fn start_playback(&mut self) {
        let path = match find_latest_replay(&self.config.replay_dir) {
            Ok(Some(path)) => path,
            Ok(None) => {
                self.log_action("no_replay_to_load");
                self.quit();
                return;
            }
            Err(error) => {
                log::error!("core: replay_not_loaded: {error}");
                self.quit();
                return;
            }
        };
        let empty_board = Board::new(self.geometry.hex_radius);
        match load_replay(&path, &empty_board) {
            Ok(replay) => {
                self.replay = replay;
                self.history = History::new(empty_board);
                self.playback_timer = Some(PlaybackTimer::new(Instant::now()));
                self.log_action(&format!("loaded_replay_{}", path.display()));
            }
            Err(error) => {
                log::error!("core: replay_not_loaded: {error}");
                self.quit();
            }
        }
    }

    fn play_due_action(&mut self) {
        let timer = match &mut self.playback_timer {
            Some(timer) => timer,
            None => return,
        };
        if !timer.poll(Instant::now()) {
            return;
        }
        if let Some(action) = self.replay.action_to_play() {
            self.history = History::new(action.board().clone());
            let note = action.note().to_owned();
            self.replay.finish_playing_action();
            self.log_action(&note);
        } else {
            self.log_action("replay_finished");
            self.quit();
        }
    }

    fn return_to_none_mode(&mut self) {
        if self.playback_timer.is_some() {
            self.stop_playback();
        }
        self.mode = InputMode::None;
        self.log_action("input_mode_none");
    }

    fn stop_playback(&mut self) {
        self.log_action("playback_stopped");
        self.playback_timer = None;
        self.replay = Replay::new(self.config.last_action);
        self.history = History::new(Board::new(self.geometry.hex_radius));
    }

    fn quit(&self) {
        self.log_action("quit");
        corela_gfx::quit();
    }
}

impl App for Corela {
    fn frame(&mut self, frame: &mut Frame<'_>) {
        self.play_due_action();
        let config = &self.config;
        let theme = self.theme();
        let board = self.history.board();
        frame.clear(theme.base_color);
        frame.footer(board.region().name(), theme.label_text_color);
        let mode_label = format!("Mode: {}", self.mode.name());
        frame.above_footer(&mode_label, theme.label_text_color);
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
        if self.mode == InputMode::None && key != config.key_cycle {
            return;
        }
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
            self.return_to_none_mode();
            return;
        }
        if self.playback_timer.is_some() {
            return;
        }
        if key == config.key_cycle {
            self.mode = self.mode.next();
            self.log_action(&format!("input_mode_{}", self.mode.code()));
            return;
        }
        if key == config.key_run {
            self.run_mode();
            return;
        }
        if self.mode == InputMode::Input
            && let Some(command) = self.command_for(key)
        {
            self.run_command(command);
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
        playback_timer: None,
        layout,
        geometry,
        dark: true,
        mode: InputMode::None,
    };
    corela_gfx::run(&window, Box::new(app));
    Ok(())
}
