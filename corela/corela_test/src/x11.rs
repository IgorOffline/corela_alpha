//! Driving and observing corela's window on an X11 desktop.
//!
//! Requirements: an X11 session (`DISPLAY`), `xdotool` and `xwd`.
//! `XDOTOOL` may point at an xdotool binary that is not on `PATH`. If its
//! `libxdo.so.4` sits next to it (an uninstalled build), the suite finds it.
//!
//! Safety rule: every key press first checks that corela's window has the
//! keyboard focus, and the test stops if it does not. Key presses go to the
//! focused window, so without this check a test could type into some other
//! application on the desktop.

use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use crate::product;

const POLL: Duration = Duration::from_millis(100);

static DESKTOP: Mutex<()> = Mutex::new(());

/// Exclusive use of the desktop: GUI tests in one test binary run one at a
/// time, because they compete for keyboard focus.
pub fn desktop() -> MutexGuard<'static, ()> {
    match DESKTOP.lock() {
        Ok(guard) => guard,
        // an earlier GUI test failed while holding the lock; the desktop
        // itself is still usable
        Err(poisoned) => poisoned.into_inner(),
    }
}

pub struct Xdotool {
    binary: PathBuf,
    library_dir: Option<PathBuf>,
}

impl Xdotool {
    #[must_use]
    pub fn locate() -> Self {
        let binary = if let Some(path) = std::env::var_os("XDOTOOL") {
            PathBuf::from(path)
        } else {
            let fallback = PathBuf::from("/home/igor/dev/xdotool-4.20260303.1/xdotool.static");
            if fallback.is_file() {
                fallback
            } else {
                PathBuf::from("xdotool")
            }
        };
        let mut library_dir = None;
        if let Some(dir) = binary.parent()
            && dir.join("libxdo.so.4").is_file()
        {
            library_dir = Some(dir.to_path_buf());
        }
        Self {
            binary,
            library_dir,
        }
    }

    fn run(&self, args: &[&str]) -> Output {
        let mut command = Command::new(&self.binary);
        command.args(args);
        if let Some(dir) = &self.library_dir {
            command.env("LD_LIBRARY_PATH", dir);
        }
        match command.output() {
            Ok(output) => output,
            Err(error) => panic!(
                "cannot run xdotool ({}): {error}; install it or set XDOTOOL",
                self.binary.display()
            ),
        }
    }

    fn stdout(&self, args: &[&str]) -> String {
        String::from_utf8_lossy(&self.run(args).stdout)
            .trim()
            .to_owned()
    }
}

/// corela's top-level window.
pub struct Window {
    xdotool: Xdotool,
    id: String,
}

impl Window {
    /// Waits for exactly one window titled `corela_bpmn`, then gives it the
    /// keyboard focus.
    ///
    /// # Panics
    /// If no such window appears within `limit`, or more than one exists
    /// (another corela is running).
    #[must_use]
    pub fn find(limit: Duration) -> Self {
        let xdotool = Xdotool::locate();
        let pattern = format!("^{}$", product::WINDOW_TITLE);
        let deadline = Instant::now() + limit;
        loop {
            let found = xdotool.stdout(&["search", "--name", &pattern]);
            let mut ids: Vec<&str> = Vec::new();
            for id in found.split_whitespace() {
                ids.push(id);
            }
            match ids.as_slice() {
                [id] => {
                    let window = Self {
                        id: (*id).to_owned(),
                        xdotool,
                    };
                    window.focus();
                    return window;
                }
                [] => {}
                _ => panic!("more than one corela window is open; close the others"),
            }
            assert!(
                Instant::now() < deadline,
                "no corela window appeared within {limit:?}"
            );
            std::thread::sleep(POLL);
        }
    }

    fn focus(&self) {
        let _activated = self.xdotool.run(&["windowactivate", "--sync", &self.id]);
        let _focused = self.xdotool.run(&["windowfocus", "--sync", &self.id]);
    }

    fn has_focus(&self) -> bool {
        self.xdotool.stdout(&["getwindowfocus"]) == self.id
    }

    /// Presses and releases one key in corela's window.
    ///
    /// # Panics
    /// If corela does not have the keyboard focus. Nothing is sent then.
    pub fn press(&self, key: &str) {
        assert!(
            self.has_focus(),
            "corela lost keyboard focus; stopped before pressing {key:?} so \
             that no other application receives it"
        );
        let _pressed = self.xdotool.run(&["key", "--clearmodifiers", key]);
    }

    /// The window's contents once two consecutive captures agree, i.e. after
    /// the frame has settled.
    ///
    /// # Panics
    /// If no stable capture is possible within `limit`.
    #[must_use]
    pub fn capture(&self, limit: Duration) -> Frame {
        let deadline = Instant::now() + limit;
        let mut previous: Option<Frame> = None;
        while Instant::now() < deadline {
            if let Some(frame) = self.capture_once() {
                if previous.as_ref() == Some(&frame) {
                    return frame;
                }
                previous = Some(frame);
            }
            std::thread::sleep(POLL);
        }
        panic!("no stable capture of the corela window within {limit:?}");
    }

    fn capture_once(&self) -> Option<Frame> {
        let output = Command::new("xwd")
            .args(["-id", &self.id, "-silent"])
            .output();
        match output {
            Ok(output) if output.status.success() => Frame::from_xwd(&output.stdout),
            Ok(_) => None,
            Err(error) => panic!("cannot run xwd: {error}; install x11-apps"),
        }
    }
}

/// An RGB image of the window.
#[derive(PartialEq, Eq)]
pub struct Frame {
    pub width: i32,
    pub height: i32,
    rgb: Vec<u8>,
}

/// Fields of the XWD file header (all big-endian 32-bit words).
const XWD_HEADER_WORDS: usize = 25;
const XWD_HEADER_SIZE: usize = 0;
const XWD_WIDTH: usize = 4;
const XWD_HEIGHT: usize = 5;
const XWD_BYTE_ORDER: usize = 7;
const XWD_BITS_PER_PIXEL: usize = 11;
const XWD_BYTES_PER_LINE: usize = 12;
const XWD_COLORMAP_ENTRIES: usize = 19;
const XWD_COLORMAP_ENTRY_SIZE: usize = 12;
const LSB_FIRST: u32 = 0;

impl Frame {
    /// Decodes an `xwd` dump of a 24- or 32-bit `TrueColor` window. X servers
    /// differ (this suite has seen both on one machine), so both are read.
    #[must_use]
    pub fn from_xwd(data: &[u8]) -> Option<Self> {
        let mut header = [0_u32; XWD_HEADER_WORDS];
        for (index, word) in header.iter_mut().enumerate() {
            let bytes = data.get(index * 4..index * 4 + 4)?;
            *word = u32::from_be_bytes([
                *bytes.first()?,
                *bytes.get(1)?,
                *bytes.get(2)?,
                *bytes.get(3)?,
            ]);
        }
        let (width, height) = (
            header_word(&header, XWD_WIDTH)?,
            header_word(&header, XWD_HEIGHT)?,
        );
        let bytes_per_pixel = match header[XWD_BITS_PER_PIXEL] {
            32 => 4,
            24 => 3,
            _ => return None,
        };
        let stride = header_word(&header, XWD_BYTES_PER_LINE)?;
        let start = header_word(&header, XWD_HEADER_SIZE)?
            + header_word(&header, XWD_COLORMAP_ENTRIES)? * XWD_COLORMAP_ENTRY_SIZE;
        let lsb_first = header[XWD_BYTE_ORDER] == LSB_FIRST;

        let mut rgb = Vec::with_capacity(width * height * 3);
        for row in 0..height {
            for column in 0..width {
                let at = start + row * stride + column * bytes_per_pixel;
                let pixel = data.get(at..at + bytes_per_pixel)?;
                // LSB-first stores B, G, R(, X); MSB-first stores (X,) R, G, B
                let (red, green, blue) = if lsb_first {
                    (pixel.get(2)?, pixel.get(1)?, pixel.first()?)
                } else {
                    let skip = bytes_per_pixel - 3;
                    (pixel.get(skip)?, pixel.get(skip + 1)?, pixel.get(skip + 2)?)
                };
                rgb.extend_from_slice(&[*red, *green, *blue]);
            }
        }
        Some(Self {
            width: i32::try_from(width).ok()?,
            height: i32::try_from(height).ok()?,
            rgb,
        })
    }

    /// The color at `(x, y)`, or `None` outside the frame.
    #[must_use]
    pub fn pixel(&self, x: i32, y: i32) -> Option<[u8; 3]> {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return None;
        }
        let index = usize::try_from((y * self.width + x) * 3).ok()?;
        let bytes = self.rgb.get(index..index + 3)?;
        Some([*bytes.first()?, *bytes.get(1)?, *bytes.get(2)?])
    }

    /// The pixel center of board cell `(q, r)`.
    #[must_use]
    pub fn cell_center(&self, q: i32, r: i32) -> (i32, i32) {
        let x = f64::from(self.width) / 2.0 + product::CELL_PITCH_X * f64::from(q);
        let y = f64::from(self.height) / 2.0
            + product::CELL_PITCH_Y * (f64::from(r) + f64::from(q) / 2.0);
        (round(x), round(y))
    }

    /// How many pixels differ from `other` inside the board area.
    #[must_use]
    pub fn board_difference(&self, other: &Self) -> usize {
        let (center_x, center_y) = (self.width / 2, self.height / 2);
        let mut differing = 0;
        for y in center_y - product::BOARD_HALF_HEIGHT..center_y + product::BOARD_HALF_HEIGHT {
            for x in center_x - product::BOARD_HALF_WIDTH..center_x + product::BOARD_HALF_WIDTH {
                if self.pixel(x, y) != other.pixel(x, y) {
                    differing += 1;
                }
            }
        }
        differing
    }
}

/// XWD header word `field` as a `usize`.
fn header_word(header: &[u32; XWD_HEADER_WORDS], field: usize) -> Option<usize> {
    let word = header.get(field)?;
    usize::try_from(*word).ok()
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "pixel coordinates are a few thousand at most"
)]
fn round(value: f64) -> i32 {
    value.round() as i32
}
