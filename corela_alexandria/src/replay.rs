use corela_ema::dto::Snapshot;
use corela_ema::message::Command;

use crate::action::Action;

const FIRST_ACTION: i32 = 1;

pub struct Replay {
    last_action: i32,
    current_action: i32,
    actions: Vec<Action>,
}

impl Replay {
    #[must_use]
    pub const fn new(last_action: i32) -> Self {
        Self {
            last_action,
            current_action: FIRST_ACTION,
            actions: Vec::new(),
        }
    }

    pub(crate) const fn from_actions(last_action: i32, actions: Vec<Action>) -> Self {
        Self {
            last_action,
            current_action: FIRST_ACTION,
            actions,
        }
    }

    #[must_use]
    pub fn current_action(&self) -> i32 {
        self.current_action.min(self.last_action)
    }

    #[must_use]
    pub const fn last_action(&self) -> i32 {
        self.last_action
    }

    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.current_action > self.last_action
    }

    pub(crate) fn actions(&self) -> &[Action] {
        &self.actions
    }

    #[must_use]
    pub const fn has_actions(&self) -> bool {
        !self.actions.is_empty()
    }

    pub fn record(&mut self, command: Command, note: String, snapshot: Snapshot) {
        self.actions
            .push(Action::new(self.current_action, command, note, snapshot));
        self.current_action += 1;
    }

    #[must_use]
    pub fn action_to_play(&self) -> Option<&Action> {
        match usize::try_from(self.current_action - FIRST_ACTION) {
            Ok(index) => self.actions.get(index),
            Err(_) => None,
        }
    }

    pub const fn finish_playing_action(&mut self) {
        self.current_action += 1;
    }
}
