use objc2::MainThreadMarker;
use objc2::ffi::NSInteger;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSEvent, NSScreen};
use std::ffi::{CString, c_uint, c_void};

type CGConnectionID = c_uint;

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(cf: *const c_void);
}

#[allow(improper_ctypes)]
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGSMainConnectionID() -> CGConnectionID;
    fn CGSCopyManagedDisplaySpaces(conn: CGConnectionID) -> *const AnyObject;
    fn CGSCopyActiveMenuBarDisplayIdentifier(conn: CGConnectionID) -> *const AnyObject;
}

#[derive(Clone, PartialEq)]
pub struct DisplayInfo {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub primary_height: f64,
    pub space_number: i32,
    pub total_spaces: i32,
}

pub fn active_display_info(mtm: MainThreadMarker) -> DisplayInfo {
    let mouse = NSEvent::mouseLocation();
    let screens = NSScreen::screens(mtm);

    let mut frame = None;
    for screen in screens.iter() {
        let f = screen.frame();
        if mouse.x >= f.origin.x
            && mouse.x < f.origin.x + f.size.width
            && mouse.y >= f.origin.y
            && mouse.y < f.origin.y + f.size.height
        {
            frame = Some(f);
            break;
        }
    }

    let primary = screens.objectAtIndex(0).frame();
    let f = frame.unwrap_or(primary);
    let (space_number, total_spaces) = query_space_info();

    DisplayInfo {
        x: f.origin.x,
        y: f.origin.y,
        width: f.size.width,
        primary_height: primary.size.height,
        space_number,
        total_spaces,
    }
}

fn query_space_info() -> (i32, i32) {
    unsafe {
        let conn = CGSMainConnectionID();
        let displays = CGSCopyManagedDisplaySpaces(conn);
        let active_id = CGSCopyActiveMenuBarDisplayIdentifier(conn);

        if displays.is_null() || active_id.is_null() {
            release_non_null(displays);
            release_non_null(active_id);
            return (1, 1);
        }

        let count: usize = objc2::msg_send![&*displays, count];
        let mut active_space_id: NSInteger = -1;
        let mut all_space_ids: Vec<NSInteger> = Vec::new();
        let cache = nsstring_cache();

        for i in 0..count {
            let entry: *const AnyObject = objc2::msg_send![&*displays, objectAtIndex: i];
            if entry.is_null() {
                continue;
            }

            let current: *const AnyObject =
                objc2::msg_send![&*entry, objectForKey: cache.current_space];
            let spaces: *const AnyObject = objc2::msg_send![&*entry, objectForKey: cache.spaces];
            let disp_ident: *const AnyObject =
                objc2::msg_send![&*entry, objectForKey: cache.display_identifier];

            if current.is_null() || spaces.is_null() || disp_ident.is_null() {
                continue;
            }

            let desc: *const AnyObject = objc2::msg_send![&*disp_ident, description];
            let is_main: bool = objc2::msg_send![&*desc, isEqualToString: cache.main];
            let is_active: bool = objc2::msg_send![&*desc, isEqualToString: &*active_id];

            if is_main || is_active {
                let managed: *const AnyObject =
                    objc2::msg_send![&*current, objectForKey: cache.managed_space_id];
                if !managed.is_null() {
                    active_space_id = objc2::msg_send![&*managed, integerValue];
                }
            }

            collect_space_ids(&mut all_space_ids, spaces, cache.managed_space_id);
        }

        CFRelease(displays.cast());
        CFRelease(active_id.cast());

        if active_space_id == -1 {
            return (1, 1);
        }

        let total = all_space_ids.len() as i32;
        for (idx, &sid) in all_space_ids.iter().enumerate() {
            if sid == active_space_id {
                return (idx as i32 + 1, total);
            }
        }

        (1, total.max(1))
    }
}

unsafe fn release_non_null(ptr: *const AnyObject) {
    if !ptr.is_null() {
        unsafe {
            CFRelease(ptr.cast());
        }
    }
}

unsafe fn collect_space_ids(
    ids: &mut Vec<NSInteger>,
    spaces: *const AnyObject,
    managed_key: *const AnyObject,
) {
    unsafe {
        let scount: usize = objc2::msg_send![&*spaces, count];
        for j in 0..scount {
            let space: *const AnyObject = objc2::msg_send![&*spaces, objectAtIndex: j];
            let managed: *const AnyObject = objc2::msg_send![&*space, objectForKey: managed_key];
            if managed.is_null() {
                ids.push(-1);
                continue;
            }
            ids.push(objc2::msg_send![&*managed, integerValue]);
        }
    }
}

struct NsStringCache {
    current_space: *const AnyObject,
    spaces: *const AnyObject,
    display_identifier: *const AnyObject,
    managed_space_id: *const AnyObject,
    main: *const AnyObject,
}

unsafe impl Send for NsStringCache {}
unsafe impl Sync for NsStringCache {}

fn nsstring_cache() -> &'static NsStringCache {
    use std::sync::OnceLock;
    static CACHE: OnceLock<NsStringCache> = OnceLock::new();
    CACHE.get_or_init(|| unsafe {
        NsStringCache {
            current_space: alloc_nsstring("Current Space"),
            spaces: alloc_nsstring("Spaces"),
            display_identifier: alloc_nsstring("Display Identifier"),
            managed_space_id: alloc_nsstring("ManagedSpaceID"),
            main: alloc_nsstring("Main"),
        }
    })
}

unsafe fn alloc_nsstring(s: &str) -> *const AnyObject {
    let cls = objc2::class!(NSString);
    let cstr = CString::new(s).unwrap();
    let raw: *const AnyObject = objc2::msg_send![cls, alloc];
    objc2::msg_send![raw, initWithUTF8String: cstr.as_ptr()]
}
