use cocoa::appkit::{
    NSApplication, NSApplicationActivationPolicy, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem,
};
use objc::{class, msg_send, sel, sel_impl};
use objc::declare::ClassDecl;
use cocoa::base::{id, nil, YES, NO, BOOL};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use std::process::Command;
use objc::runtime::{Class, Object, Sel, self};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

static TUNNEL_PROCESS: AtomicBool = AtomicBool::new(false);

#[no_mangle]
extern "C" fn toggleTunnel(_: &Object, _: Sel, item: id) {
    println!("toggleTunnel called!");
    unsafe {
        let state: BOOL = msg_send![item, state];
        println!("Current state: {}", state == YES);

        // Toggle state
        let new_state = if state == YES { NO } else { YES };
        let _: () = msg_send![item, setState:new_state];

        if new_state == YES {
            println!("Starting tunnel...");
            TUNNEL_PROCESS.store(true, Ordering::SeqCst);
            thread::spawn(|| {
                while TUNNEL_PROCESS.load(Ordering::SeqCst) {
                    println!("Spawning ssh command...");
                    match Command::new("ssh")
                        .args(["-N", "lb-prod.rds"])
                        .spawn() {
                            Ok(mut child) => {
                                println!("SSH process started");
                                let _ = child.wait();
                            },
                            Err(e) => println!("Failed to start tunnel: {}", e),
                    }
                }
            });
        } else {
            println!("Stopping tunnel...");
            TUNNEL_PROCESS.store(false, Ordering::SeqCst);
            match Command::new("pkill")
                .args(["-f", "ssh -N lb-prod.rds"])
                .output() {
                    Ok(_) => println!("Tunnel stopped"),
                    Err(e) => println!("Failed to stop tunnel: {}", e),
            }
        }
    }
}

fn register_selector() -> *const Class {
    unsafe {
        let superclass = class!(NSObject);
        let mut decl = ClassDecl::new("MenuHandler", superclass).unwrap();

        // Add method
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

        let title = NSString::alloc(nil).init_str("Open tunnel PROD");
        let tunnel_item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
            title,
            sel!(toggleTunnel:),
            NSString::alloc(nil).init_str(""),
        );

        let _: () = msg_send![tunnel_item, setTarget:handler];
        let _: () = msg_send![tunnel_item, setState:NO];

        let quit_title = NSString::alloc(nil).init_str("Quit");
        let quit_item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
            quit_title,
            sel!(terminate:),
            NSString::alloc(nil).init_str("q"),
        );

        menu.addItem_(tunnel_item);
        menu.addItem_(quit_item);
        menu
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
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);
        let app = NSApplication::sharedApplication(nil);
        app.setActivationPolicy_(
            NSApplicationActivationPolicy::NSApplicationActivationPolicyAccessory,
        );
        let _status_item = create_status_item();
        app.run();
    }
}
