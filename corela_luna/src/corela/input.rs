use corela_adahira::Key;
use corela_ema::message::{Command, Edit};

use super::Corela;
use crate::entity::InputMode;

const NORTH: (i32, i32) = (0, -1);
const NORTH_EAST: (i32, i32) = (1, -1);
const SOUTH_EAST: (i32, i32) = (1, 0);
const SOUTH: (i32, i32) = (0, 1);
const SOUTH_WEST: (i32, i32) = (-1, 1);
const NORTH_WEST: (i32, i32) = (-1, 0);

const fn move_command(direction: (i32, i32)) -> Command {
    let (dq, dr) = direction;
    Command::Edit(Edit::Move { dq, dr })
}

impl Corela {
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

    fn return_to_none_mode(&mut self) {
        if self.playback_timer.is_some() {
            self.stop_playback();
        }
        self.mode = InputMode::None;
        self.log_action("input_mode_none");
    }

    pub(super) fn handle_key(&mut self, key: Key) {
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
