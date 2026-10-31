//! The chevron icon is an SVG file named by `[asset] icon_chevron`.
//!
//! An invalid SVG must stop startup with one error line that names the file
//! and the problem; this needs no desktop. A valid one must let corela start;
//! that opens a window, so it is in the GUI tier.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::sandbox::Sandbox;
use crate::{Checklist, product, x11};

const EXIT_LIMIT: Duration = Duration::from_secs(10);
const READY_LIMIT: Duration = Duration::from_secs(15);

/// Every fixture and what corela must do with it. `None` means "accepted".
const FIXTURES: [(&str, Option<&str>); 23] = [
    ("01_abs.svg", None),
    ("02_rel.svg", None),
    ("03_hole.svg", None),
    ("04_single_quotes.svg", None),
    ("05_decoy.svg", None),
    ("06_numbers.svg", None),
    ("07_unicode.svg", None),
    ("08_unicode_attr_value.svg", None),
    ("09_unicode_before_name.svg", None),
    ("10_multi_implicit.svg", None),
    ("e01_no_svg.svg", Some("no <svg> element")),
    ("e02_no_viewbox.svg", Some("<svg> has no viewBox")),
    ("e03_no_path.svg", Some("no <path> data")),
    ("e04_no_d.svg", Some("<path> has no d")),
    ("e05_arc.svg", Some("arc commands are not supported")),
    ("e06_bad_cmd.svg", Some("unknown path command")),
    ("e07_bad_num.svg", Some("malformed number")),
    (
        "e08_viewbox_zero.svg",
        Some("viewBox must have a positive width and height"),
    ),
    ("e09_viewbox_short.svg", Some("malformed number")),
    ("e10_unquoted.svg", Some("<svg> has no viewBox")),
    ("e11_unicode_number.svg", Some("malformed number")),
    ("e12_viewbox_unicode.svg", Some("malformed number")),
    ("e13_dangling_exponent.svg", Some("malformed number")),
];

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/svg")
}

fn sandbox_with(fixture: &str) -> Sandbox {
    let name = format!("svg_{}", fixture.trim_end_matches(".svg"));
    let sandbox = Sandbox::new(&name);
    sandbox.use_chevron(&fixture_dir().join(fixture));
    sandbox
}

/// Guards against a fixture being added to the directory but never tested.
#[must_use]
pub fn every_fixture_has_an_expectation() -> i32 {
    let mut on_disk = Vec::new();
    let entries = match std::fs::read_dir(fixture_dir()) {
        Ok(entries) => entries,
        Err(error) => {
            eprintln!("cannot list fixtures: {error}");
            return 1;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(error) => {
                eprintln!("cannot read fixture directory entry: {error}");
                return 1;
            }
        };
        let name = entry.file_name();
        // Repository metadata is not an SVG fixture.
        if name == ".gitignore" {
            continue;
        }
        if let Ok(s) = name.into_string() {
            on_disk.push(s);
        } else {
            eprintln!("fixture filename is not valid UTF-8");
            return 1;
        }
    }
    on_disk.sort();
    let mut listed = Vec::new();
    for (name, _) in FIXTURES {
        listed.push(name.to_owned());
    }
    listed.sort();
    if on_disk != listed {
        eprintln!("fixtures on disk and in FIXTURES differ: on_disk={on_disk:?}, listed={listed:?}");
        return 1;
    }
    0
}

/// Whether every line contains `wanted`.
fn all_contain(lines: &[&str], wanted: &str) -> bool {
    for line in lines {
        if !line.contains(wanted) {
            return false;
        }
    }
    true
}

#[must_use]
pub fn invalid_svgs_are_rejected_with_the_reason() -> i32 {
    let mut checks = Checklist::default();
    for (fixture, expected) in FIXTURES {
        let reason = match expected {
            Some(reason) => reason,
            None => continue,
        };
        let outcome = sandbox_with(fixture).run(EXIT_LIMIT);
        let errors = outcome.lines_with(product::ERROR_TAG);
        let wanted = format!("chevron.svg: {reason}");
        let passed = outcome.status == Some(product::EXIT_STARTUP_ERROR)
            && errors.len() == 1
            && all_contain(&errors, &wanted)
            && outcome.lines_with(product::PANIC_TEXT).is_empty();
        checks.check(
            passed,
            format!(
                "{fixture}: expected {wanted:?}, got status {:?}, {errors:?}",
                outcome.status
            ),
        );
    }
    checks.finish()
}

#[must_use]
pub fn valid_svgs_are_accepted() -> i32 {
    let _desktop = x11::desktop();
    let mut checks = Checklist::default();
    for (fixture, expected) in FIXTURES {
        if expected.is_some() {
            continue;
        }
        let app = sandbox_with(fixture).launch();
        let ready = app.wait_for(product::READY_LINE, READY_LIMIT);
        let stderr = app.stderr();
        let clean = !stderr.contains(product::ERROR_TAG) && !stderr.contains(product::PANIC_TEXT);
        checks.check(ready && clean, format!("{fixture}: started cleanly"));
    }
    checks.finish()
}
