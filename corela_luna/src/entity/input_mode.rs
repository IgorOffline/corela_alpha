#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InputMode {
    None,
    Input,
    Undo,
    Redo,
    Save,
    Load,
}

impl InputMode {
    pub(crate) const fn next(self) -> Self {
        match self {
            Self::None => Self::Input,
            Self::Input => Self::Undo,
            Self::Undo => Self::Redo,
            Self::Redo => Self::Save,
            Self::Save => Self::Load,
            Self::Load => Self::None,
        }
    }

    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Input => "Input",
            Self::Undo => "Undo",
            Self::Redo => "Redo",
            Self::Save => "Save",
            Self::Load => "Load",
        }
    }

    pub(crate) const fn code(self) -> &'static str {
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
