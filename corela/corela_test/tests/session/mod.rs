//! One corela session: a private copy of corela, started, driven through
//! its window with xdotool, and left to save its replay and quit.
//!
//! corela reads the `regina.toml` next to its executable, so each session
//! gets its own copy of both in `target/tmp/session-<pid>`, where they stay
//! after the run along with `corela.log` and the replay.

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

const PRODUCT_NAME: &str = "corela_bpmn";
pub const NORTH_KEY: &str = "w";
pub const SOUTH_KEY: &str = "s";
/// How long corela gets to open its window, and to quit after the last action.
const TIMEOUT: Duration = Duration::from_secs(15);
const POLL: Duration = Duration::from_millis(50);
/// How long a key gets to show up in corela's log before it is sent again.
const PRESS_WAIT: Duration = Duration::from_secs(1);
const PRESS_ATTEMPTS: u32 = 3;

#[derive(Debug)]
pub struct Session {
    /// The private copy of corela: executable, config, log and replay.
    dir: PathBuf,
    corela: Option<Child>,
    window: String,
    /// Keys pressed so far, each one an action.
    pressed: usize,
}

impl Session {
    /// Starts a private corela that saves its replay and quits after
    /// `last_action` actions, and waits for its window.
    pub fn start(last_action: u32) -> Self {
        let dir =
            Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("session-{}", std::process::id()));
        let _ignored = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("sandbox directory");
        let binary = dir.join(PRODUCT_NAME);
        let built = corela_home().join("target/debug").join(PRODUCT_NAME);
        fs::hard_link(&built, &binary).expect("corela binary (build corela first)");
        fs::write(dir.join("regina.toml"), session_config(last_action)).expect("regina.toml");

        let log = File::create(dir.join("corela.log")).expect("corela.log");
        let corela = Command::new(&binary)
            .current_dir(&dir)
            .env("RUST_LOG", "info")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(log)
            .spawn()
            .expect("cannot start corela");
        let pid = corela.id().to_string();
        let mut session = Self {
            dir,
            corela: Some(corela),
            window: String::new(),
            pressed: 0,
        };

        let deadline = Instant::now() + TIMEOUT;
        loop {
            let found = xdotool(&[
                "search",
                "--onlyvisible",
                "--pid",
                &pid,
                "--name",
                "^corela_bpmn$",
            ]);
            if let Some(windows) = found
                && let Some(window) = windows.lines().next()
            {
                session.window = window.to_owned();
                return session;
            }
            assert!(
                Instant::now() < deadline,
                "corela window did not appear:\n{}",
                session.log()
            );
            thread::sleep(POLL);
        }
    }

    /// Presses `key` in corela's window, then waits until corela logs it as
    /// the next action. A live X display sometimes loses or reorders keys
    /// sent in a burst, so keys go one at a time, and a lost one is sent
    /// again. The log only paces the input; the replay is what gets checked.
    pub fn press(&mut self, key: &str) {
        self.pressed += 1;
        let logged = format!("core: action {}/", self.pressed);
        for _ in 0..PRESS_ATTEMPTS {
            // corela closes its window on the last action, which can make
            // xdotool fail after the key went in
            let _pressed = xdotool(&["windowactivate", "--sync", &self.window, "key", key]);
            let deadline = Instant::now() + PRESS_WAIT;
            while Instant::now() < deadline {
                if self.log().contains(&logged) {
                    return;
                }
                thread::sleep(POLL);
            }
        }
        panic!(
            "corela did not take {key:?} as action {}:\n{}",
            self.pressed,
            self.log()
        );
    }

    /// Waits for corela to quit by itself, and returns the `actions` array
    /// of the replay it saved.
    pub fn finish(&mut self) -> Vec<Value> {
        let corela = self.corela.take().expect("corela is running");
        match wait_for_exit(corela) {
            Some(status) => assert!(
                status.success(),
                "corela exited with {status}:\n{}",
                self.log()
            ),
            None => panic!("corela did not quit:\n{}", self.log()),
        }

        let path = self.dir.join("replay/replay_1000.json");
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) => panic!("{}: {error}\n{}", path.display(), self.log()),
        };
        let replay: Value = serde_json::from_str(&text).expect("replay is JSON");
        replay["actions"]
            .as_array()
            .expect("replay has actions")
            .clone()
    }

    fn log(&self) -> String {
        fs::read_to_string(self.dir.join("corela.log")).unwrap_or_default()
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if let Some(corela) = &mut self.corela {
            let _killed = corela.kill();
            let _reaped = corela.wait();
        }
    }
}

/// The corela checkout this suite belongs to.
fn corela_home() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Runs xdotool, returning its trimmed stdout, or `None` if it failed.
fn xdotool(args: &[&str]) -> Option<String> {
    let tool = match std::env::var_os("XDOTOOL") {
        Some(tool) => tool,
        None => "xdotool".into(),
    };
    let output = Command::new(tool)
        .args(args)
        .output()
        .expect("cannot run xdotool (set XDOTOOL)");
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    } else {
        None
    }
}

/// The shipped `regina.toml`, changed to end after `last_action` actions and
/// to save its replay inside the sandbox.
fn session_config(last_action: u32) -> String {
    let home = corela_home();
    let shipped = fs::read_to_string(home.join("regina.toml")).expect("shipped regina.toml");
    let mut config = String::new();
    let mut changed = 0;
    for line in shipped.lines() {
        if line.starts_with("replay_last_action ") {
            config.push_str(&format!("replay_last_action = {last_action}"));
            changed += 1;
        } else if line.starts_with("replay_dir ") {
            config.push_str("replay_dir = \"replay\"");
            changed += 1;
        } else if let Some(icon) = line.strip_prefix("icon_chevron = ") {
            // the sandbox has no assets; point at the shipped one
            let shipped_icon = home.join(icon.trim_matches('"'));
            config.push_str(&format!("icon_chevron = \"{}\"", shipped_icon.display()));
            changed += 1;
        } else {
            config.push_str(line);
        }
        config.push('\n');
    }
    assert_eq!(
        changed, 3,
        "regina.toml no longer has the keys this test edits"
    );
    config
}

/// Waits for corela to quit; kills it and returns `None` if it does not.
fn wait_for_exit(mut corela: Child) -> Option<ExitStatus> {
    let deadline = Instant::now() + TIMEOUT;
    while Instant::now() < deadline {
        if let Some(status) = corela.try_wait().expect("corela status") {
            return Some(status);
        }
        thread::sleep(POLL);
    }
    let _killed = corela.kill();
    let _reaped = corela.wait();
    None
}
