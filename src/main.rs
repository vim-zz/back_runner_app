use cocoa::appkit::{
    NSApp, NSApplication, NSApplicationActivationPolicy, NSButton, NSMenu, NSMenuItem, NSStatusBar,
    NSStatusItem,
};
use cocoa::base::{id, nil, YES};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use objc::{msg_send, sel, sel_impl};
use std::ptr;

#[macro_use]
extern crate objc;

fn create_menu() -> id {
    unsafe {
        let menu = NSMenu::new(nil).autorelease();
        let title = NSString::alloc(nil).init_str("Quit");
        let quit_item = NSMenuItem::alloc(nil).initWithTitle_action_keyEquivalent_(
            title,
            sel!(terminate:),
            NSString::alloc(nil).init_str("q"),
        );
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
            NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular,
        );

        let _status_item = create_status_item();

        app.run();
    }
}
