use std::ffi::OsStr;
use cocoa::appkit::{
    NSApplication, NSApplicationActivationPolicy, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem,
};
use cocoa::base::{id, nil, BOOL, NO, YES};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use objc::{class, msg_send, sel, sel_impl};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::collections::{HashMap, HashSet};
use log::{info, error, warn, debug};
use oslog;

const PATH: &str = "/bin:/usr/bin:/usr/local/bin:/sbin:/usr/sbin";

#[derive(Clone)]
struct TunnelCommand {
    command: String,
    args: Vec<String>,
    kill_command: String,
    kill_args: Vec<String>,
}

static TUNNEL_PROCESS: AtomicBool = AtomicBool::new(false);
static mut COMMANDS_CONFIG: Option<Arc<Mutex<HashMap<String, TunnelCommand>>>> = None;
static mut ACTIVE_TUNNELS: Option<Arc<Mutex<HashSet<String>>>> = None;

#[no_mangle]
extern "C" fn toggleTunnel(_: &Object, _: Sel, item: id) {
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
                    info!("Spawning command: {} {:?} ({attempts} attempt) ", command.command, command.args);

                    // Update path to include /usr/local/bin (aws cli)
                    let mut cmd = Command::new(&command.command);
                    let new_path = cmd.get_envs()
                        .find(|(key, _)| key == &OsStr::new("PATH"))
                        .map(|(_, value)| {
                            value.map(|path| {format!("{PATH}:{}", path.to_string_lossy())})
                        })
                        .flatten()
                        .unwrap_or(PATH.to_string());
                    debug!("Update PATH to: {new_path}");
                    cmd.env("PATH", new_path);

                    // Blocking call
                    match cmd
                        .args(&command.args)
                        .spawn() {
                            Ok(mut child) => {
                                info!("Process started");
                                let _ = child.wait();
                            },
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
                .output() {
                    Ok(_) => debug!("Process stopped"),
                    Err(e) => error!("Failed to stop process: {e}"),
            }
            debug!("Done");
        }
    }
}

fn register_selector() -> *const Class {
    unsafe {
        let superclass = class!(NSObject);
        let mut decl = ClassDecl::new("MenuHandler", superclass).unwrap();

        decl.add_method(
            sel!(toggleTunnel:),
            toggleTunnel as extern "C" fn(&Object, Sel, id)
        );

        decl.add_method(
            sel!(applicationWillTerminate:),
            applicationWillTerminate as extern "C" fn(&Object, Sel, id)
        );

        decl.register()
    }
}

fn create_menu(handler: id) -> id {
    unsafe {
        let menu = NSMenu::new(nil).autorelease();

        // Create menu items
        let prod_item = create_menu_item(handler, "Open tunnel PROD", "prod");
        let dev_item = create_menu_item(handler, "Open tunnel DEV-01", "dev-01");

        let quit_title = NSString::alloc(nil).init_str("Quit");
        let quit_item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
            quit_title,
            sel!(terminate:),
            NSString::alloc(nil).init_str("q"),
        );

        menu.addItem_(prod_item);
        menu.addItem_(dev_item);
        menu.addItem_(quit_item);
        menu
    }
}

fn create_menu_item(handler: id, title: &str, command_id: &str) -> id {
    unsafe {
        let title = NSString::alloc(nil).init_str(title);
        let item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
            title,
            sel!(toggleTunnel:),
            NSString::alloc(nil).init_str(""),
        );

        let command_id = NSString::alloc(nil).init_str(command_id);
        let _: () = msg_send![item, setRepresentedObject:command_id];
        let _: () = msg_send![item, setTarget:handler];
        let _: () = msg_send![item, setState:NO];

        item
    }
}

fn create_status_item(handler: id) -> id {  // Modified to accept handler as parameter
    unsafe {
        let status_bar = NSStatusBar::systemStatusBar(nil);
        let status_item = status_bar.statusItemWithLength_(-1.0);
        let title = NSString::alloc(nil).init_str("☰");
        let button: id = msg_send![status_item, button];
        let _: () = msg_send![button, setTitle:title];
        status_item.setMenu_(create_menu(handler));
        status_item
    }
}

#[no_mangle]
extern "C" fn applicationWillTerminate(_: &Object, _: Sel, _notification: id) {
    info!("Application is terminating, cleaning up tunnels");
    cleanup_tunnels();
}

fn cleanup_tunnels() {
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
                            .output() {
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

fn main() {
    // Initialize the logger at the start of main
    oslog::OsLogger::new("com.1000ants.menubarapp")
        .level_filter(log::LevelFilter::Debug) // Set logging level
        .init()
        .unwrap();

    info!("Application starting up"); // This will show in Console.app
    let mut commands = HashMap::new();

    // Add PROD configuration
    commands.insert(
        "prod".to_string(),
        TunnelCommand {
            command: "ssh".to_string(),
            args: vec!["-N".to_string(), "lb-prod.rds".to_string()],
            kill_command: "pkill".to_string(),
            kill_args: vec!["-f".to_string(), "lb-prod.rds".to_string()],
        }
    );

    // Add DEV configuration
    commands.insert(
        "dev-01".to_string(),
        TunnelCommand {
            command: "ssh".to_string(),
            args: vec!["-N".to_string(), "lb-dev-01.rds".to_string()],
            kill_command: "pkill".to_string(),
            kill_args: vec!["-f".to_string(), "lb-dev-01.rds".to_string()],
        }
    );

    unsafe {
        COMMANDS_CONFIG = Some(Arc::new(Mutex::new(commands)));
        ACTIVE_TUNNELS = Some(Arc::new(Mutex::new(HashSet::new())));

        let _pool = NSAutoreleasePool::new(nil);
        let app = NSApplication::sharedApplication(nil);
        app.setActivationPolicy_(
            NSApplicationActivationPolicy::NSApplicationActivationPolicyAccessory,
        );

        // Create handler once
        let handler_class = register_selector();
        let handler: id = msg_send![handler_class, new];

        // Create status item with handler
        let _status_item = create_status_item(handler);

        // Register for termination notification
        let notification_center: id = msg_send![class!(NSNotificationCenter), defaultCenter];
        let _: () = msg_send![notification_center,
            addObserver:handler
            selector:sel!(applicationWillTerminate:)
            name:NSString::alloc(nil).init_str("NSApplicationWillTerminateNotification")
            object:nil];

        app.run();
    }
}
