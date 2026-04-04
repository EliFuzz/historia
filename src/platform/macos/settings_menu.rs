use objc2::msg_send;
use objc2::runtime::{AnyClass, AnyObject, Sel};
use objc2_foundation::{NSPoint, NSString};

use super::hud::controller::action_target;
use crate::hud::settings::{get, ItemsLimit, RetentionPeriod};

pub fn show_menu(button: &AnyObject) {
    let s = get();
    unsafe {
        let menu: *mut AnyObject = msg_send![AnyClass::get(c"NSMenu").unwrap(), new];
        submenu(menu, "Retention Period", RetentionPeriod::ALL, c"handleRetention:", |p| s.retention_period == *p, |p| p.label());
        submenu(menu, "Items Limit", ItemsLimit::ALL, c"handleItemsLimit:", |l| s.items_limit == *l, |l| l.label());
        let sep: *mut AnyObject = msg_send![AnyClass::get(c"NSMenuItem").unwrap(), separatorItem];
        let _: () = msg_send![menu, addItem: sep];
        let _: () = msg_send![menu, addItem: menu_item("Quit", c"handleQuit:", 0)];
        let _: bool = msg_send![menu, popUpMenuPositioningItem: std::ptr::null::<AnyObject>(), atLocation: NSPoint::ZERO, inView: button];
    }
}

unsafe fn submenu<T>(menu: *mut AnyObject, title: &str, items: &[T], sel: &std::ffi::CStr, active: impl Fn(&T) -> bool, lbl: impl Fn(&T) -> &str) {
    let parent: *mut AnyObject = msg_send![AnyClass::get(c"NSMenuItem").unwrap(), new];
    let _: () = msg_send![parent, setTitle: &*NSString::from_str(title)];
    let sub: *mut AnyObject = msg_send![AnyClass::get(c"NSMenu").unwrap(), new];
    for (i, item) in items.iter().enumerate() {
        let mi = unsafe { menu_item(lbl(item), sel, i as isize) };
        let _: () = msg_send![mi, setState: if active(item) { 1_isize } else { 0_isize }];
        let _: () = msg_send![sub, addItem: mi];
    }
    let _: () = msg_send![parent, setSubmenu: sub];
    let _: () = msg_send![menu, addItem: parent];
}

unsafe fn menu_item(label: &str, sel: &std::ffi::CStr, tag: isize) -> *mut AnyObject {
    let mi_alloc: *mut AnyObject = msg_send![AnyClass::get(c"NSMenuItem").unwrap(), alloc];
    let mi: *mut AnyObject = msg_send![mi_alloc, initWithTitle: &*NSString::from_str(label), action: Sel::register(sel), keyEquivalent: &*NSString::from_str("")];
    let _: () = msg_send![mi, setTag: tag];
    if let Some(t) = action_target() { let _: () = msg_send![mi, setTarget: &*t]; }
    mi
}
