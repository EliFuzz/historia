use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::msg_send;
use objc2_app_kit::{NSButton, NSFont, NSTextField, NSView};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize};

use objc2::MainThreadOnly;
use crate::platform::macos::objc_utils::{self, gray, rgba, symbol_button, view_with_frame, wire_action};

const SEARCH_H: f64 = 28.0;
const BAR_PAD: f64 = 8.0;
const BTN_SIZE: f64 = 28.0;

pub fn create_top_bar(mtm: MainThreadMarker, width: f64, y: f64) -> Retained<NSView> {
    let bar = view_with_frame(mtm, NSRect::new(NSPoint::new(0.0, y), NSSize::new(width, SEARCH_H + BAR_PAD * 2.0)));
    let (sc, sf) = search_field(mtm, width);
    let del = toolbar_btn(mtm, width, 2, "trash", 13.0, c"deleteAllItems:");
    let set = toolbar_btn(mtm, width, 1, "gearshape", 14.0, c"showSettingsMenu:");
    super::controller::set_search_field(sf);
    bar.addSubview(&sc); bar.addSubview(&del); bar.addSubview(&set);
    bar
}

pub fn top_bar_height() -> f64 { SEARCH_H + BAR_PAD * 2.0 }

fn search_field(mtm: MainThreadMarker, w: f64) -> (Retained<NSView>, Retained<NSTextField>) {
    let fw = w - BAR_PAD * 3.0 - BTN_SIZE * 2.0 - BAR_PAD;
    let c = view_with_frame(mtm, NSRect::new(NSPoint::new(BAR_PAD, BAR_PAD), NSSize::new(fw, SEARCH_H)));
    unsafe {
        objc_utils::set_layer_bg(&c, 1.0, 1.0, 1.0, 0.08);
        objc_utils::set_layer_corner(&c, 8.0);
    }
    let fh = 18.0;
    let tf = NSTextField::init(NSTextField::alloc(mtm));
    tf.setFrame(NSRect::new(NSPoint::new(10.0, (SEARCH_H - fh) / 2.0), NSSize::new(fw - 20.0, fh)));
    tf.setBezeled(false); tf.setDrawsBackground(false);
    tf.setTextColor(Some(&rgba(0.90, 0.90, 0.90, 1.0)));
    tf.setFont(Some(&NSFont::systemFontOfSize(13.0)));
    tf.setFocusRingType(objc2_app_kit::NSFocusRingType::None);
    unsafe {
        let _: () = msg_send![&tf, setAutomaticTextCompletionEnabled: false];
        let null: *const AnyObject = std::ptr::null();
        let _: () = msg_send![&tf, setContentType: null];
    }
    set_placeholder(&tf);
    if let Some(d) = super::controller::search_delegate_ref() { unsafe { let _: () = msg_send![&tf, setDelegate: &*d]; } }
    c.addSubview(&tf);
    (c, tf)
}

fn set_placeholder(tf: &NSTextField) {
    unsafe {
        let text = objc2_foundation::NSString::from_str("Search");
        let dict: *mut AnyObject = msg_send![objc2::runtime::AnyClass::get(c"NSMutableDictionary").unwrap(), new];
        let _: () = msg_send![dict, setObject: &*NSFont::systemFontOfSize(13.0), forKey: objc2_app_kit::NSFontAttributeName];
        let _: () = msg_send![dict, setObject: &*rgba(1.0, 1.0, 1.0, 0.45), forKey: objc2_app_kit::NSForegroundColorAttributeName];
        let attr_alloc: *mut AnyObject = msg_send![objc2::runtime::AnyClass::get(c"NSAttributedString").unwrap(), alloc];
        let attr: *mut AnyObject = msg_send![attr_alloc, initWithString: &*text, attributes: dict];
        if attr.is_null() { return; }
        let cell: *mut AnyObject = msg_send![tf, cell];
        if cell.is_null() { return; }
        let _: () = msg_send![cell, setPlaceholderAttributedString: attr];
    }
}

fn toolbar_btn(mtm: MainThreadMarker, w: f64, pos: usize, sym: &str, sz: f64, sel: &std::ffi::CStr) -> Retained<NSButton> {
    let btn = symbol_button(mtm, BTN_SIZE, sym, sz, &gray(0.63, 0.78));
    let x = w - BAR_PAD - BTN_SIZE * pos as f64 - BAR_PAD * (pos as f64 - 1.0);
    btn.setFrame(NSRect::new(NSPoint::new(x, BAR_PAD), NSSize::new(BTN_SIZE, BTN_SIZE)));
    let Some(target) = super::controller::action_target() else { return btn; };
    unsafe { wire_action(&target, &btn, sel, 0); }
    btn
}
