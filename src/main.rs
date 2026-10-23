#![no_std]
#![no_main]

use core::ffi::{c_char, c_int};
use core::sync::atomic::{AtomicI32, Ordering};

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
    fn ls_run(desc: *const Desc) -> i32;
    fn ls_quit();
    fn ls_renderer_init(clear_r: f32, clear_g: f32, clear_b: f32);
    fn ls_frame_begin();
    fn ls_draw_label(index: i32, x: f32, y: f32, background: u32, selected: bool);
    fn ls_draw_circle(x: f32, y: f32, radius: f32, color: u32);
    fn ls_draw_hexagon(x: f32, y: f32, radius: f32, color: u32);
    fn ls_frame_end();
    fn abort() -> !;
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    unsafe { abort() }
}

#[unsafe(no_mangle)]
extern "C" fn rust_eh_personality() -> ! {
    unsafe { abort() }
}

const KEY_ESCAPE: i32 = 256;

const WINDOW_WIDTH: i32 = 1152;
const WINDOW_HEIGHT: i32 = 648;

const BASE: u32 = 0x626880;
const RADIUS: i32 = 4;
const SIZE: f32 = 32.0;
const TILE: u32 = 0x8caaee;
const SELECTED: u32 = 0xca9ee6;
static SELECTED_Q: AtomicI32 = AtomicI32::new(0);
static SELECTED_R: AtomicI32 = AtomicI32::new(0);

fn in_bounds(q: i32, r: i32) -> bool {
    q.abs().max(r.abs()).max((q + r).abs()) <= RADIUS
}

fn to_pixel(q: i32, r: i32) -> (f32, f32) {
    (
        WINDOW_WIDTH as f32 / 2.0 + SIZE * 1.5 * q as f32,
        WINDOW_HEIGHT as f32 / 2.0 + SIZE * 1.732_050_8 * (r as f32 + q as f32 / 2.0),
    )
}

const fn channel(hex: u32, shift: u32) -> f32 {
    ((hex >> shift) & 0xff) as f32 / 255.0
}

const fn rgb(hex: u32) -> [f32; 3] {
    [channel(hex, 16), channel(hex, 8), channel(hex, 0)]
}

extern "C" fn init() {
    let [r, g, b] = rgb(BASE);
    unsafe { ls_renderer_init(r, g, b) }
}

extern "C" fn frame() {
    unsafe { ls_frame_begin() }
    let selected = (
        SELECTED_Q.load(Ordering::Relaxed),
        SELECTED_R.load(Ordering::Relaxed),
    );
    let mut label = 0;
    for q in -RADIUS..=RADIUS {
        for r in -RADIUS..=RADIUS {
            if !in_bounds(q, r) {
                continue;
            }
            let (x, y) = to_pixel(q, r);
            let active = (q, r) == selected;
            let color = if active { SELECTED } else { TILE };
            draw_hexagon(x, y, SIZE - 2.0, color);
            unsafe { ls_draw_label(label, x, y, color, active) }
            label += 1;
        }
    }
    let (x, y) = to_pixel(-3, 3);
    unsafe {
        ls_draw_circle(x, y, 15.75, 0x232634);
        ls_draw_circle(x, y, 14.25, 0xffffff);
        ls_frame_end();
    }
}

fn draw_hexagon(x: f32, y: f32, radius: f32, color: u32) {
    if !x.is_finite() || !y.is_finite() || !radius.is_finite() || radius <= 0.0 {
        return;
    }
    unsafe { ls_draw_hexagon(x, y, radius, color) }
}

extern "C" fn cleanup() {}

extern "C" fn key_down(key_code: i32) {
    let (dq, dr) = match key_code {
        87 => (0, -1), // W
        83 => (0, 1),  // S
        69 => (1, -1), // E
        68 => (1, 0),  // D
        81 => (-1, 0), // Q
        65 => (-1, 1), // A
        KEY_ESCAPE => {
            unsafe { ls_quit() };
            return;
        }
        _ => return,
    };
    let q = SELECTED_Q.load(Ordering::Relaxed) + dq;
    let r = SELECTED_R.load(Ordering::Relaxed) + dr;
    if in_bounds(q, r) {
        SELECTED_Q.store(q, Ordering::Relaxed);
        SELECTED_R.store(r, Ordering::Relaxed);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn main(_argc: c_int, _argv: *const *const c_char) -> c_int {
    let desc = Desc {
        title: c"corela".as_ptr(),
        width: WINDOW_WIDTH,
        height: WINDOW_HEIGHT,
        init,
        frame,
        cleanup,
        key_down,
    };
    unsafe { ls_run(&desc) }
}
