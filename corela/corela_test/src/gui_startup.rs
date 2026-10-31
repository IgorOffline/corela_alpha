//! Valid configurations, including values at the edges of each allowed
//! range, must start corela: the window opens and no error is logged.

use std::time::Duration;

use crate::sandbox::Sandbox;
use crate::{Checklist, product, x11};

const READY_LIMIT: Duration = Duration::from_secs(15);

/// `(test name, section, key, value)`; `None` key means "as shipped".
const CASES: [(&str, &str, Option<&str>, &str); 7] = [
    ("shipped", "", None, ""),
    ("lowest_key_code", "input", Some("input_key_start"), "1"),
    ("highest_key_code", "input", Some("input_key_arrow"), "511"),
    ("black", "theme_dark_color", Some("base_value"), "0x000000"),
    ("white", "theme_dark_color", Some("base_value"), "0xFFFFFF"),
    ("smallest_window_width", "gfx", Some("window_width"), "1"),
    ("small_window_height", "gfx", Some("window_height"), "200"),
];

#[must_use]
pub fn valid_configurations_start() -> i32 {
    let _desktop = x11::desktop();
    let mut checks = Checklist::default();
    for (name, section, key, value) in CASES {
        let mut sandbox = Sandbox::new(&format!("accept_{name}"));
        if let Some(key) = key {
            sandbox.config().set(section, key, value);
        }
        let app = sandbox.launch();
        let ready = app.wait_for(product::READY_LINE, READY_LIMIT);
        let stderr = app.stderr();
        let clean = !stderr.contains(product::ERROR_TAG) && !stderr.contains(product::PANIC_TEXT);
        let setting = match key {
            Some(key) => format!("[{section}] {key} = {value}"),
            None => String::from("shipped regina.toml"),
        };
        checks.check(ready && clean, format!("{setting}: started cleanly"));
    }
    checks.finish()
}
