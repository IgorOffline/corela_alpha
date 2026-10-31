//! Black-box acceptance tests for corela.
//!
//! The suite treats corela as a delivered product: it runs the executable
//! with its shipped configuration and assets, and judges it only by what a
//! user or an operator can observe. Every assumption about the product is
//! listed in [`product`].

pub mod config_rejects;
pub mod gui_keyboard;
pub mod gui_replay;
pub mod gui_startup;
pub mod product;
pub mod sandbox;
pub mod session;
pub mod svg_assets;
pub mod x11;

/// Collects the outcome of many checks in one scenario, so a failure report
/// lists all of them instead of stopping at the first.
#[derive(Default)]
pub struct Checklist {
    results: Vec<(bool, String)>,
}

impl Checklist {
    pub fn check(&mut self, passed: bool, description: impl Into<String>) {
        self.results.push((passed, description.into()));
    }

    /// Prints every check, then returns 0 if all checks passed, or 1 if any failed.
    #[must_use]
    pub fn finish(self) -> i32 {
        let mut failed = 0;
        for (passed, description) in &self.results {
            let mark = if *passed { "PASS" } else { "FAIL" };
            eprintln!("  {mark}  {description}");
            if !passed {
                failed += 1;
            }
        }
        let total = self.results.len();
        eprintln!("  {} of {total} checks passed", total - failed);
        i32::from(failed != 0)
    }
}
