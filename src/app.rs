// src/app.rs
//
// Defines the `App` structure holding shared state (commands, active tunnels).
// Also provides methods for cleanup or other global operations.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use cocoa::base::id;

use crate::tunnel::{TunnelCommand, TunnelManager};

// Wrapper type to make the status item thread-safe
pub struct StatusItemWrapper(pub id);
unsafe impl Send for StatusItemWrapper {}
unsafe impl Sync for StatusItemWrapper {}

/// Primary application structure. Contains references to any data that
/// must be shared across modules (e.g., commands, active tunnels).
pub struct App {
    pub tunnel_manager: TunnelManager,
    pub status_item: Option<Arc<Mutex<StatusItemWrapper>>>,
}

impl App {
    /// Creates a new `App` with default commands or any custom setup.
    pub fn new() -> Self {
        // Setup your commands
        let mut commands = HashMap::new();

        commands.insert(
            "prod".to_owned(),
            TunnelCommand {
                command: "ssh".to_owned(),
                args: vec!["-N".to_owned(), "lb-prod.rds".to_owned()],
                kill_command: "pkill".to_owned(),
                kill_args: vec!["-f".to_owned(), "lb-prod.rds".to_owned()],
            },
        );

        commands.insert(
            "dev-01".to_owned(),
            TunnelCommand {
                command: "ssh".to_owned(),
                args: vec!["-N".to_owned(), "lb-dev-01.rds".to_owned()],
                kill_command: "pkill".to_owned(),
                kill_args: vec!["-f".to_owned(), "lb-dev-01.rds".to_owned()],
            },
        );

        // Initialize the tunnel manager
        let tunnel_manager = TunnelManager {
            commands_config: Arc::new(Mutex::new(commands)),
            active_tunnels: Arc::new(Mutex::new(HashSet::new())),
        };

        Self {
            tunnel_manager,
            status_item: None,
        }
    }

    pub fn set_status_item(&mut self, item: id) {
        self.status_item = Some(Arc::new(Mutex::new(StatusItemWrapper(item))));
    }

    pub fn get_status_item(&self) -> Option<id> {
        self.status_item.as_ref().and_then(|wrapper| {
            wrapper.lock().ok().map(|guard| guard.0)
        })
    }

    /// Cleans up any active tunnels. Called on app termination.
    pub fn cleanup_tunnels(&self) {
        self.tunnel_manager.cleanup();
    }
}
