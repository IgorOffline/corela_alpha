use corela_model::dto::Snapshot;
use corela_model::entity::Board;
use corela_model::message::Command;

pub struct Action {
    number: i32,
    command: Command,
    note: String,
    snapshot: Snapshot,
}

impl Action {
    pub(crate) const fn new(
        number: i32,
        command: Command,
        note: String,
        snapshot: Snapshot,
    ) -> Self {
        Self {
            number,
            command,
            note,
            snapshot,
        }
    }

    #[must_use]
    pub(crate) const fn number(&self) -> i32 {
        self.number
    }

    #[must_use]
    pub(crate) const fn command(&self) -> Command {
        self.command
    }

    #[must_use]
    pub fn note(&self) -> &str {
        &self.note
    }

    #[must_use]
    pub(crate) const fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }

    #[must_use]
    pub const fn board(&self) -> &Board {
        &self.snapshot.board
    }
}
