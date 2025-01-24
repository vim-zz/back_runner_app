use cocoa::appkit::{
    NSApplication, NSApplicationActivationPolicy, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem,
};
use cocoa::base::{id, nil, BOOL, NO, YES};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use objc::declare::ClassDecl;
use objc::runtime::{self, Class, Object, Sel};
use objc::{class, msg_send, sel, sel_impl};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::collections::HashMap;

#[derive(Clone)]
struct TunnelCommand {
    command: String,
    args: Vec<String>,
    kill_command: String,
    kill_args: Vec<String>,
}

static TUNNEL_PROCESS: AtomicBool = AtomicBool::new(false);
static mut COMMANDS_CONFIG: Option<Arc<Mutex<HashMap<String, TunnelCommand>>>> = None;
#[no_mangle]
extern "C" fn toggleTunnel(_: &Object, _: Sel, item: id) {
    println!("toggleTunnel called!");
    unsafe {
        let state: BOOL = msg_send![item, state];
        println!("Current state: {}", state == YES);

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

        if new_state == YES {
            println!("Starting command for {}", command_key);
            TUNNEL_PROCESS.store(true, Ordering::SeqCst);
            thread::spawn(move || {
                while TUNNEL_PROCESS.load(Ordering::SeqCst) {
                    let command = {
                        let config = commands_config.lock().unwrap();
                        config.get(&command_key).unwrap().clone()
                    };
                    println!("Spawning command: {} {:?}", command.command, command.args);

                    // Blocking call
                    match Command::new(&command.command)
                        .args(&command.args)
                        .spawn() {
                            Ok(mut child) => {
                                println!("Process started");
                                let _ = child.wait();
                            },
                            Err(e) => println!("Failed to start command: {e}"),
                    }
                    println!("Done");
                }
            });
        } else {
            TUNNEL_PROCESS.store(false, Ordering::SeqCst);
            let command = {
                let config = commands_config.lock().unwrap();
                config.get(&command_key).unwrap().clone()
            };
            println!("Stopping command: {} {:?}", command.command, command.args);

            match Command::new(&command.kill_command)
                .args(&command.kill_args)
                .output() {
                    Ok(_) => println!("Process stopped"),
                    Err(e) => println!("Failed to stop process: {e}"),
            }
            println!("Done");
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

        decl.register()
    }
}

fn create_menu() -> id {
    unsafe {
        let menu = NSMenu::new(nil).autorelease();

        let handler_class = register_selector();
        let handler: id = msg_send![handler_class, new];

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

fn create_status_item() -> id {
    unsafe {
        let status_bar = NSStatusBar::systemStatusBar(nil);
        let status_item = status_bar.statusItemWithLength_(-1.0);
        let title = NSString::alloc(nil).init_str("☰");
        let button: id = msg_send![status_item, button];
        let _: () = msg_send![button, setTitle:title];
        status_item.setMenu_(create_menu());
        status_item
    }
}

fn main() {
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

        let _pool = NSAutoreleasePool::new(nil);
        let app = NSApplication::sharedApplication(nil);
        app.setActivationPolicy_(
            NSApplicationActivationPolicy::NSApplicationActivationPolicyAccessory,
        );
        let _status_item = create_status_item();
        app.run();
    }
}
