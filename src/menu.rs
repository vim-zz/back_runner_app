// src/menu.rs
//
// Responsible for creating the NSStatusItem and NSMenu, plus the Objective-C class
// that receives menu events. We keep the function references the same, but route
// the logic to `toggleTunnel` in `tunnel.rs`.

use cocoa::appkit::{NSMenu, NSMenuItem, NSStatusBar, NSStatusItem};
use cocoa::base::{id, nil, NO};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use objc::{class, msg_send, sel, sel_impl};

use crate::{applicationWillTerminate, tunnel::toggleTunnel};

/// Registers our Objective-C class, `MenuHandler`, with the selectors
/// for toggling tunnels and handling app termination.
pub fn register_selector() -> *const Class {
    unsafe {
        let superclass = class!(NSObject);
        let mut decl = ClassDecl::new("MenuHandler", superclass).unwrap();

        // Link the "toggleTunnel:" selector to our Rust function
        decl.add_method(
            sel!(toggleTunnel:),
            toggleTunnel as extern "C" fn(&Object, Sel, id),
        );

        // Link the "applicationWillTerminate:" selector
        decl.add_method(
            sel!(applicationWillTerminate:),
            applicationWillTerminate as extern "C" fn(&Object, Sel, id),
        );

        decl.register()
    }
}

/// Create the NSMenu for the status item.
pub fn create_menu(handler: id) -> id {
    unsafe {
        let menu = NSMenu::new(nil).autorelease();

        // Menu items
        let prod_item = create_menu_item(handler, "Open tunnel PROD", "prod");
        let dev_item = create_menu_item(handler, "Open tunnel DEV-01", "dev-01");

        // Quit menu item
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

/// Helper to create a single NSMenuItem for toggling a tunnel.
fn create_menu_item(handler: id, title: &str, command_id: &str) -> id {
    unsafe {
        let title_ns = NSString::alloc(nil).init_str(title);
        let item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
            title_ns,
            sel!(toggleTunnel:),
            NSString::alloc(nil).init_str(""),
        );

        let command_id_ns = NSString::alloc(nil).init_str(command_id);
        let _: () = msg_send![item, setRepresentedObject: command_id_ns];
        let _: () = msg_send![item, setTarget: handler];
        let _: () = msg_send![item, setState: NO];

        item
    }
}

/// Creates a status bar item and attaches the menu to it.
pub fn create_status_item(handler: id) -> id {
    unsafe {
        let status_bar = NSStatusBar::systemStatusBar(nil);
        let status_item = status_bar.statusItemWithLength_(-1.0);
        let title = NSString::alloc(nil).init_str("☰");
        let button: id = msg_send![status_item, button];
        let _: () = msg_send![button, setTitle: title];

        status_item.setMenu_(create_menu(handler));
        status_item
    }
}
