#![no_std]

extern crate alloc;

mod icon;

// links the native sokol/nanovg/GLib library; no Rust items are used
use corela_gfx_impl as _;

pub use icon::Icon;

use alloc::boxed::Box;
use core::ffi::{CStr, c_char};
use core::ptr;
use core::sync::atomic::{AtomicPtr, Ordering};

use icon::IconDesc;

/// Mirrors `gfx_desc` in sokol.c; keep the field order in sync.
#[repr(C)]
struct Desc {
    title: *const c_char,
    width: i32,
    height: i32,
    init: extern "C" fn(),
    frame: extern "C" fn(),
    cleanup: extern "C" fn(),
    key_down: extern "C" fn(key_code: i32),
}

unsafe extern "C" {
    fn gfx_get_geometry() -> Geometry;
    fn gfx_run(desc: *const Desc) -> i32;
    fn gfx_quit();
    fn gfx_renderer_init(clear_r: f32, clear_g: f32, clear_b: f32);
    fn gfx_frame_begin();
    fn gfx_set_clear(clear_r: f32, clear_g: f32, clear_b: f32);
    fn gfx_draw_label(index: i32, x: f32, y: f32, background: u32, text: u32, selected: bool);
    fn gfx_set_font(data: *const u8, length: usize);
    fn gfx_draw_footer(text: *const c_char, color: u32);
    fn gfx_draw_circle(x: f32, y: f32, radius: f32, color: u32);
    fn gfx_draw_icon(
        icon: *const IconDesc,
        x: f32,
        y: f32,
        dx: f32,
        dy: f32,
        size: f32,
        color: u32,
    );
    fn gfx_draw_hexagon(x: f32, y: f32, radius: f32, color: u32);
    fn gfx_frame_end();
}

/// Fixed application geometry defined by the C renderer's geometry.h.
/// Mirrors `gfx_geometry`; keep the field order in sync.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Geometry {
    pub cell_marker_outer_radius: f32,
    pub cell_marker_inner_radius: f32,
    pub chevron_size: f32,
    pub hex_radius: i32,
    pub hex_size: f32,
    pub hex_tile_inset: f32,
}

#[must_use]
pub fn geometry() -> Geometry {
    // SAFETY: gfx_get_geometry only returns compile-time constants.
    unsafe { gfx_get_geometry() }
}

pub type Point = (f32, f32);

pub struct Window {
    pub title: &'static CStr,
    pub width: i32,
    pub height: i32,
    pub clear: u32,
}

pub trait App {
    fn frame(&mut self, frame: &mut Frame);
    fn key_down(&mut self, key_code: i32);
    fn cleanup(&mut self) {}
}

/// Drawing commands for the current frame. The private `()` field means only
/// this crate can create one, so drawing is only possible inside `App::frame`.
///
/// This is also the safety argument for every draw call below: `on_frame`
/// creates the only `Frame`s, between `gfx_frame_begin` and `gfx_frame_end`,
/// so holding one proves the renderer is initialized and a frame is open.
pub struct Frame(());

#[expect(
    clippy::unused_self,
    clippy::needless_pass_by_ref_mut,
    reason = "Frame is a capability token: `&mut self` proves a frame is open"
)]
impl Frame {
    pub fn hexagon(&mut self, center: Point, radius: f32, color: u32) {
        let (x, y) = center;
        if !x.is_finite() || !y.is_finite() || !radius.is_finite() || radius <= 0.0 {
            return;
        }
        // SAFETY: a frame is open (see Frame).
        unsafe { gfx_draw_hexagon(x, y, radius, color) }
    }

    pub fn clear(&mut self, color: u32) {
        let (red, green, blue) = channels(color);
        // SAFETY: a frame is open (see Frame).
        unsafe { gfx_set_clear(red, green, blue) }
    }

    pub fn label(&mut self, index: i32, center: Point, background: u32, text: u32, selected: bool) {
        let (x, y) = center;
        // SAFETY: a frame is open (see Frame); draw.c ignores unknown indices.
        unsafe { gfx_draw_label(index, x, y, background, text, selected) }
    }

    pub fn footer(&mut self, text: &CStr, color: u32) {
        // SAFETY: a frame is open (see Frame); `text` is NUL-terminated and
        // nanovg copies it before the call returns.
        unsafe { gfx_draw_footer(text.as_ptr(), color) }
    }

    pub fn circle(&mut self, center: Point, radius: f32, color: u32) {
        let (x, y) = center;
        // SAFETY: a frame is open (see Frame).
        unsafe { gfx_draw_circle(x, y, radius, color) }
    }

    pub fn icon(&mut self, icon: &Icon, at: Point, direction: Point, size: f32, color: u32) {
        let (x, y) = at;
        let (dx, dy) = direction;
        // SAFETY: a frame is open (see Frame). The descriptor's pointers point
        // into `icon`, which outlives the call, and draw.c does not keep them.
        unsafe { gfx_draw_icon(&icon.desc(), x, y, dx, dy, size, color) }
    }
}

/// Horizontal distance between neighbouring hex column centers, per unit of hex size.
const HEX_COLUMN_SPACING_TO_SIZE_RATIO: f32 = 1.5;
/// Vertical distance between neighbouring hex row centers (sqrt(3)), per unit of hex size.
const HEX_ROW_SPACING_TO_SIZE_RATIO: f32 = 1.732_050_8;

/// Maps axial `(q, r)` cells of a flat-top hex grid to pixel centers.
#[derive(Clone, Copy)]
pub struct HexLayout {
    pub origin: Point,
    pub size: f32,
}

impl HexLayout {
    #[must_use]
    pub fn to_pixel(self, q: i32, r: i32) -> Point {
        let column = small_to_f32(q);
        let row = small_to_f32(r);
        (
            self.origin.0 + self.size * HEX_COLUMN_SPACING_TO_SIZE_RATIO * column,
            self.origin.1 + self.size * HEX_ROW_SPACING_TO_SIZE_RATIO * (row + column / 2.0),
        )
    }
}

/// `value` as an `f32`, exactly, if it fits in an `i16`.
///
/// That covers hex coordinates (within `hex_radius`) and window sides (at
/// most 32767, as the config checks). Anything larger becomes NaN, which
/// `Frame::hexagon` refuses to draw, instead of a silently rounded position.
#[must_use]
pub fn small_to_f32(value: i32) -> f32 {
    match i16::try_from(value) {
        Ok(small) => f32::from(small),
        Err(_) => f32::NAN,
    }
}

/// The red, green and blue channels of a `0xRRGGBB` color, each scaled to
/// `0.0..=1.0`. The top byte is ignored.
fn channels(hex: u32) -> (f32, f32, f32) {
    let [_, red, green, blue] = hex.to_be_bytes();
    let max = f32::from(u8::MAX);
    (
        f32::from(red) / max,
        f32::from(green) / max,
        f32::from(blue) / max,
    )
}

/// Everything the C callbacks need while `run` is active.
struct Running {
    clear: u32,
    app: Box<dyn App>,
}

/// The C callbacks take no arguments
static RUNNING: AtomicPtr<Running> = AtomicPtr::new(ptr::null_mut());

/// The state of the active `run`.
///
/// # Safety
/// Call only from a sokol callback (`on_*`): sokol calls those only inside
/// `gfx_run`, after `run` has set `RUNNING` and before it clears it, and
/// never nests them. Drop the reference before the callback returns, so no
/// two are ever alive at once.
unsafe fn running() -> &'static mut Running {
    let state = RUNNING.load(Ordering::Acquire);
    debug_assert!(!state.is_null(), "sokol callback outside corela_gfx::run");
    // SAFETY: the caller upholds the contract above, so `state` is the live,
    // exclusively owned Box from `run`.
    unsafe { &mut *state }
}

static FONT: &[u8] = include_bytes!("../../corela_gfx_impl/asset/font/IntelOneMono-Medium.ttf");

extern "C" fn on_init() {
    // SAFETY: a sokol callback (see running).
    let clear = unsafe { running() }.clear;
    let (red, green, blue) = channels(clear);
    // SAFETY: sokol calls init once, after the GL context exists.
    unsafe { gfx_renderer_init(red, green, blue) }
    // SAFETY: the renderer was just initialized. nanovg keeps the pointer
    // without copying the bytes, which is sound because FONT is 'static;
    // draw.c checks that the length fits nanovg's int.
    unsafe { gfx_set_font(FONT.as_ptr(), FONT.len()) }
}

extern "C" fn on_frame() {
    // SAFETY: sokol calls frame only after init.
    unsafe { gfx_frame_begin() }
    // SAFETY: a sokol callback (see running).
    unsafe { running() }.app.frame(&mut Frame(()));
    // SAFETY: closes the frame opened above; the Frame is gone.
    unsafe { gfx_frame_end() }
}

extern "C" fn on_cleanup() {
    // SAFETY: a sokol callback (see running).
    unsafe { running() }.app.cleanup();
}

extern "C" fn on_key_down(key_code: i32) {
    // SAFETY: a sokol callback (see running).
    unsafe { running() }.app.key_down(key_code);
}

/// Opens the window and runs `app` until the window closes.
///
/// # Panics
/// If another `run` is already active; there is only one window.
pub fn run(window: &Window, app: Box<dyn App>) {
    let state = Box::into_raw(Box::new(Running {
        clear: window.clear,
        app,
    }));
    let previous = RUNNING.swap(state, Ordering::AcqRel);
    assert!(previous.is_null(), "corela_gfx::run is already running");
    let desc = Desc {
        title: window.title.as_ptr(),
        width: window.width,
        height: window.height,
        init: on_init,
        frame: on_frame,
        cleanup: on_cleanup,
        key_down: on_key_down,
    };
    // SAFETY: `desc` and the title it points to outlive the call, and gfx_run
    // returns only after sokol has called on_cleanup.
    unsafe { gfx_run(ptr::from_ref(&desc)) };
    let finished = RUNNING.swap(ptr::null_mut(), Ordering::AcqRel);
    // SAFETY: `finished` is the Box::into_raw pointer stored above; RUNNING
    // held its only copy, and no callback can run any more.
    drop(unsafe { Box::from_raw(finished) });
}

pub fn quit() {
    // SAFETY: sapp_request_quit only sets a flag, which is harmless even
    // when no app is running.
    unsafe { gfx_quit() }
}
