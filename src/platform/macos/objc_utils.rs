use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, Sel};
use objc2::{MainThreadOnly, msg_send};
use objc2_app_kit::{NSBezelStyle, NSButton, NSColor, NSFont, NSTextField, NSView};
use objc2_core_graphics::CGColor;
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize, NSString, ns_string};

pub fn rgba(r: f64, g: f64, b: f64, a: f64) -> Retained<NSColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, a)
}

pub fn gray(v: f64, a: f64) -> Retained<NSColor> {
    rgba(v, v, v, a)
}

pub unsafe fn set_layer_bg(view: &AnyObject, r: f64, g: f64, b: f64, a: f64) {
    let _: () = msg_send![view, setWantsLayer: true];
    let layer: *mut AnyObject = msg_send![view, layer];
    if layer.is_null() { return; }
    let bg = rgba(r, g, b, a);
    let cg: *const CGColor = msg_send![&*bg, CGColor];
    let _: () = msg_send![layer, setBackgroundColor: cg];
}

pub unsafe fn set_layer_corner(view: &AnyObject, radius: f64) {
    let layer: *mut AnyObject = msg_send![view, layer];
    if layer.is_null() { return; }
    let _: () = msg_send![layer, setCornerRadius: radius];
    let _: () = msg_send![layer, setMasksToBounds: true];
}

pub unsafe fn set_layer_border(view: &AnyObject, r: f64, g: f64, b: f64, a: f64, width: f64) {
    let layer: *mut AnyObject = msg_send![view, layer];
    if layer.is_null() { return; }
    let color = rgba(r, g, b, a);
    let cg: *const CGColor = msg_send![&*color, CGColor];
    let _: () = msg_send![layer, setBorderColor: cg];
    let _: () = msg_send![layer, setBorderWidth: width];
}

pub fn label(mtm: MainThreadMarker, size: f64, alpha: f64, wrap: bool) -> Retained<NSTextField> {
    let lbl = NSTextField::labelWithString(ns_string!(""), mtm);
    lbl.setBezeled(false);
    lbl.setDrawsBackground(false);
    lbl.setEditable(false);
    lbl.setSelectable(false);
    lbl.setTextColor(Some(&rgba(1.0, 1.0, 1.0, alpha)));
    lbl.setFont(Some(&NSFont::systemFontOfSize(size)));
    if !wrap {
        lbl.setLineBreakMode(objc2_app_kit::NSLineBreakMode::ByTruncatingTail);
        return lbl;
    }
    unsafe {
        let cell: *mut AnyObject = msg_send![&lbl, cell];
        if !cell.is_null() {
            let _: () = msg_send![cell, setWraps: true];
            let _: () = msg_send![cell, setTruncatesLastVisibleLine: true];
        }
        let _: () = msg_send![&lbl, setMaximumNumberOfLines: 0_isize];
    }
    lbl
}

pub fn button(mtm: MainThreadMarker, size: NSSize, transparent: bool) -> Retained<NSButton> {
    let btn = NSButton::init(NSButton::alloc(mtm));
    btn.setFrame(NSRect::new(NSPoint::ZERO, size));
    if transparent { btn.setTransparent(true); }
    btn.setBordered(false);
    btn.setTitle(ns_string!(""));
    btn
}

pub fn symbol_button(
    mtm: MainThreadMarker,
    size: f64,
    symbol: &str,
    symbol_size: f64,
    tint: &NSColor,
) -> Retained<NSButton> {
    let btn = button(mtm, NSSize::new(size, size), false);
    #[allow(deprecated)]
    btn.setBezelStyle(NSBezelStyle::Inline);
    set_symbol_image(&btn, symbol, symbol_size, tint);
    btn
}

pub fn set_symbol_image(btn: &NSButton, name: &str, point_size: f64, tint: &NSColor) {
    unsafe {
        let sym = NSString::from_str(name);
        let nil: *const AnyObject = std::ptr::null();
        let base: *mut AnyObject = msg_send![
            AnyClass::get(c"NSImage").unwrap(),
            imageWithSystemSymbolName: &*sym, accessibilityDescription: nil
        ];
        if base.is_null() { return; }
        let config: *mut AnyObject = msg_send![
            AnyClass::get(c"NSImageSymbolConfiguration").unwrap(),
            configurationWithPointSize: point_size, weight: 0.0_f64
        ];
        let image = if config.is_null() { base } else {
            let sized: *mut AnyObject = msg_send![base, imageWithSymbolConfiguration: config];
            if sized.is_null() { base } else { sized }
        };
        let _: () = msg_send![btn, setImage: image];
        let _: () = msg_send![btn, setContentTintColor: tint];
        let _: () = msg_send![btn, setImagePosition: 1_isize];
    }
}

pub unsafe fn wire_action(target: &AnyObject, view: &AnyObject, sel_name: &std::ffi::CStr, tag: isize) {
    let _: () = msg_send![view, setTag: tag];
    let _: () = msg_send![view, setTarget: target];
    let _: () = msg_send![view, setAction: Sel::register(sel_name)];
}

pub fn view_with_frame(mtm: MainThreadMarker, frame: NSRect) -> Retained<NSView> {
    NSView::initWithFrame(NSView::alloc(mtm), frame)
}

pub unsafe fn set_dark_appearance(view: *mut AnyObject) {
    let cls = AnyClass::get(c"NSAppearance").unwrap();
    let name = NSString::from_str("NSAppearanceNameVibrantDark");
    let dark: *const AnyObject = msg_send![cls, appearanceNamed: &*name];
    if dark.is_null() { return; }
    let _: () = msg_send![view, setAppearance: dark];
}

pub unsafe fn ns_app() -> *mut AnyObject {
    msg_send![AnyClass::get(c"NSApplication").unwrap(), sharedApplication]
}
