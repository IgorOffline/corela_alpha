#![no_std]
#![no_main]

extern crate alloc;

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
    fn gfx_run(desc: *const Desc) -> i32;
    fn gfx_quit();
    fn gfx_renderer_init(clear_r: f32, clear_g: f32, clear_b: f32);
    fn gfx_frame_begin();
    fn gfx_draw_label(index: i32, x: f32, y: f32, background: u32, selected: bool);
    fn gfx_draw_circle(x: f32, y: f32, radius: f32, color: u32);
    fn gfx_draw_double_chevron_right(x: f32, y: f32, dx: f32, dy: f32, color: u32);
    fn gfx_draw_hexagon(x: f32, y: f32, radius: f32, color: u32);
    fn gfx_frame_end();
    fn abort() -> !;
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    unsafe { abort() }
}

#[unsafe(no_mangle)]
extern "C" fn _Unwind_Resume() -> ! {
    unsafe { abort() }
}

#[unsafe(no_mangle)]
extern "C" fn rust_eh_personality() -> ! {
    unsafe { abort() }
}

struct LibcAlloc;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut u8;
    fn free(ptr: *mut u8);
    fn realloc(ptr: *mut u8, size: usize) -> *mut u8;
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut core::ffi::c_void;
    fn fread(buf: *mut u8, size: usize, count: usize, file: *mut core::ffi::c_void) -> usize;
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn fclose(file: *mut core::ffi::c_void) -> c_int;
}

unsafe impl core::alloc::GlobalAlloc for LibcAlloc {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        unsafe { malloc(layout.size()) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, _: core::alloc::Layout) {
        unsafe { free(ptr) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, _: core::alloc::Layout, new_size: usize) -> *mut u8 {
        unsafe { realloc(ptr, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: LibcAlloc = LibcAlloc;

fn read_file(path: &core::ffi::CStr) -> alloc::string::String {
    let mut bytes = alloc::vec::Vec::new();
    unsafe {
        let file = fopen(path.as_ptr(), c"rb".as_ptr());
        if file.is_null() {
            abort();
        }
        let mut chunk = [0u8; 1024];
        loop {
            let n = fread(chunk.as_mut_ptr(), 1, chunk.len(), file);
            if n == 0 {
                break;
            }
            bytes.extend_from_slice(&chunk[..n]);
        }
        fclose(file);
    }
    alloc::string::String::from_utf8(bytes).unwrap_or_else(|_| unsafe { abort() })
}

struct Config {
    name: alloc::ffi::CString,
    version: alloc::ffi::CString,
    cell: (i32, i32),
    key_escape: i32,
    key_q: i32,
    key_w: i32,
    key_e: i32,
    key_a: i32,
    key_s: i32,
    key_d: i32,
    window_width: i32,
    window_height: i32,
    radius: i32,
    size: f32,
    base: u32,
    tile: u32,
    selected: u32,
}

fn config() -> &'static Config {
    static CONFIG: core::sync::atomic::AtomicPtr<Config> =
        core::sync::atomic::AtomicPtr::new(core::ptr::null_mut());
    let existing = CONFIG.load(Ordering::Acquire);
    if !existing.is_null() {
        return unsafe { &*existing };
    }
    let text = read_file(c"regina.toml");
    let table: toml::Table = text.parse().unwrap_or_else(|_| unsafe { abort() });
    let int = |section: &str, key: &str| -> i64 {
        table
            .get(section)
            .and_then(|t| t.get(key))
            .and_then(|v| v.as_integer())
            .unwrap_or_else(|| unsafe { abort() })
    };
    let color = |role: &str| -> u32 {
        let name = table
            .get("gfx")
            .and_then(|t| t.get(role))
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| unsafe { abort() });
        let mut key = alloc::string::String::from(name);
        key.push_str("_value");
        int("color", &key) as u32
    };
    let meta = |key: &str| -> alloc::ffi::CString {
        let v = table
            .get("meta")
            .and_then(|t| t.get(key))
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| unsafe { abort() });
        alloc::ffi::CString::new(v).unwrap_or_else(|_| unsafe { abort() })
    };
    let cfg = alloc::boxed::Box::into_raw(alloc::boxed::Box::new(Config {
        name: meta("name"),
        version: meta("version"),
        cell: (int("bpmn", "cell_x") as i32, int("bpmn", "cell_y") as i32),
        key_escape: int("gfx", "input_key_escape") as i32,
        key_q: int("gfx", "input_key_q") as i32,
        key_w: int("gfx", "input_key_w") as i32,
        key_e: int("gfx", "input_key_e") as i32,
        key_a: int("gfx", "input_key_a") as i32,
        key_s: int("gfx", "input_key_s") as i32,
        key_d: int("gfx", "input_key_d") as i32,
        window_width: int("gfx", "window_width") as i32,
        window_height: int("gfx", "window_height") as i32,
        radius: int("gfx", "hex_radius") as i32,
        size: int("gfx", "hex_size") as f32,
        base: color("color_base"),
        tile: color("color_tile"),
        selected: color("color_selected"),
    }));
    CONFIG.store(cfg, Ordering::Release);
    unsafe { &*cfg }
}

fn radius() -> i32 {
    config().radius
}
fn size() -> f32 {
    config().size
}
fn window_width() -> i32 {
    config().window_width
}
fn window_height() -> i32 {
    config().window_height
}

fn bpmn_cell() -> (i32, i32) {
    config().cell
}
const CHEVRON_OFFSETS: [(i32, i32); 6] = [(1, -1), (1, 0), (0, 1), (-1, 1), (-1, 0), (0, -1)];
static SELECTED_Q: AtomicI32 = AtomicI32::new(0);
static SELECTED_R: AtomicI32 = AtomicI32::new(0);

fn in_bounds(q: i32, r: i32) -> bool {
    q.abs().max(r.abs()).max((q + r).abs()) <= radius()
}

fn to_pixel(q: i32, r: i32) -> (f32, f32) {
    (
        window_width() as f32 / 2.0 + size() * 1.5 * q as f32,
        window_height() as f32 / 2.0 + size() * 1.732_050_8 * (r as f32 + q as f32 / 2.0),
    )
}

const fn channel(hex: u32, shift: u32) -> f32 {
    ((hex >> shift) & 0xff) as f32 / 255.0
}

const fn rgb(hex: u32) -> [f32; 3] {
    [channel(hex, 16), channel(hex, 8), channel(hex, 0)]
}

extern "C" fn init() {
    let [r, g, b] = rgb(config().base);
    unsafe { gfx_renderer_init(r, g, b) }
}

extern "C" fn frame() {
    unsafe { gfx_frame_begin() }
    let selected = (
        SELECTED_Q.load(Ordering::Relaxed),
        SELECTED_R.load(Ordering::Relaxed),
    );
    let mut label = 0;
    for q in -radius()..=radius() {
        for r in -radius()..=radius() {
            if !in_bounds(q, r) {
                continue;
            }
            let (x, y) = to_pixel(q, r);
            let active = (q, r) == selected;
            let color = if active {
                config().selected
            } else {
                config().tile
            };
            draw_hexagon(x, y, size() - 2.0, color);
            let occupied = (q, r) == bpmn_cell()
                || CHEVRON_OFFSETS
                    .iter()
                    .any(|&(dq, dr)| (q, r) == (bpmn_cell().0 + dq, bpmn_cell().1 + dr));
            if !occupied {
                unsafe { gfx_draw_label(label, x, y, color, active) }
            }
            label += 1;
        }
    }
    let (x, y) = to_pixel(bpmn_cell().0, bpmn_cell().1);
    unsafe {
        gfx_draw_circle(x, y, 15.75, 0x232634);
        gfx_draw_circle(x, y, 14.25, 0xffffff);
    }

    for (dq, dr) in CHEVRON_OFFSETS {
        let (cell_x, cell_y) = to_pixel(bpmn_cell().0 + dq, bpmn_cell().1 + dr);
        unsafe { gfx_draw_double_chevron_right(cell_x, cell_y, cell_x - x, cell_y - y, 0xef9f76) }
    }
    unsafe {
        gfx_frame_end();
    }
}

fn draw_hexagon(x: f32, y: f32, radius: f32, color: u32) {
    if !x.is_finite() || !y.is_finite() || !radius.is_finite() || radius <= 0.0 {
        return;
    }
    unsafe { gfx_draw_hexagon(x, y, radius, color) }
}

extern "C" fn cleanup() {}

extern "C" fn key_down(key_code: i32) {
    let (dq, dr) = match key_code {
        k if k == config().key_w => (0, -1),
        k if k == config().key_s => (0, 1),
        k if k == config().key_e => (1, -1),
        k if k == config().key_d => (1, 0),
        k if k == config().key_q => (-1, 0),
        k if k == config().key_a => (-1, 1),
        k if k == config().key_escape => {
            unsafe { gfx_quit() };
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
    let cfg = config();
    unsafe { printf(c"%s %s\n".as_ptr(), cfg.name.as_ptr(), cfg.version.as_ptr()) };
    let desc = Desc {
        title: c"corela".as_ptr(),
        width: window_width(),
        height: window_height(),
        init,
        frame,
        cleanup,
        key_down,
    };
    unsafe { gfx_run(&desc) }
}
