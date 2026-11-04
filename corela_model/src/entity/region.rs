#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Region {
    Center,
    North,
}

impl Region {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Center => "Center",
            Self::North => "North",
        }
    }

    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Center => "center",
            Self::North => "north",
        }
    }

    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        if code == Self::Center.code() {
            Some(Self::Center)
        } else if code == Self::North.code() {
            Some(Self::North)
        } else {
            None
        }
    }
}
