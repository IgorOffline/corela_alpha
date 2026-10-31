//! A private copy of corela per test: the executable, an editable
//! `regina.toml` and the assets it names, in a fresh directory.
//!
//! corela finds its configuration next to its executable, so each test
//! gets its own copy of both and can change the configuration freely
//! without touching the shipped files.

use std::io::Read;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::product;

/// Name of the chevron icon inside the sandbox.
const CHEVRON_FILE: &str = "chevron.svg";
/// How often a waiting loop looks again.
const POLL: Duration = Duration::from_millis(50);
/// Attempts at starting the executable while the kernel reports it busy.
const SPAWN_ATTEMPTS: u32 = 20;
/// Shells report death by signal N as exit status 128 + N.
const SHELL_SIGNAL_BASE: i32 = 128;

pub struct Sandbox {
    dir: PathBuf,
    config: ConfigText,
}

impl Sandbox {
    /// A fresh sandbox at `<base>/<name>` holding the shipped configuration,
    /// with its chevron icon copied in. `base` is normally the test's
    /// `env!("CARGO_TARGET_TMPDIR")`, so that no stray `regina.toml` sits in
    /// the directories above it.
    ///
    /// # Panics
    /// If the directory or its files cannot be created.
    #[must_use]
    pub fn new(name: &str) -> Self {
        let base = match std::env::var_os("CARGO_TARGET_TMPDIR") {
            Some(dir) => PathBuf::from(dir),
            None => product::home().join("target/tmp"),
        };
        Self::with_base(&base, name)
    }

    #[must_use]
    pub fn with_base(base: &Path, name: &str) -> Self {
        let dir = base.join(name);
        if dir.exists() {
            must(std::fs::remove_dir_all(&dir), &dir);
        }
        must(std::fs::create_dir_all(&dir), &dir);
        let binary = dir.join(product::PRODUCT_NAME);
        install_binary(&product::binary(), &binary);

        let mut config = ConfigText::new(&product::shipped_config());
        let shipped_chevron = product::home().join(config.value("asset", "icon_chevron"));
        let chevron = dir.join(CHEVRON_FILE);
        must(std::fs::copy(&shipped_chevron, &chevron), &chevron);
        config.set("asset", "icon_chevron", &quoted(CHEVRON_FILE));
        Self { dir, config }
    }

    /// The sandbox directory, which also holds corela's replay directory.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The configuration that `run` and `launch` will write.
    pub const fn config(&mut self) -> &mut ConfigText {
        &mut self.config
    }

    /// Uses `svg` as the chevron icon.
    ///
    /// # Panics
    /// If `svg` cannot be copied.
    pub fn use_chevron(&self, svg: &Path) {
        let chevron = self.dir.join(CHEVRON_FILE);
        must(std::fs::copy(svg, &chevron), &chevron);
    }

    /// Removes the configuration file entirely.
    pub fn without_config(&mut self) {
        self.config = ConfigText::new("");
    }

    fn write_config(&self) {
        let path = self.dir.join(product::CONFIG_FILE_NAME);
        if self.config.is_empty() {
            if path.exists() {
                must(std::fs::remove_file(&path), &path);
            }
        } else {
            must(std::fs::write(&path, self.config.text()), &path);
        }
    }

    fn spawn(&self) -> Child {
        self.write_config();
        let binary = self.dir.join(product::PRODUCT_NAME);
        let mut attempt = 1;
        loop {
            let spawned = Command::new(&binary)
                .env("RUST_LOG", "info")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn();
            match spawned {
                Ok(child) => return child,
                // see install_binary
                Err(error)
                    if error.kind() == std::io::ErrorKind::ExecutableFileBusy
                        && attempt < SPAWN_ATTEMPTS =>
                {
                    attempt += 1;
                    std::thread::sleep(POLL);
                }
                Err(error) => panic!("cannot start {}: {error}", binary.display()),
            }
        }
    }

    /// Runs corela and waits for it to exit by itself, for at most `limit`.
    #[must_use]
    pub fn run(&self, limit: Duration) -> Outcome {
        let mut app = RunningApp::start(self.spawn());
        let deadline = Instant::now() + limit;
        while Instant::now() < deadline {
            if let Some(status) = app.exit_status() {
                return Outcome {
                    status: Some(status),
                    stderr: app.finish(),
                };
            }
            std::thread::sleep(POLL);
        }
        app.kill();
        Outcome {
            status: None,
            stderr: app.finish(),
        }
    }

    /// Starts corela and leaves it running; it is killed when dropped.
    #[must_use]
    pub fn launch(&self) -> RunningApp {
        RunningApp::start(self.spawn())
    }
}

/// How a run that was expected to exit by itself ended.
pub struct Outcome {
    /// `None` if it was still running at the time limit (and was killed).
    pub status: Option<i32>,
    pub stderr: String,
}

impl Outcome {
    /// Lines of stderr containing `needle`.
    #[must_use]
    pub fn lines_with(&self, needle: &str) -> Vec<&str> {
        let mut found = Vec::new();
        for line in self.stderr.lines() {
            if line.contains(needle) {
                found.push(line);
            }
        }
        found
    }

    /// Checks the startup-failure contract: exit status 1, exactly one error
    /// line, which contains `expected`, and no panic. Returns 0 on success, 1 on failure.
    #[must_use]
    pub fn assert_rejected(&self, expected: &str) -> i32 {
        if self.status != Some(product::EXIT_STARTUP_ERROR) {
            eprintln!(
                "expected exit status {}; got {:?}; stderr:\n{}",
                product::EXIT_STARTUP_ERROR,
                self.status,
                self.stderr
            );
            return 1;
        }
        let errors = self.lines_with(product::ERROR_TAG);
        if errors.len() != 1 {
            eprintln!(
                "expected exactly one error line; got {}; stderr:\n{}",
                errors.len(),
                self.stderr
            );
            return 1;
        }
        let mentions_expected = match errors.first() {
            Some(line) => line.contains(expected),
            None => false,
        };
        if !mentions_expected {
            eprintln!(
                "error line does not mention {expected:?}; stderr:\n{}",
                self.stderr
            );
            return 1;
        }
        if !self.lines_with(product::PANIC_TEXT).is_empty() {
            eprintln!("corela panicked; stderr:\n{}", self.stderr);
            return 1;
        }
        0
    }
}

/// A running corela process, with its stderr collected in the background.
pub struct RunningApp {
    child: Child,
    stderr: Arc<Mutex<String>>,
    reader: Option<JoinHandle<()>>,
}

impl RunningApp {
    fn start(mut child: Child) -> Self {
        let stderr = Arc::new(Mutex::new(String::new()));
        let reader = match child.stderr.take() {
            Some(pipe) => {
                let sink = Arc::clone(&stderr);
                // thread::spawn only takes a closure; this one just hands
                // its two values to a named function
                Some(std::thread::spawn(move || collect_stderr(pipe, &sink)))
            }
            None => None,
        };
        Self {
            child,
            stderr,
            reader,
        }
    }

    /// Everything written to stderr so far.
    #[must_use]
    pub fn stderr(&self) -> String {
        match self.stderr.lock() {
            Ok(all) => all.clone(),
            Err(_) => String::new(),
        }
    }

    /// The gameplay events logged so far (the text after `core: `).
    #[must_use]
    pub fn events(&self) -> Vec<String> {
        let mut events = Vec::new();
        for line in self.stderr().lines() {
            if let Some((_, event)) = line.split_once(product::EVENT_PREFIX) {
                events.push(event.to_owned());
            }
        }
        events
    }

    /// Waits until stderr contains `needle`, for at most `limit`.
    #[must_use]
    pub fn wait_for(&self, needle: &str, limit: Duration) -> bool {
        let deadline = Instant::now() + limit;
        while Instant::now() < deadline {
            if self.stderr().contains(needle) {
                return true;
            }
            std::thread::sleep(POLL);
        }
        false
    }

    /// The exit status, if the process has exited. A process killed by a
    /// signal reports `128 + signal`, as shells do (an abort is 134).
    pub fn exit_status(&mut self) -> Option<i32> {
        match self.child.try_wait() {
            Ok(Some(status)) => match (status.code(), status.signal()) {
                (Some(code), _) => Some(code),
                (None, Some(signal)) => Some(SHELL_SIGNAL_BASE + signal),
                (None, None) => None,
            },
            _ => None,
        }
    }

    /// Waits for the process to exit by itself, for at most `limit`.
    pub fn wait_exit(&mut self, limit: Duration) -> Option<i32> {
        let deadline = Instant::now() + limit;
        while Instant::now() < deadline {
            if let Some(status) = self.exit_status() {
                return Some(status);
            }
            std::thread::sleep(POLL);
        }
        None
    }

    fn kill(&mut self) {
        if self.exit_status().is_none() {
            let _ignored: std::io::Result<()> = self.child.kill();
            let _reaped: std::io::Result<std::process::ExitStatus> = self.child.wait();
        }
    }

    fn finish(mut self) -> String {
        self.kill();
        if let Some(reader) = self.reader.take() {
            let _joined: std::thread::Result<()> = reader.join();
        }
        self.stderr()
    }
}

/// Appends everything read from `pipe` to `sink` until the pipe closes.
fn collect_stderr(mut pipe: ChildStderr, sink: &Mutex<String>) {
    let mut chunk = [0_u8; 4096];
    loop {
        match pipe.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(count) => {
                let text = String::from_utf8_lossy(chunk.get(..count).unwrap_or(&[]));
                if let Ok(mut all) = sink.lock() {
                    all.push_str(&text);
                }
            }
        }
    }
}

impl Drop for RunningApp {
    fn drop(&mut self) {
        self.kill();
    }
}

/// `regina.toml` as lines of text, edited by section and key. Editing text
/// (rather than re-serializing TOML) keeps everything not touched by a test
/// byte-for-byte as shipped.
pub struct ConfigText {
    lines: Vec<String>,
}

impl ConfigText {
    #[must_use]
    pub fn new(text: &str) -> Self {
        let mut lines = Vec::new();
        for line in text.lines() {
            lines.push(line.to_owned());
        }
        Self { lines }
    }

    #[must_use]
    pub fn text(&self) -> String {
        let mut text = self.lines.join("\n");
        text.push('\n');
        text
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// The line index of `key` inside `[section]`.
    fn find(&self, section: &str, key: &str) -> Option<usize> {
        let header = format!("[{section}]");
        let mut in_section = false;
        for (index, line) in self.lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                in_section = trimmed == header;
            } else if in_section
                && let Some((name, _)) = trimmed.split_once('=')
                && name.trim() == key
            {
                return Some(index);
            }
        }
        None
    }

    /// Like `find`, for a key the test knows is there.
    ///
    /// # Panics
    /// If the key does not exist (a mistake in the test).
    fn must_find(&self, section: &str, key: &str) -> usize {
        match self.find(section, key) {
            Some(index) => index,
            None => panic!("test bug: no [{section}] {key} in the config"),
        }
    }

    /// The raw value of `key` in `[section]`, without quotes or comment.
    ///
    /// # Panics
    /// If the key does not exist (a mistake in the test).
    #[must_use]
    pub fn value(&self, section: &str, key: &str) -> String {
        let index = self.must_find(section, key);
        let line = match self.lines.get(index) {
            Some(line) => line.as_str(),
            None => "",
        };
        let (_, value) = line.split_once('=').unwrap_or(("", ""));
        let (value, _) = value.split_once('#').unwrap_or((value, ""));
        value.trim().trim_matches('"').to_owned()
    }

    /// Sets `key` in `[section]` to `raw` (written as-is, so strings need
    /// their quotes).
    ///
    /// # Panics
    /// If the key does not exist (a mistake in the test).
    pub fn set(&mut self, section: &str, key: &str, raw: &str) {
        let index = self.must_find(section, key);
        if let Some(line) = self.lines.get_mut(index) {
            *line = format!("{key} = {raw}");
        }
    }

    /// Deletes `key` from `[section]`.
    ///
    /// # Panics
    /// If the key does not exist (a mistake in the test).
    pub fn remove(&mut self, section: &str, key: &str) {
        let index = self.must_find(section, key);
        self.lines.remove(index);
    }

    /// Replaces the whole text (for deliberately malformed files).
    pub fn replace_all(&mut self, text: &str) {
        *self = Self::new(text);
    }
}

/// `text` as a TOML string literal.
#[must_use]
pub fn quoted(text: &str) -> String {
    format!("\"{text}\"")
}

/// Puts the executable into the sandbox, preferably as a hard link.
///
/// A copy is open for writing while it is written, and a process that
/// another test thread forks at that moment inherits the open handle until
/// it execs. Starting the copy then fails with "Text file busy" (ETXTBSY).
/// A hard link is never open for writing, so it avoids the race; the copy
/// is only a fallback (e.g. across file systems), backed by retries in
/// `spawn`.
fn install_binary(from: &Path, to: &Path) {
    if std::fs::hard_link(from, to).is_err() {
        must(std::fs::copy(from, to), to);
    }
}

fn must<T>(result: std::io::Result<T>, path: &Path) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("{}: {error}", path.display()),
    }
}
