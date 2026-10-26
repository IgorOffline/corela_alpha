#![no_std]

extern crate alloc;

use alloc::boxed::Box;
use core::ffi::{CStr, c_char};
use core::ptr;
use core::sync::atomic::{AtomicPtr, Ordering};

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
    fn gfx_run(desc: *const Desc) -> i32;
    fn gfx_quit();
    fn gfx_renderer_init(clear_r: f32, clear_g: f32, clear_b: f32);
    fn gfx_frame_begin();
    fn gfx_draw_label(index: i32, x: f32, y: f32, background: u32, selected: bool);
    fn gfx_draw_circle(x: f32, y: f32, radius: f32, color: u32);
    fn gfx_draw_double_chevron_right(x: f32, y: f32, dx: f32, dy: f32, color: u32);
    fn gfx_draw_hexagon(x: f32, y: f32, radius: f32, color: u32);
    fn gfx_frame_end();
}

pub type Point = (f32, f32);

pub struct Window<'a> {
    pub title: &'a CStr,
    pub width: i32,
    pub height: i32,
    pub clear: u32,
}

pub trait App: 'static {
    fn frame(&mut self, frame: &mut Frame);
    fn key_down(&mut self, key_code: i32);
    fn cleanup(&mut self) {}
}

pub struct Frame(());

impl Frame {
    pub fn hexagon(&mut self, (x, y): Point, radius: f32, color: u32) {
        if !x.is_finite() || !y.is_finite() || !radius.is_finite() || radius <= 0.0 {
            return;
        }
        unsafe { gfx_draw_hexagon(x, y, radius, color) }
    }

    pub fn label(&mut self, index: i32, (x, y): Point, background: u32, selected: bool) {
        unsafe { gfx_draw_label(index, x, y, background, selected) }
    }

    pub fn circle(&mut self, (x, y): Point, radius: f32, color: u32) {
        unsafe { gfx_draw_circle(x, y, radius, color) }
    }

    pub fn double_chevron_right(&mut self, (x, y): Point, (dx, dy): Point, color: u32) {
        unsafe { gfx_draw_double_chevron_right(x, y, dx, dy, color) }
    }
}

#[derive(Clone, Copy)]
pub struct HexLayout {
    pub origin: Point,
    pub size: f32,
}

impl HexLayout {
    pub fn to_pixel(&self, q: i32, r: i32) -> Point {
        (
            self.origin.0 + self.size * 1.5 * q as f32,
            self.origin.1 + self.size * 1.732_050_8 * (r as f32 + q as f32 / 2.0),
        )
    }
}

const fn channel(hex: u32, shift: u32) -> f32 {
    ((hex >> shift) & 0xff) as f32 / 255.0
}

struct Running<A> {
    clear: u32,
    app: A,
}

static RUNNING: AtomicPtr<()> = AtomicPtr::new(ptr::null_mut());

fn running<A>() -> &'static mut Running<A> {
    unsafe { &mut *RUNNING.load(Ordering::Acquire).cast() }
}

extern "C" fn init<A: App>() {
    let clear = running::<A>().clear;
    unsafe { gfx_renderer_init(channel(clear, 16), channel(clear, 8), channel(clear, 0)) }
}

extern "C" fn frame<A: App>() {
    unsafe { gfx_frame_begin() }
    running::<A>().app.frame(&mut Frame(()));
    unsafe { gfx_frame_end() }
}

extern "C" fn cleanup<A: App>() {
    running::<A>().app.cleanup();
}

extern "C" fn key_down<A: App>(key_code: i32) {
    running::<A>().app.key_down(key_code);
}

pub fn run<A: App>(window: &Window, app: A) {
    let state = Box::into_raw(Box::new(Running {
        clear: window.clear,
        app,
    }));
    assert!(
        RUNNING.swap(state.cast(), Ordering::AcqRel).is_null(),
        "corela_gfx::run is already running"
    );
    let desc = Desc {
        title: window.title.as_ptr(),
        width: window.width,
        height: window.height,
        init: init::<A>,
        frame: frame::<A>,
        cleanup: cleanup::<A>,
        key_down: key_down::<A>,
    };
    unsafe {
        gfx_run(&desc);
        drop(Box::from_raw(
            RUNNING
                .swap(ptr::null_mut(), Ordering::AcqRel)
                .cast::<Running<A>>(),
        ));
    }
}

pub fn quit() {
    unsafe { gfx_quit() }
}
