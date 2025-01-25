// src/logger.rs
//
// Single responsibility: setting up or providing the logger

use log::LevelFilter;
use oslog::OsLogger;

/// Initializes the logger for the entire application.
/// Typically called early in `main()`.
pub fn init_logger() {
    OsLogger::new("com.1000ants.menubarapp")
        .level_filter(LevelFilter::Debug)
        .init()
        .unwrap();
}
