//! nanovg :: path data supports `M L H V C S Q T Z` in absolute and relative form; arcs are rejected.

use alloc::string::String;
use alloc::vec::Vec;

/// The cubic Bezier drawing the same curve as a quadratic one has its control
/// points 2/3 of the way from each end point towards the quadratic control point.
const QUAD_TO_CUBIC_CONTROL_RATIO: f32 = 2.0 / 3.0;

// Indices into a viewBox, mirroring the VIEW_BOX_* enum in draw.h.
const VIEW_BOX_WIDTH: usize = 2;
const VIEW_BOX_HEIGHT: usize = 3;
const VIEW_BOX_COUNT: usize = 4;

/// Mirrors the `GFX_ICON_*` enum in draw.h; keep the order in sync.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Verb {
    MoveTo,
    LineTo,
    BezierTo,
    CloseSolid,
    CloseHole,
}

#[derive(Debug)]
pub struct Icon {
    view_box: [f32; VIEW_BOX_COUNT],
    verbs: Vec<Verb>,
    points: Vec<[f32; 2]>,
}

/// Mirrors `gfx_icon` in draw.h; keep the field order in sync.
#[repr(C)]
pub(crate) struct IconDesc {
    verbs: *const Verb,
    verb_count: i32,
    points: *const [f32; 2],
    view_box: [f32; VIEW_BOX_COUNT],
}

impl Icon {
    pub fn parse(svg: &str) -> Result<Icon, &'static str> {
        let mut view_box: Option<[f32; VIEW_BOX_COUNT]> = None;
        let mut builder = Builder::default();
        let mut is_first_piece = true;
        for piece in svg.split('<') {
            // the text before the first '<' is not a tag
            if is_first_piece {
                is_first_piece = false;
                continue;
            }
            let tag = text_before(piece, '>');
            let name = tag_name(tag);
            if name == "svg" {
                let value = match attribute(tag, "viewBox") {
                    Some(value) => value,
                    None => return Err("<svg> has no viewBox"),
                };
                view_box = Some(parse_view_box(&value)?);
            } else if name == "path" {
                let data = match attribute(tag, "d") {
                    Some(data) => data,
                    None => return Err("<path> has no d"),
                };
                builder.path(&data)?;
            }
        }
        let view_box = match view_box {
            Some(view_box) => view_box,
            None => return Err("no <svg> element"),
        };
        if builder.verbs.is_empty() {
            return Err("no <path> data");
        }
        if builder.verbs.len() > i32::MAX as usize {
            return Err("too many path commands");
        }
        Ok(Icon {
            view_box,
            verbs: builder.verbs,
            points: builder.points,
        })
    }

    pub(crate) fn desc(&self) -> IconDesc {
        IconDesc {
            verbs: self.verbs.as_ptr(),
            verb_count: self.verbs.len() as i32,
            points: self.points.as_ptr(),
            view_box: self.view_box,
        }
    }
}

/// `text` up to (not including) the first `end`, or all of `text`.
fn text_before(text: &str, end: char) -> &str {
    match text.find(end) {
        Some(index) => &text[..index],
        None => text,
    }
}

/// The element name at the start of a tag, e.g. `path` in `path d="..."/`.
fn tag_name(tag: &str) -> &str {
    for (index, c) in tag.char_indices() {
        if c.is_whitespace() || c == '/' {
            return &tag[..index];
        }
    }
    tag
}

fn ends_with_whitespace(text: &str) -> bool {
    match text.chars().next_back() {
        Some(c) => c.is_whitespace(),
        None => false,
    }
}

/// The value of `name="..."` (or `name='...'`) inside `tag`.
fn attribute(tag: &str, name: &str) -> Option<String> {
    let mut search_from = 0;
    while let Some(found) = tag[search_from..].find(name) {
        let at = search_from + found;
        search_from = at + name.len();
        // `name` must be a whole attribute name, not the end of a longer one
        if !ends_with_whitespace(&tag[..at]) {
            continue;
        }
        let after_name = tag[search_from..].trim_start();
        if !after_name.starts_with('=') {
            continue;
        }
        let after_equals = after_name[1..].trim_start();
        let quote = match after_equals.chars().next() {
            Some(c) if c == '"' || c == '\'' => c,
            _ => return None,
        };
        let value = &after_equals[1..];
        let end = value.find(quote)?;
        return Some(String::from(&value[..end]));
    }
    None
}

fn parse_view_box(value: &str) -> Result<[f32; VIEW_BOX_COUNT], &'static str> {
    let mut scanner = Scanner::new(value);
    let mut view_box = [0.0; VIEW_BOX_COUNT];
    for number in &mut view_box {
        *number = scanner.number()?;
    }
    if !scanner.is_empty() {
        return Err("viewBox must be four finite numbers");
    }
    for number in view_box {
        if !number.is_finite() {
            return Err("viewBox must be four finite numbers");
        }
    }
    if view_box[VIEW_BOX_WIDTH] <= 0.0 || view_box[VIEW_BOX_HEIGHT] <= 0.0 {
        return Err("viewBox must have a positive width and height");
    }
    Ok(view_box)
}

fn is_sign(byte: Option<u8>) -> bool {
    byte == Some(b'+') || byte == Some(b'-')
}

fn is_digit(byte: Option<u8>) -> bool {
    match byte {
        Some(byte) => byte.is_ascii_digit(),
        None => false,
    }
}

/// Reads command letters and numbers from SVG attribute text, left to right.
struct Scanner {
    text: String,
    /// Byte index of the next unread character.
    position: usize,
}

impl Scanner {
    fn new(text: &str) -> Scanner {
        Scanner {
            text: String::from(text),
            position: 0,
        }
    }

    /// The next unread character, without consuming it.
    fn peek(&self) -> Option<char> {
        self.text[self.position..].chars().next()
    }

    fn byte_at(&self, index: usize) -> Option<u8> {
        if index < self.text.len() {
            Some(self.text.as_bytes()[index])
        } else {
            None
        }
    }

    /// The index of the first non-digit at or after `index`.
    fn skip_digits(&self, index: usize) -> usize {
        let mut index = index;
        while is_digit(self.byte_at(index)) {
            index += 1;
        }
        index
    }

    /// Whitespace and commas only separate values.
    fn skip_separators(&mut self) {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() || c == ',' => self.position += c.len_utf8(),
                _ => return,
            }
        }
    }

    fn is_empty(&mut self) -> bool {
        self.skip_separators();
        self.position == self.text.len()
    }

    fn command(&mut self) -> Option<u8> {
        self.skip_separators();
        match self.peek() {
            Some(c) if c.is_ascii_alphabetic() => {
                self.position += 1;
                Some(c as u8)
            }
            _ => None,
        }
    }

    fn starts_number(&mut self) -> bool {
        self.skip_separators();
        match self.peek() {
            Some(c) => c.is_ascii_digit() || c == '+' || c == '-' || c == '.',
            None => false,
        }
    }

    /// Reads `[sign] digits [. digits] [e [sign] digits]`.
    fn number(&mut self) -> Result<f32, &'static str> {
        self.skip_separators();
        let mut end = self.position;
        if is_sign(self.byte_at(end)) {
            end += 1;
        }
        end = self.skip_digits(end);
        if self.byte_at(end) == Some(b'.') {
            end = self.skip_digits(end + 1);
        }
        if self.byte_at(end) == Some(b'e') || self.byte_at(end) == Some(b'E') {
            let mut exponent = end + 1;
            if is_sign(self.byte_at(exponent)) {
                exponent += 1;
            }
            // without digits the `e` is not part of this number
            if is_digit(self.byte_at(exponent)) {
                end = self.skip_digits(exponent);
            }
        }
        let parsed = self.text[self.position..end].parse::<f32>();
        self.position = end;
        match parsed {
            Ok(number) => Ok(number),
            Err(_) => Err("malformed number"),
        }
    }

    /// Reads an `x y` pair, offset by `origin` (zero for absolute commands).
    fn point(&mut self, origin: [f32; 2]) -> Result<[f32; 2], &'static str> {
        let x = origin[0] + self.number()?;
        let y = origin[1] + self.number()?;
        Ok([x, y])
    }
}

#[derive(Default)]
struct Builder {
    verbs: Vec<Verb>,
    points: Vec<[f32; 2]>,
    start: [f32; 2],
    current: [f32; 2],
    /// Index in `points` where the open subpath begins, if one is open.
    subpath: Option<usize>,
    /// Winding of the first closed subpath of the current `<path>`; later
    /// subpaths with the same winding are solid, the others are holes.
    reference: Option<bool>,
    last_cubic: Option<[f32; 2]>,
    last_quad: Option<[f32; 2]>,
}

impl Builder {
    fn path(&mut self, d: &str) -> Result<(), &'static str> {
        self.reference = None;
        let mut data = Scanner::new(d);
        while !data.is_empty() {
            let letter = match data.command() {
                Some(letter) => letter,
                None => return Err("expected a path command"),
            };
            // a command letter may be followed by several argument groups
            let mut first = true;
            loop {
                let origin = if letter.is_ascii_lowercase() {
                    self.current
                } else {
                    [0.0; 2]
                };
                match letter.to_ascii_uppercase() {
                    // extra pairs after a move are implicit line-tos
                    b'M' if first => {
                        let at = data.point(origin)?;
                        self.move_to(at);
                    }
                    b'M' | b'L' => {
                        let at = data.point(origin)?;
                        self.line_to(at);
                    }
                    b'H' => {
                        let x = origin[0] + data.number()?;
                        self.line_to([x, self.current[1]]);
                    }
                    b'V' => {
                        let y = origin[1] + data.number()?;
                        self.line_to([self.current[0], y]);
                    }
                    b'C' => {
                        let first_control = data.point(origin)?;
                        let second_control = data.point(origin)?;
                        let at = data.point(origin)?;
                        self.bezier_to(first_control, second_control, at);
                    }
                    b'S' => {
                        let first_control = self.reflect(self.last_cubic);
                        let second_control = data.point(origin)?;
                        let at = data.point(origin)?;
                        self.bezier_to(first_control, second_control, at);
                    }
                    b'Q' => {
                        let control = data.point(origin)?;
                        let at = data.point(origin)?;
                        self.quad_to(control, at);
                    }
                    b'T' => {
                        let control = self.reflect(self.last_quad);
                        let at = data.point(origin)?;
                        self.quad_to(control, at);
                    }
                    b'Z' => {
                        self.close();
                        break;
                    }
                    b'A' => return Err("arc commands are not supported"),
                    _ => return Err("unknown path command"),
                }
                first = false;
                if !data.starts_number() {
                    break;
                }
            }
        }
        self.close();
        Ok(())
    }

    /// Mirrors the previous control point through the current point.
    fn reflect(&self, control: Option<[f32; 2]>) -> [f32; 2] {
        match control {
            Some([x, y]) => [2.0 * self.current[0] - x, 2.0 * self.current[1] - y],
            None => self.current,
        }
    }

    fn push(&mut self, verb: Verb, points: &[[f32; 2]]) {
        self.verbs.push(verb);
        self.points.extend_from_slice(points);
        self.last_cubic = None;
        self.last_quad = None;
    }

    fn move_to(&mut self, at: [f32; 2]) {
        self.close();
        self.subpath = Some(self.points.len());
        self.push(Verb::MoveTo, &[at]);
        self.start = at;
        self.current = at;
    }

    fn ensure_open(&mut self) {
        if self.subpath.is_none() {
            self.move_to(self.current);
        }
    }

    fn line_to(&mut self, at: [f32; 2]) {
        self.ensure_open();
        self.push(Verb::LineTo, &[at]);
        self.current = at;
    }

    fn bezier_to(&mut self, first: [f32; 2], second: [f32; 2], at: [f32; 2]) {
        self.ensure_open();
        self.push(Verb::BezierTo, &[first, second, at]);
        self.current = at;
        self.last_cubic = Some(second);
    }

    fn quad_to(&mut self, control: [f32; 2], at: [f32; 2]) {
        let [x0, y0] = self.current;
        let [cx, cy] = control;
        let [x, y] = at;
        self.bezier_to(
            [
                x0 + QUAD_TO_CUBIC_CONTROL_RATIO * (cx - x0),
                y0 + QUAD_TO_CUBIC_CONTROL_RATIO * (cy - y0),
            ],
            [
                x + QUAD_TO_CUBIC_CONTROL_RATIO * (cx - x),
                y + QUAD_TO_CUBIC_CONTROL_RATIO * (cy - y),
            ],
            at,
        );
        self.last_quad = Some(control);
    }

    /// nanovg winding
    fn close(&mut self) {
        let first = match self.subpath.take() {
            Some(first) => first,
            None => return,
        };
        // shoelace formula: the sign of the area gives the winding direction
        let count = self.points.len() - first;
        let mut area = 0.0;
        for offset in 0..count {
            let [x0, y0] = self.points[first + offset];
            let [x1, y1] = self.points[first + (offset + 1) % count];
            area += x0 * y1 - x1 * y0;
        }
        let orientation = area >= 0.0;
        let reference = match self.reference {
            Some(reference) => reference,
            None => {
                self.reference = Some(orientation);
                orientation
            }
        };
        let verb = if orientation == reference {
            Verb::CloseSolid
        } else {
            Verb::CloseHole
        };
        self.push(verb, &[]);
        self.current = self.start;
    }
}
