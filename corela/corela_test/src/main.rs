//! C-esque test runner for `corela_test`.
//!
//! Redesigned to use 0 for success, 1 for failure, and if the sum of all tests
//! is still 0 then all of the tests pass.

use std::process::ExitCode;

use corela_test::{
    config_rejects, gui_keyboard, gui_replay, gui_startup, svg_assets,
};

struct TestCase {
    name: &'static str,
    tier: &'static str,
    run: fn() -> i32,
}

const TESTS: &[TestCase] = &[
    // config_rejects (headless)
    TestCase {
        name: "missing_config_file",
        tier: "headless",
        run: config_rejects::missing_config_file,
    },
    TestCase {
        name: "config_for_another_version_is_ignored",
        tier: "headless",
        run: config_rejects::config_for_another_version_is_ignored,
    },
    TestCase {
        name: "config_for_another_product_is_ignored",
        tier: "headless",
        run: config_rejects::config_for_another_product_is_ignored,
    },
    TestCase {
        name: "malformed_toml_is_ignored",
        tier: "headless",
        run: config_rejects::malformed_toml_is_ignored,
    },
    TestCase {
        name: "missing_key",
        tier: "headless",
        run: config_rejects::missing_key,
    },
    TestCase {
        name: "string_where_integer_expected",
        tier: "headless",
        run: config_rejects::string_where_integer_expected,
    },
    TestCase {
        name: "integer_where_string_expected",
        tier: "headless",
        run: config_rejects::integer_where_string_expected,
    },
    TestCase {
        name: "role_names_a_color_that_does_not_exist",
        tier: "headless",
        run: config_rejects::role_names_a_color_that_does_not_exist,
    },
    TestCase {
        name: "icon_file_does_not_exist",
        tier: "headless",
        run: config_rejects::icon_file_does_not_exist,
    },
    TestCase {
        name: "key_code_that_overflows_32_bits",
        tier: "headless",
        run: config_rejects::key_code_that_overflows_32_bits,
    },
    TestCase {
        name: "key_code_zero",
        tier: "headless",
        run: config_rejects::key_code_zero,
    },
    TestCase {
        name: "key_code_negative",
        tier: "headless",
        run: config_rejects::key_code_negative,
    },
    TestCase {
        name: "key_code_just_above_range",
        tier: "headless",
        run: config_rejects::key_code_just_above_range,
    },
    TestCase {
        name: "window_width_zero",
        tier: "headless",
        run: config_rejects::window_width_zero,
    },
    TestCase {
        name: "window_height_beyond_x11_limit",
        tier: "headless",
        run: config_rejects::window_height_beyond_x11_limit,
    },
    TestCase {
        name: "negative_color",
        tier: "headless",
        run: config_rejects::negative_color,
    },
    TestCase {
        name: "color_with_more_than_24_bits",
        tier: "headless",
        run: config_rejects::color_with_more_than_24_bits,
    },
    TestCase {
        name: "light_theme_colors_are_checked_too",
        tier: "headless",
        run: config_rejects::light_theme_colors_are_checked_too,
    },
    // svg_assets
    TestCase {
        name: "every_fixture_has_an_expectation",
        tier: "headless",
        run: svg_assets::every_fixture_has_an_expectation,
    },
    TestCase {
        name: "invalid_svgs_are_rejected_with_the_reason",
        tier: "headless",
        run: svg_assets::invalid_svgs_are_rejected_with_the_reason,
    },
    TestCase {
        name: "valid_svgs_are_accepted",
        tier: "gui",
        run: svg_assets::valid_svgs_are_accepted,
    },
    // gui_startup
    TestCase {
        name: "valid_configurations_start",
        tier: "gui",
        run: gui_startup::valid_configurations_start,
    },
    // gui_keyboard
    TestCase {
        name: "keyboard_session",
        tier: "gui",
        run: gui_keyboard::keyboard_session,
    },
    // gui_replay
    TestCase {
        name: "undo_redo_and_replay",
        tier: "gui",
        run: gui_replay::undo_redo_and_replay,
    },
];

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut selected: Vec<&TestCase> = Vec::new();
    for test in TESTS {
        if args.is_empty() {
            selected.push(test);
        } else {
            let matches = args.iter().any(|arg| {
                if arg == "--headless" {
                    test.tier == "headless"
                } else if arg == "--gui" {
                    test.tier == "gui"
                } else if arg == "--all" {
                    true
                } else {
                    test.name.contains(arg)
                }
            });
            if matches {
                selected.push(test);
            }
        }
    }

    let mut sum: i32 = 0;
    let mut total: usize = 0;
    for test in &selected {
        total += 1;
        eprintln!("running {}", test.name);
        let status = (test.run)();
        if status == 0 {
            eprintln!("{} ... ok", test.name);
        } else {
            eprintln!("{} ... FAILED (status {status})", test.name);
        }
        sum += status;
    }

    if sum == 0 {
        eprintln!("\nAll of the tests pass ({total} tests, sum: {sum}).");
        ExitCode::SUCCESS
    } else {
        eprintln!("\nTests failed ({total} tests ran, sum: {sum}).");
        ExitCode::FAILURE
    }
}
