mod icon;
mod key;

pub use icon::Icon;
pub use key::Key;

use std::sync::atomic::{AtomicBool, Ordering};

use macroquad::color::Color;
use macroquad::input::KeyCode;
use macroquad::miniquad;
use macroquad::text::{Font, TextDimensions, TextParams};
use macroquad::texture::{DrawTextureParams, Texture2D};

#[derive(Clone, Copy, Debug)]
pub struct Geometry {
    pub cell_marker_outer_radius: f32,
    pub cell_marker_inner_radius: f32,
    pub hex_radius: i32,
    pub hex_size: f32,
    pub hex_tile_inset: f32,
}

const GEOMETRY: Geometry = Geometry {
    cell_marker_outer_radius: 15.75,
    cell_marker_inner_radius: 14.25,
    hex_radius: 4,
    hex_size: 32.0,
    hex_tile_inset: 2.0,
};

#[must_use]
pub const fn geometry() -> Geometry {
    GEOMETRY
}

pub type Point = (f32, f32);

pub struct Window {
    pub title: &'static str,
    pub width: i32,
    pub height: i32,
}

pub trait App {
    fn frame(&mut self, frame: &mut Frame<'_>);
    fn key_down(&mut self, key: Key);
    fn cleanup(&mut self) {}
}

const MSAA_SAMPLES: i32 = 4;
const HEX_CORNERS: u8 = 6;
const CIRCLE_SEGMENTS: u8 = 64;
const UI_FONT_SIZE: u16 = 20;
const LABEL_FONT_SIZE: u16 = 20;
const TEXT_MARGIN: f32 = 16.0;
const LINE_METRICS_SAMPLE: &str = "Ag";
const LABEL_SELECTED_OPACITY: f32 = 1.0;
const LABEL_UNSELECTED_OPACITY: f32 = 0.16;

static FONT: &[u8] = include_bytes!("../asset/font/IntelOneMono-Medium.ttf");

static QUIT: AtomicBool = AtomicBool::new(false);

struct IconTexture {
    icon_id: u32,
    texture: Texture2D,
}

struct IconTextures {
    loaded: Vec<IconTexture>,
}

impl IconTextures {
    const fn new() -> Self {
        Self { loaded: Vec::new() }
    }

    fn get(&mut self, icon: &Icon) -> Texture2D {
        for loaded in &self.loaded {
            if loaded.icon_id == icon.id() {
                return loaded.texture.clone();
            }
        }
        let texture = Texture2D::from_image(icon.image());
        self.loaded.push(IconTexture {
            icon_id: icon.id(),
            texture: texture.clone(),
        });
        texture
    }
}

pub struct Frame<'a> {
    font: Option<&'a Font>,
    icon_textures: &'a mut IconTextures,
}

#[expect(
    clippy::unused_self,
    clippy::needless_pass_by_ref_mut,
    reason = "Frame is a capability token: `&mut self` proves a frame is open"
)]
impl Frame<'_> {
    pub fn clear(&mut self, color: u32) {
        macroquad::window::clear_background(rgb(color));
    }

    pub fn hexagon(&mut self, center: Point, radius: f32, color: u32) {
        let (x, y) = center;
        if !x.is_finite() || !y.is_finite() || !radius.is_finite() || radius <= 0.0 {
            return;
        }
        macroquad::shapes::draw_poly(x, y, HEX_CORNERS, radius, 0.0, rgb(color));
    }

    pub fn circle(&mut self, center: Point, radius: f32, color: u32) {
        let (x, y) = center;
        macroquad::shapes::draw_poly(x, y, CIRCLE_SEGMENTS, radius, 0.0, rgb(color));
    }

    pub fn label(&mut self, text: &str, center: Point, color: u32, selected: bool) {
        let mut faded = rgb(color);
        if selected {
            faded.a = LABEL_SELECTED_OPACITY;
        } else {
            faded.a = LABEL_UNSELECTED_OPACITY;
        }
        let size = self.measure(text, LABEL_FONT_SIZE);
        let (x, y) = center;
        let left = (x - size.width / 2.0).round();
        let baseline = (y - size.height / 2.0 + size.offset_y).round();
        self.text(text, left, baseline, LABEL_FONT_SIZE, faded);
    }

    pub fn header(&mut self, text: &str, color: u32) {
        let line = self.measure(LINE_METRICS_SAMPLE, UI_FONT_SIZE);
        let baseline = (TEXT_MARGIN + line.offset_y).round();
        self.text(text, TEXT_MARGIN, baseline, UI_FONT_SIZE, rgb(color));
    }

    pub fn footer(&mut self, text: &str, color: u32) {
        let line = self.measure(LINE_METRICS_SAMPLE, UI_FONT_SIZE);
        let descent = line.height - line.offset_y;
        let bottom = macroquad::window::screen_height() - TEXT_MARGIN;
        let baseline = (bottom - descent).round();
        self.text(text, TEXT_MARGIN, baseline, UI_FONT_SIZE, rgb(color));
    }

    pub fn icon(&mut self, icon: &Icon, center: Point, angle: f32, color: u32) {
        let texture = self.icon_textures.get(icon);
        let (x, y) = center;
        let left = (x - texture.width() / 2.0).round();
        let top = (y - texture.height() / 2.0).round();
        let params = DrawTextureParams {
            rotation: angle,
            ..DrawTextureParams::default()
        };
        macroquad::texture::draw_texture_ex(&texture, left, top, rgb(color), params);
    }

    fn measure(&self, text: &str, font_size: u16) -> TextDimensions {
        macroquad::text::measure_text(text, self.font, font_size, 1.0)
    }

    fn text(&self, text: &str, x: f32, baseline: f32, font_size: u16, color: Color) {
        let params = TextParams {
            font: self.font,
            font_size,
            color,
            ..TextParams::default()
        };
        macroquad::text::draw_text_ex(text, x, baseline, params);
    }
}

const HEX_COLUMN_SPACING_TO_SIZE_RATIO: f32 = 1.5;
const HEX_ROW_SPACING_TO_SIZE_RATIO: f32 = 1.732_050_8;

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
        let x = self.origin.0 + self.size * HEX_COLUMN_SPACING_TO_SIZE_RATIO * column;
        let y = self.origin.1 + self.size * HEX_ROW_SPACING_TO_SIZE_RATIO * (row + column / 2.0);
        (x, y)
    }
}

#[must_use]
pub fn small_to_f32(value: i32) -> f32 {
    match i16::try_from(value) {
        Ok(small) => f32::from(small),
        Err(_) => f32::NAN,
    }
}

const fn rgb(hex: u32) -> Color {
    let [_, red, green, blue] = hex.to_be_bytes();
    Color::from_rgba(red, green, blue, u8::MAX)
}

struct KeyForwarder<'a> {
    app: &'a mut dyn App,
}

impl miniquad::EventHandler for KeyForwarder<'_> {
    fn update(&mut self) {}

    fn draw(&mut self) {}

    fn key_down_event(&mut self, code: KeyCode, _modifiers: miniquad::KeyMods, repeat: bool) {
        if repeat {
            return;
        }
        if let Some(key) = Key::from_code(code) {
            self.app.key_down(key);
        }
    }
}

fn load_font() -> Option<Font> {
    match macroquad::text::load_ttf_font_from_bytes(FONT) {
        Ok(font) => Some(font),
        Err(error) => {
            log::error!("gfx: bundled font failed to load, using macroquad's: {error}");
            None
        }
    }
}

#[expect(
    clippy::future_not_send,
    reason = "macroquad polls this future on the thread that made it, and never sends it"
)]
async fn main_loop(mut app: Box<dyn App>) {
    macroquad::input::prevent_quit();
    let subscriber = macroquad::input::utils::register_input_subscriber();
    let font = load_font();
    let mut icon_textures = IconTextures::new();
    loop {
        let mut forwarder = KeyForwarder { app: &mut *app };
        macroquad::input::utils::repeat_all_miniquad_input(&mut forwarder, subscriber);
        if QUIT.load(Ordering::Acquire) || macroquad::input::is_quit_requested() {
            break;
        }
        let mut frame = Frame {
            font: font.as_ref(),
            icon_textures: &mut icon_textures,
        };
        app.frame(&mut frame);
        macroquad::window::next_frame().await;
    }
    app.cleanup();
}

#[expect(
    clippy::large_stack_frames,
    reason = "miniquad's Conf holds its window icon inline (21 KiB) and from_config takes it by value"
)]
pub fn run(window: &Window, app: Box<dyn App>) {
    QUIT.store(false, Ordering::Release);
    let platform = miniquad::conf::Platform {
        linux_backend: miniquad::conf::LinuxBackend::X11Only,
        linux_wm_class: window.title,
        ..miniquad::conf::Platform::default()
    };
    let conf = macroquad::conf::Conf {
        miniquad_conf: miniquad::conf::Conf {
            window_title: String::from(window.title),
            window_width: window.width,
            window_height: window.height,
            window_resizable: false,
            high_dpi: false,
            sample_count: MSAA_SAMPLES,
            platform,
            ..miniquad::conf::Conf::default()
        },
        ..macroquad::conf::Conf::default()
    };
    macroquad::Window::from_config(conf, main_loop(app));
}

pub fn quit() {
    QUIT.store(true, Ordering::Release);
}
