use cocoa::appkit::{
    NSMenu, NSMenuItem, NSStatusBar, NSStatusItem,
};
use cocoa::base::{id, nil, NO};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use objc::{class, msg_send, sel, sel_impl};

use crate::tunnel::toggleTunnel;
use crate::applicationWillTerminate;

pub fn register_selector() -> *const Class {
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

pub fn create_menu(handler: id) -> id {
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

pub fn create_menu_item(handler: id, title: &str, command_id: &str) -> id {
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

pub fn create_status_item(handler: id) -> id {  // Modified to accept handler as parameter
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
