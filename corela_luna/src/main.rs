mod config;
mod corela;
mod dto;
mod entity;
mod regina;

use std::process::ExitCode;

use corela_adahira::Window;

use crate::config::load_config;
use crate::corela::Corela;
use crate::regina::find_regina;

const WINDOW_TITLE: &str = "corela_bpmn";
const DEFAULT_LOG_LEVEL: &str = "info";

fn main() -> ExitCode {
    let log_environment = env_logger::Env::default().default_filter_or(DEFAULT_LOG_LEVEL);
    env_logger::Builder::from_env(log_environment).init();

    match start() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            log::error!("corela: {error}");
            ExitCode::FAILURE
        }
    }
}

fn start() -> Result<(), String> {
    let path = find_regina()?;
    let config = load_config(&path)?;
    log::info!("{}_v{}", config.name, config.version);
    log::debug!("configuration_{}", path.display());

    let window = Window {
        title: WINDOW_TITLE,
        width: config.window_width,
        height: config.window_height,
    };
    let app = Corela::new(config);
    corela_adahira::run(&window, Box::new(app));
    Ok(())
}
