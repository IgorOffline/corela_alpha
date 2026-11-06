mod input;
mod playback;
mod render;

use corela_adahira::{App, Frame, Geometry, HexLayout, Key};
use corela_alexandria::save::save_replay;
use corela_alexandria::{PlaybackTimer, Replay};
use corela_ema::entity::Board;
use corela_ema::message::Command;
use corela_helena::History;

use crate::dto::{Config, Theme};
use crate::entity::InputMode;

pub(crate) struct Corela {
    config: Config,
    history: History,
    replay: Replay,
    playback_timer: Option<PlaybackTimer>,
    layout: HexLayout,
    geometry: Geometry,
    dark: bool,
    mode: InputMode,
}

impl Corela {
    pub(crate) fn new(config: Config) -> Self {
        let geometry = corela_adahira::geometry();
        let layout = HexLayout {
            origin: (
                corela_adahira::small_to_f32(config.window_width) / 2.0,
                corela_adahira::small_to_f32(config.window_height) / 2.0,
            ),
            size: geometry.hex_size,
        };
        let history = History::new(Board::new(geometry.hex_radius));
        let replay = Replay::new(config.last_action);
        Self {
            config,
            history,
            replay,
            playback_timer: None,
            layout,
            geometry,
            dark: true,
            mode: InputMode::None,
        }
    }

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

    fn quit(&self) {
        self.log_action("quit");
        corela_adahira::quit();
    }
}

impl App for Corela {
    fn frame(&mut self, frame: &mut Frame<'_>) {
        self.play_due_action();
        self.draw(frame);
    }

    fn key_down(&mut self, key: Key) {
        self.handle_key(key);
    }
}
