use std::path::{Path, PathBuf};

use crate::config::{CONFIG_FILE_NAME, KEY_NAME, KEY_VERSION, SECTION_META};

const NAME: &str = env!("CARGO_BIN_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");
const CONFIG_SEARCH_DEPTH: usize = 3;

fn meta_string(table: &toml::Table, key: &str) -> Option<String> {
    let section = table.get(SECTION_META)?;
    let value = section.get(key)?;
    let text = value.as_str()?;
    Some(String::from(text))
}

fn belongs(path: &Path) -> bool {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(_) => return false,
    };
    let table = match text.parse::<toml::Table>() {
        Ok(table) => table,
        Err(_) => return false,
    };
    let name = meta_string(&table, KEY_NAME);
    let version = meta_string(&table, KEY_VERSION);
    name == Some(String::from(NAME)) && version == Some(String::from(VERSION))
}

pub(crate) fn find_regina() -> Result<PathBuf, String> {
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(error) => return Err(format!("cannot_locate_the_corela_executable: {error}")),
    };
    let mut directory = exe.parent();
    for _ in 0..CONFIG_SEARCH_DEPTH {
        let current = match directory {
            Some(current) => current,
            None => break,
        };
        let candidate = current.join(CONFIG_FILE_NAME);
        if belongs(&candidate) {
            return Ok(candidate);
        }
        directory = current.parent();
    }
    Err(format!(
        "no_{CONFIG_FILE_NAME}_for_{NAME}_{VERSION}_within_{CONFIG_SEARCH_DEPTH}_directories_of_{}",
        exe.display()
    ))
}
