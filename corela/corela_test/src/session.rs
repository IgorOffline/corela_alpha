//! A running corela window driven from the keyboard, judging each key
//! press by the events it logs.

use std::time::{Duration, Instant};

use crate::Checklist;
use crate::product;
use crate::sandbox::{RunningApp, Sandbox};
use crate::x11::{self, Window};

const READY_LIMIT: Duration = Duration::from_secs(15);
const EVENT_LIMIT: Duration = Duration::from_secs(3);
/// How long to wait for the events after the first one.
const EVENT_SETTLE: Duration = Duration::from_millis(100);
const CAPTURE_LIMIT: Duration = Duration::from_secs(10);
const EXIT_LIMIT: Duration = Duration::from_secs(5);
const POLL: Duration = Duration::from_millis(50);

pub struct Session {
    pub app: RunningApp,
    pub window: Window,
    pub checks: Checklist,
    /// The action the user is on, as corela's counter shows it.
    action: i32,
    last_action: i32,
    /// Index into `product::MODES` of the input mode corela is in.
    mode: usize,
}

impl Session {
    /// Launches `sandbox` and waits for its window. `last_action` must
    /// match the sandbox's `[replay] replay_last_action`.
    ///
    /// # Panics
    /// If corela does not start or its window does not appear.
    #[must_use]
    pub fn start(sandbox: &Sandbox, last_action: i32) -> Self {
        let app = sandbox.launch();
        assert!(
            app.wait_for(product::READY_LINE, READY_LIMIT),
            "corela did not start; stderr:\n{}",
            app.stderr()
        );
        let window = Window::find(READY_LIMIT);
        Self {
            app,
            window,
            checks: Checklist::default(),
            action: 1,
            last_action,
            mode: 0,
        }
    }

    /// The event `note` logged on the current action.
    #[must_use]
    pub fn event(&self, note: &str) -> String {
        product::action(self.action, self.last_action, note)
    }

    /// Presses `key` and returns the events it logged.
    #[must_use]
    pub fn press_raw(&self, key: &str) -> Vec<String> {
        let before = self.app.events().len();
        self.window.press(key);
        let deadline = Instant::now() + EVENT_LIMIT;
        while Instant::now() < deadline && self.app.events().len() == before {
            std::thread::sleep(POLL);
        }
        // let a further, unexpected event show up if there is one
        std::thread::sleep(EVENT_SETTLE);
        let events = self.app.events();
        events.get(before..).unwrap_or(&[]).to_vec()
    }

    /// Presses `key`, a key that is one action, and checks that it logs
    /// exactly `note` for the current action. Then moves to the next action.
    pub fn act(&mut self, key: &str, note: &str, what: &str) {
        self.press_expecting(key, note, what);
        self.action += 1;
    }

    /// Presses the cycle key until corela is in input mode `name`.
    ///
    /// # Panics
    /// If `name` is not one of `product::MODES`.
    pub fn select_mode(&mut self, name: &str) {
        let target = product::MODES.iter().position(|mode| *mode == name);
        let target = target.unwrap_or_else(|| panic!("unknown input mode {name}"));
        while self.mode != target {
            self.mode = (self.mode + 1) % product::MODES.len();
            let note = product::input_mode(product::MODES[self.mode]);
            self.view(product::keys::CYCLE, &note, "cycle the input mode");
        }
    }

    /// Presses `key`, a key that is not an action (theme, quit), and checks
    /// that it logs exactly `note` for the current action.
    pub fn view(&mut self, key: &str, note: &str, what: &str) {
        self.press_expecting(key, note, what);
    }

    fn press_expecting(&mut self, key: &str, note: &str, what: &str) {
        let wanted = vec![self.event(note)];
        let new = self.press_raw(key);
        self.checks.check(
            new == wanted,
            format!("{key:<6} {what}: expected {wanted:?}, got {new:?}"),
        );
    }

    #[must_use]
    pub fn capture(&self) -> x11::Frame {
        self.window.capture(CAPTURE_LIMIT)
    }

    /// Checks that corela has exited with status `EXIT_OK`.
    pub fn check_exit(&mut self, after: &str) {
        let status = self.app.wait_exit(EXIT_LIMIT);
        self.checks.check(
            status == Some(product::EXIT_OK),
            format!(
                "exits with status {} after {after}, got {status:?}",
                product::EXIT_OK
            ),
        );
    }
}
