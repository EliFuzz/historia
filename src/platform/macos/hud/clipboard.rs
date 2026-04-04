use std::cell::Cell;
use std::ffi::c_void;

use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, Sel};
use objc2::{DefinedClass, MainThreadOnly, define_class, msg_send};
use objc2_foundation::{MainThreadMarker, NSObject, NSObjectProtocol, NSString};

use crate::hud::state::{ItemKind, UTI_FILE_URL, UTI_PLAIN_TEXT, UTI_PNG, UTI_TIFF, format_bytes};

pub(crate) struct MonitorIvars { change_count: Cell<i64> }

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = MonitorIvars]
    #[name = "HSTClipboardMonitor"]
    pub(crate) struct ClipboardMonitor;
    unsafe impl NSObjectProtocol for ClipboardMonitor {}
    impl ClipboardMonitor {
        #[unsafe(method(tick:))]
        fn tick(&self, _: &AnyObject) {
            unsafe {
                let pb = pasteboard();
                let count: i64 = msg_send![pb, changeCount];
                if count == self.ivars().change_count.get() { return; }
                self.ivars().change_count.set(count);
                if super::controller::should_skip_capture() { return; }
                capture(pb);
            }
        }
    }
);

impl ClipboardMonitor {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let cc: i64 = unsafe { msg_send![pasteboard(), changeCount] };
        let this = Self::alloc(mtm).set_ivars(MonitorIvars { change_count: Cell::new(cc) });
        unsafe { msg_send![super(this), init] }
    }
}

pub fn start_monitoring(mtm: MainThreadMarker) -> Retained<ClipboardMonitor> {
    let m = ClipboardMonitor::new(mtm);
    unsafe {
        let _: *mut AnyObject = msg_send![
            AnyClass::get(c"NSTimer").unwrap(),
            scheduledTimerWithTimeInterval: 0.5_f64, target: &*m,
            selector: Sel::register(c"tick:"), userInfo: std::ptr::null::<AnyObject>(), repeats: true
        ];
    }
    m
}

unsafe fn pasteboard() -> *mut AnyObject { msg_send![AnyClass::get(c"NSPasteboard").unwrap(), generalPasteboard] }

fn capture(pb: *mut AnyObject) {
    unsafe {
        let types: *mut AnyObject = msg_send![pb, types];
        if types.is_null() { return; }
        let has = |u: &str| -> bool { let ns = NSString::from_str(u); msg_send![types, containsObject: &*ns] };
        let app = app_name();
        if has(UTI_FILE_URL) { return capture_file(pb, &app); }
        if has(UTI_TIFF) || has(UTI_PNG) { return capture_image(pb, has(UTI_PNG), &app); }
        if has(UTI_PLAIN_TEXT) { capture_text(pb, &app); }
    }
}

unsafe fn capture_file(pb: *mut AnyObject, app: &str) {
    unsafe {
        let ns = NSString::from_str(UTI_FILE_URL);
        let url: *mut AnyObject = msg_send![pb, stringForType: &*ns];
        if url.is_null() { return; }
        let dec: *mut AnyObject = msg_send![&*(url as *const NSString), stringByRemovingPercentEncoding];
        let s: &NSString = if dec.is_null() { &*(url as *const NSString) } else { &*(dec as *const NSString) };
        let raw = s.to_string();
        let path = raw.strip_prefix("file://").unwrap_or(&raw).to_string();
        let name = path.rsplit('/').find(|s| !s.is_empty()).unwrap_or(&path);
        let kind = ItemKind::from_filename(name);
        super::controller::add_clipboard_item(path, app.to_owned(), kind, None);
    }
}

unsafe fn capture_image(pb: *mut AnyObject, has_png: bool, app: &str) {
    unsafe {
        let uti = if has_png { UTI_PNG } else { UTI_TIFF };
        let ns = NSString::from_str(uti);
        let data: *mut AnyObject = msg_send![pb, dataForType: &*ns];
        if data.is_null() { return; }
        let len: usize = msg_send![data, length];
        if len == 0 { return; }
        let ptr: *const c_void = msg_send![data, bytes];
        if ptr.is_null() { return; }
        let bytes = std::slice::from_raw_parts(ptr as *const u8, len).to_vec();
        super::controller::add_clipboard_item(format!("Image ({})", format_bytes(len)), app.to_owned(), ItemKind::Image, Some((uti.into(), bytes)));
    }
}

unsafe fn capture_text(pb: *mut AnyObject, app: &str) {
    unsafe {
        let ns = NSString::from_str(UTI_PLAIN_TEXT);
        let text: *mut AnyObject = msg_send![pb, stringForType: &*ns];
        if text.is_null() { return; }
        let content = (&*(text as *const NSString)).to_string();
        if content.is_empty() { return; }
        let kind = ItemKind::from_text(&content);
        super::controller::add_clipboard_item(content, app.to_owned(), kind, None);
    }
}

fn app_name() -> String {
    unsafe {
        let ws: *mut AnyObject = msg_send![AnyClass::get(c"NSWorkspace").unwrap(), sharedWorkspace];
        let app: *mut AnyObject = msg_send![ws, frontmostApplication];
        if app.is_null() { return "Unknown".into(); }
        let name: *mut AnyObject = msg_send![app, localizedName];
        if name.is_null() { return "Unknown".into(); }
        (&*(name as *const NSString)).to_string()
    }
}
