use std::time::Instant;

use corela_alexandria::load::{find_latest_replay, load_replay};
use corela_alexandria::{PlaybackTimer, Replay};
use corela_ema::entity::Board;
use corela_helena::History;

use super::Corela;

impl Corela {
    pub(super) fn start_playback(&mut self) {
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

    pub(super) fn play_due_action(&mut self) {
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

    pub(super) fn stop_playback(&mut self) {
        self.log_action("playback_stopped");
        self.playback_timer = None;
        self.replay = Replay::new(self.config.last_action);
        self.history = History::new(Board::new(self.geometry.hex_radius));
    }
}
