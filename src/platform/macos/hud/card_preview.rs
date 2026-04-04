use std::cell::RefCell;
use std::collections::VecDeque;
use std::ffi::c_void;

use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject};
use objc2::msg_send;
use objc2_app_kit::{NSColor, NSView};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize};

use crate::hud::state::{ClipboardItem, ItemKind};
use crate::platform::macos::objc_utils::{self, rgba};

const MAX_THUMBS: usize = 64;
const THUMB_MAX: f64 = 128.0;

thread_local! {
    static THUMBS: RefCell<VecDeque<(usize, *mut AnyObject)>> = const { RefCell::new(VecDeque::new()) };
}

pub fn create_image_view(mtm: MainThreadMarker) -> Retained<NSView> {
    let fallback = || objc_utils::view_with_frame(mtm, NSRect::new(NSPoint::ZERO, NSSize::new(100.0, 100.0)));
    unsafe {
        let Some(cls) = AnyClass::get(c"NSImageView") else { return fallback() };
        let alloc: *mut AnyObject = msg_send![cls, alloc];
        let inst: *mut AnyObject = msg_send![alloc, initWithFrame: NSRect::new(NSPoint::ZERO, NSSize::new(100.0, 100.0))];
        if inst.is_null() { return fallback(); }
        let _: () = msg_send![inst, setImageScaling: 3_usize];
        let _: () = msg_send![inst, setImageAlignment: 0_usize];
        let _: () = msg_send![inst, setHidden: true];
        Retained::retain(inst).map(|r| Retained::cast_unchecked(r)).unwrap_or_else(fallback)
    }
}

pub fn configure_preview(view: &NSView, item: &ClipboardItem) {
    match &item.kind {
        ItemKind::Color(hex) => set_color(view, hex),
        ItemKind::Image => { if let Some(t) = get_thumb(item) { unsafe { let _: () = msg_send![view, setImage: t]; } } }
        ItemKind::Text => {}
        _ => set_file_icon(view, &item.content),
    }
}

fn set_color(view: &NSView, hex: &str) {
    let Some(color) = parse_hex(hex) else { return };
    unsafe {
        let sz: NSSize = msg_send![view, frame];
        let sz = NSSize::new(sz.width, sz.height);
        let alloc: *mut AnyObject = msg_send![AnyClass::get(c"NSImage").unwrap(), alloc];
        let img: *mut AnyObject = msg_send![alloc, initWithSize: sz];
        if img.is_null() { return; }
        let _: () = msg_send![img, lockFocus];
        let _: () = msg_send![&*color, set];
        let bp: *mut AnyObject = msg_send![AnyClass::get(c"NSBezierPath").unwrap(), bezierPathWithRect: NSRect::new(NSPoint::ZERO, sz)];
        let _: () = msg_send![bp, fill];
        let _: () = msg_send![img, unlockFocus];
        let _: () = msg_send![view, setImage: img];
    }
}

fn set_file_icon(view: &NSView, path: &str) {
    if path.is_empty() { return; }
    unsafe {
        let ws: *mut AnyObject = msg_send![AnyClass::get(c"NSWorkspace").unwrap(), sharedWorkspace];
        let ns = objc2_foundation::NSString::from_str(path);
        let icon: *mut AnyObject = msg_send![ws, iconForFile: &*ns];
        if !icon.is_null() { let _: () = msg_send![view, setImage: icon]; }
    }
}

fn get_thumb(item: &ClipboardItem) -> Option<*mut AnyObject> {
    let cached = THUMBS.with(|c| c.borrow().iter().find(|(id, _)| *id == item.id).map(|(_, img)| *img));
    if cached.is_some() { return cached; }
    let src = load_source(item)?;
    let thumb = scale(src);
    THUMBS.with(|c| { let mut q = c.borrow_mut(); q.push_back((item.id, thumb)); while q.len() > MAX_THUMBS { q.pop_front(); } });
    Some(thumb)
}

fn load_source(item: &ClipboardItem) -> Option<*mut AnyObject> {
    unsafe {
        if let Some(ref blob) = item.blob_path {
            let data = std::fs::read(blob).ok()?;
            let ns: *mut AnyObject = msg_send![AnyClass::get(c"NSData").unwrap(), dataWithBytes: data.as_ptr() as *const c_void, length: data.len()];
            if ns.is_null() { return None; }
            let alloc: *mut AnyObject = msg_send![AnyClass::get(c"NSImage").unwrap(), alloc];
            let img: *mut AnyObject = msg_send![alloc, initWithData: ns];
            return if img.is_null() { None } else { Some(img) };
        }
        let ns = objc2_foundation::NSString::from_str(&item.content);
        let alloc: *mut AnyObject = msg_send![AnyClass::get(c"NSImage").unwrap(), alloc];
        let img: *mut AnyObject = msg_send![alloc, initWithContentsOfFile: &*ns];
        if img.is_null() { None } else { Some(img) }
    }
}

fn scale(src: *mut AnyObject) -> *mut AnyObject {
    unsafe {
        let ss: NSSize = msg_send![&*src, size];
        let s = (THUMB_MAX / ss.width.max(1.0)).min(THUMB_MAX / ss.height.max(1.0)).min(1.0);
        let ts = NSSize::new(ss.width * s, ss.height * s);
        let alloc: *mut AnyObject = msg_send![AnyClass::get(c"NSImage").unwrap(), alloc];
        let t: *mut AnyObject = msg_send![alloc, initWithSize: ts];
        let _: () = msg_send![t, lockFocus];
        let _: () = msg_send![src, drawInRect: NSRect::new(NSPoint::ZERO, ts)];
        let _: () = msg_send![t, unlockFocus];
        t
    }
}

fn parse_hex(hex: &str) -> Option<Retained<NSColor>> {
    let h = hex.trim().trim_start_matches('#');
    if h.len() != 6 && h.len() != 8 { return None; }
    let v = u32::from_str_radix(h, 16).ok()?;
    let (r, g, b, a) = if h.len() == 6 {
        (((v >> 16) & 0xFF) as f64 / 255.0, ((v >> 8) & 0xFF) as f64 / 255.0, (v & 0xFF) as f64 / 255.0, 1.0)
    } else {
        (((v >> 24) & 0xFF) as f64 / 255.0, ((v >> 16) & 0xFF) as f64 / 255.0, ((v >> 8) & 0xFF) as f64 / 255.0, (v & 0xFF) as f64 / 255.0)
    };
    Some(rgba(r, g, b, a))
}
