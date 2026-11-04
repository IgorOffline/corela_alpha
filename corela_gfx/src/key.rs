use macroquad::input::KeyCode;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key(KeyCode);

const KEY_NAME_TABLE: [(&str, KeyCode); 121] = [
    ("Space", KeyCode::Space),
    ("Apostrophe", KeyCode::Apostrophe),
    ("Comma", KeyCode::Comma),
    ("Minus", KeyCode::Minus),
    ("Period", KeyCode::Period),
    ("Slash", KeyCode::Slash),
    ("Key0", KeyCode::Key0),
    ("Key1", KeyCode::Key1),
    ("Key2", KeyCode::Key2),
    ("Key3", KeyCode::Key3),
    ("Key4", KeyCode::Key4),
    ("Key5", KeyCode::Key5),
    ("Key6", KeyCode::Key6),
    ("Key7", KeyCode::Key7),
    ("Key8", KeyCode::Key8),
    ("Key9", KeyCode::Key9),
    ("Semicolon", KeyCode::Semicolon),
    ("Equal", KeyCode::Equal),
    ("A", KeyCode::A),
    ("B", KeyCode::B),
    ("C", KeyCode::C),
    ("D", KeyCode::D),
    ("E", KeyCode::E),
    ("F", KeyCode::F),
    ("G", KeyCode::G),
    ("H", KeyCode::H),
    ("I", KeyCode::I),
    ("J", KeyCode::J),
    ("K", KeyCode::K),
    ("L", KeyCode::L),
    ("M", KeyCode::M),
    ("N", KeyCode::N),
    ("O", KeyCode::O),
    ("P", KeyCode::P),
    ("Q", KeyCode::Q),
    ("R", KeyCode::R),
    ("S", KeyCode::S),
    ("T", KeyCode::T),
    ("U", KeyCode::U),
    ("V", KeyCode::V),
    ("W", KeyCode::W),
    ("X", KeyCode::X),
    ("Y", KeyCode::Y),
    ("Z", KeyCode::Z),
    ("LeftBracket", KeyCode::LeftBracket),
    ("Backslash", KeyCode::Backslash),
    ("RightBracket", KeyCode::RightBracket),
    ("GraveAccent", KeyCode::GraveAccent),
    ("World1", KeyCode::World1),
    ("World2", KeyCode::World2),
    ("Escape", KeyCode::Escape),
    ("Enter", KeyCode::Enter),
    ("Tab", KeyCode::Tab),
    ("Backspace", KeyCode::Backspace),
    ("Insert", KeyCode::Insert),
    ("Delete", KeyCode::Delete),
    ("Right", KeyCode::Right),
    ("Left", KeyCode::Left),
    ("Down", KeyCode::Down),
    ("Up", KeyCode::Up),
    ("PageUp", KeyCode::PageUp),
    ("PageDown", KeyCode::PageDown),
    ("Home", KeyCode::Home),
    ("End", KeyCode::End),
    ("CapsLock", KeyCode::CapsLock),
    ("ScrollLock", KeyCode::ScrollLock),
    ("NumLock", KeyCode::NumLock),
    ("PrintScreen", KeyCode::PrintScreen),
    ("Pause", KeyCode::Pause),
    ("F1", KeyCode::F1),
    ("F2", KeyCode::F2),
    ("F3", KeyCode::F3),
    ("F4", KeyCode::F4),
    ("F5", KeyCode::F5),
    ("F6", KeyCode::F6),
    ("F7", KeyCode::F7),
    ("F8", KeyCode::F8),
    ("F9", KeyCode::F9),
    ("F10", KeyCode::F10),
    ("F11", KeyCode::F11),
    ("F12", KeyCode::F12),
    ("F13", KeyCode::F13),
    ("F14", KeyCode::F14),
    ("F15", KeyCode::F15),
    ("F16", KeyCode::F16),
    ("F17", KeyCode::F17),
    ("F18", KeyCode::F18),
    ("F19", KeyCode::F19),
    ("F20", KeyCode::F20),
    ("F21", KeyCode::F21),
    ("F22", KeyCode::F22),
    ("F23", KeyCode::F23),
    ("F24", KeyCode::F24),
    ("F25", KeyCode::F25),
    ("Kp0", KeyCode::Kp0),
    ("Kp1", KeyCode::Kp1),
    ("Kp2", KeyCode::Kp2),
    ("Kp3", KeyCode::Kp3),
    ("Kp4", KeyCode::Kp4),
    ("Kp5", KeyCode::Kp5),
    ("Kp6", KeyCode::Kp6),
    ("Kp7", KeyCode::Kp7),
    ("Kp8", KeyCode::Kp8),
    ("Kp9", KeyCode::Kp9),
    ("KpDecimal", KeyCode::KpDecimal),
    ("KpDivide", KeyCode::KpDivide),
    ("KpMultiply", KeyCode::KpMultiply),
    ("KpSubtract", KeyCode::KpSubtract),
    ("KpAdd", KeyCode::KpAdd),
    ("KpEnter", KeyCode::KpEnter),
    ("KpEqual", KeyCode::KpEqual),
    ("LeftShift", KeyCode::LeftShift),
    ("LeftControl", KeyCode::LeftControl),
    ("LeftAlt", KeyCode::LeftAlt),
    ("LeftSuper", KeyCode::LeftSuper),
    ("RightShift", KeyCode::RightShift),
    ("RightControl", KeyCode::RightControl),
    ("RightAlt", KeyCode::RightAlt),
    ("RightSuper", KeyCode::RightSuper),
    ("Menu", KeyCode::Menu),
    ("Back", KeyCode::Back),
];

impl Key {
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        for (candidate, code) in KEY_NAME_TABLE {
            if candidate == name {
                return Some(Self(code));
            }
        }
        None
    }

    pub(crate) fn from_code(code: KeyCode) -> Option<Self> {
        for (_, candidate) in KEY_NAME_TABLE {
            if candidate == code {
                return Some(Self(code));
            }
        }
        None
    }

    #[must_use]
    pub fn name(self) -> &'static str {
        for (name, code) in KEY_NAME_TABLE {
            if code == self.0 {
                return name;
            }
        }
        "Unknown"
    }
}
