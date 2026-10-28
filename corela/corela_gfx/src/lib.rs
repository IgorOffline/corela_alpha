#![no_std]

extern crate alloc;
extern crate corela_gfx_impl;

mod icon;

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

pub fn geometry() -> Geometry {
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
pub struct Frame(());

impl Frame {
    pub fn hexagon(&mut self, center: Point, radius: f32, color: u32) {
        let (x, y) = center;
        if !x.is_finite() || !y.is_finite() || !radius.is_finite() || radius <= 0.0 {
            return;
        }
        unsafe { gfx_draw_hexagon(x, y, radius, color) }
    }

    pub fn clear(&mut self, color: u32) {
        unsafe {
            gfx_set_clear(
                channel(color, RED_SHIFT),
                channel(color, GREEN_SHIFT),
                channel(color, BLUE_SHIFT),
            )
        }
    }

    pub fn label(&mut self, index: i32, center: Point, background: u32, text: u32, selected: bool) {
        let (x, y) = center;
        unsafe { gfx_draw_label(index, x, y, background, text, selected) }
    }

    pub fn circle(&mut self, center: Point, radius: f32, color: u32) {
        let (x, y) = center;
        unsafe { gfx_draw_circle(x, y, radius, color) }
    }

    pub fn icon(&mut self, icon: &Icon, at: Point, direction: Point, size: f32, color: u32) {
        let (x, y) = at;
        let (dx, dy) = direction;
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
    pub fn to_pixel(&self, q: i32, r: i32) -> Point {
        (
            self.origin.0 + self.size * HEX_COLUMN_SPACING_TO_SIZE_RATIO * q as f32,
            self.origin.1 + self.size * HEX_ROW_SPACING_TO_SIZE_RATIO * (r as f32 + q as f32 / 2.0),
        )
    }
}

// Colors are packed as 0xRRGGBB.
const RED_SHIFT: u32 = 16;
const GREEN_SHIFT: u32 = 8;
const BLUE_SHIFT: u32 = 0;

/// One 8-bit channel of `hex`, scaled to `0.0..=1.0`.
const fn channel(hex: u32, shift: u32) -> f32 {
    (hex >> shift) as u8 as f32 / u8::MAX as f32
}

/// Everything the C callbacks need while `run` is active.
struct Running {
    clear: u32,
    app: Box<dyn App>,
}

/// The C callbacks take no arguments
static RUNNING: AtomicPtr<Running> = AtomicPtr::new(ptr::null_mut());

fn running() -> &'static mut Running {
    unsafe { &mut *RUNNING.load(Ordering::Acquire) }
}

extern "C" fn on_init() {
    let clear = running().clear;
    unsafe {
        gfx_renderer_init(
            channel(clear, RED_SHIFT),
            channel(clear, GREEN_SHIFT),
            channel(clear, BLUE_SHIFT),
        )
    }
}

extern "C" fn on_frame() {
    unsafe { gfx_frame_begin() }
    running().app.frame(&mut Frame(()));
    unsafe { gfx_frame_end() }
}

extern "C" fn on_cleanup() {
    running().app.cleanup();
}

extern "C" fn on_key_down(key_code: i32) {
    running().app.key_down(key_code);
}

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
    unsafe {
        gfx_run(&desc);
        let finished = RUNNING.swap(ptr::null_mut(), Ordering::AcqRel);
        drop(Box::from_raw(finished));
    }
}

pub fn quit() {
    unsafe { gfx_quit() }
}
