use cocoa::appkit::{NSApplication, NSApplicationActivationPolicy};
use cocoa::base::{id, nil};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use log::info;
use objc::runtime::{Object, Sel};
use objc::{class, msg_send, sel, sel_impl};
use oslog;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::Mutex;

mod menu;
mod tunnel;

use menu::{create_status_item, register_selector};
use tunnel::cleanup_tunnels;
use tunnel::TunnelCommand;
use tunnel::{ACTIVE_TUNNELS, COMMANDS_CONFIG};

#[no_mangle]
extern "C" fn applicationWillTerminate(_: &Object, _: Sel, _notification: id) {
    info!("Application is terminating, cleaning up tunnels");
    cleanup_tunnels();
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
        },
    );

    // Add DEV configuration
    commands.insert(
        "dev-01".to_string(),
        TunnelCommand {
            command: "ssh".to_string(),
            args: vec!["-N".to_string(), "lb-dev-01.rds".to_string()],
            kill_command: "pkill".to_string(),
            kill_args: vec!["-f".to_string(), "lb-dev-01.rds".to_string()],
        },
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
