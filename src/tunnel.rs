use cocoa::base::{id, BOOL, NO, YES};
use cocoa::foundation::NSString;
use log::{debug, error, info, warn};
use objc::runtime::{Object, Sel};
use objc::{msg_send, sel, sel_impl};
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

const PATH: &str = "/bin:/usr/bin:/usr/local/bin:/sbin:/usr/sbin";

#[derive(Clone)]
pub struct TunnelCommand {
    pub command: String,
    pub args: Vec<String>,
    pub kill_command: String,
    pub kill_args: Vec<String>,
}

static TUNNEL_PROCESS: AtomicBool = AtomicBool::new(false);
pub static mut COMMANDS_CONFIG: Option<Arc<Mutex<HashMap<String, TunnelCommand>>>> = None;
pub static mut ACTIVE_TUNNELS: Option<Arc<Mutex<HashSet<String>>>> = None;

#[no_mangle]
pub extern "C" fn toggleTunnel(_: &Object, _: Sel, item: id) {
    unsafe {
        let state: BOOL = msg_send![item, state];
        debug!("Current state: {}", state == YES);

        // Get command identifier from the menu item
        let command_id: id = msg_send![item, representedObject];
        let command_str = NSString::UTF8String(command_id);
        let command_key = std::ffi::CStr::from_ptr(command_str)
            .to_string_lossy()
            .into_owned();

        // Toggle state
        let new_state = if state == YES { NO } else { YES };
        let _: () = msg_send![item, setState:new_state];

        let commands_config = COMMANDS_CONFIG.as_ref().unwrap().clone();
        let active_tunnels = ACTIVE_TUNNELS.as_ref().unwrap().clone();

        if new_state == YES {
            debug!("Starting `{}`", command_key);

            {
                let mut tunnels = active_tunnels.lock().unwrap();
                tunnels.insert(command_key.clone());
            }

            thread::spawn(move || {
                let mut attempts = 0;
                let is_tunnel_active = || {
                    let tunnels = active_tunnels.lock().unwrap();
                    tunnels.contains(&command_key)
                };

                while is_tunnel_active() && attempts < 5 {
                    let command = {
                        let config = commands_config.lock().unwrap();
                        config.get(&command_key).unwrap().clone()
                    };
                    info!(
                        "Spawning command: {} {:?} ({attempts} attempt) ",
                        command.command, command.args
                    );

                    // Update path to include /usr/local/bin (aws cli)
                    let mut cmd = Command::new(&command.command);
                    let new_path = cmd
                        .get_envs()
                        .find(|(key, _)| key == &OsStr::new("PATH"))
                        .map(|(_, value)| {
                            value.map(|path| format!("{PATH}:{}", path.to_string_lossy()))
                        })
                        .flatten()
                        .unwrap_or(PATH.to_string());
                    debug!("Update PATH to: {new_path}");
                    cmd.env("PATH", new_path);

                    // Blocking call
                    match cmd.args(&command.args).spawn() {
                        Ok(mut child) => {
                            info!("Process started");
                            let _ = child.wait();
                        }
                        Err(e) => error!("Failed to start command: {e}"),
                    }
                    debug!("Done");
                    attempts += 1;
                }

                if attempts == 5 {
                    warn!("Failed to start command after 5 attempts");
                }
            });
        } else {
            {
                let mut tunnels = active_tunnels.lock().unwrap();
                tunnels.remove(&command_key);
            }

            let command = {
                let config = commands_config.lock().unwrap();
                config.get(&command_key).unwrap().clone()
            };
            info!("Stopping command: {} {:?}", command.command, command.args);

            match Command::new(&command.kill_command)
                .args(&command.kill_args)
                .output()
            {
                Ok(_) => debug!("Process stopped"),
                Err(e) => error!("Failed to stop process: {e}"),
            }
            debug!("Done");
        }
    }
}

pub fn cleanup_tunnels() {
    unsafe {
        if let Some(commands_config) = COMMANDS_CONFIG.as_ref() {
            if let Some(active_tunnels) = ACTIVE_TUNNELS.as_ref() {
                let mut tunnels = active_tunnels.lock().unwrap();
                let config = commands_config.lock().unwrap();

                // Clear all active tunnels first to stop any running threads
                for key in tunnels.iter() {
                    debug!("Cleaning up tunnel for {}", key);
                    if let Some(command) = config.get(key) {
                        match Command::new(&command.kill_command)
                            .args(&command.kill_args)
                            .output()
                        {
                            Ok(_) => debug!("Process stopped for {}", key),
                            Err(e) => error!("Failed to stop process for {}: {}", key, e),
                        }
                    }
                }
                tunnels.clear(); // Clear all active tunnels
            }
        }
    }
}
