use std::ffi::c_void;
use std::ptr::NonNull;

use block2::RcBlock;
use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject, ProtocolObject};
use objc2_app_kit::NSEvent;
use objc2_foundation::NSObjectProtocol;

use super::controller;
use crate::hud::state::VISIBLE_CARD_COUNT;

#[repr(C)]
struct EventTypeSpec { event_class: u32, event_kind: u32 }
#[repr(C)]
#[derive(Clone, Copy)]
struct EventHotKeyID { signature: u32, id: u32 }

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    fn GetEventDispatcherTarget() -> *mut c_void;
    fn InstallEventHandler(t: *mut c_void, h: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> i32, n: u64, l: *const EventTypeSpec, ud: *mut c_void, o: *mut *mut c_void) -> i32;
    fn RegisterEventHotKey(code: u32, mods: u32, id: EventHotKeyID, target: *mut c_void, opts: u32, out: *mut *mut c_void) -> i32;
}

const KEYCODES: [u16; VISIBLE_CARD_COUNT] = [18, 19, 20, 21, 23, 22, 26, 28, 25];
const MASK_KEY_DOWN: u64 = 1 << 10;
const MASK_LMOUSE: u64 = 1 << 1;
const MASK_CMD: u64 = 1 << 20;

pub fn setup_event_monitors() -> Vec<Retained<ProtocolObject<dyn NSObjectProtocol>>> {
    register_hotkey();
    [mouse_monitor(), key_monitor()].into_iter().flatten().collect()
}

unsafe extern "C" fn hotkey_cb(_: *mut c_void, _: *mut c_void, _: *mut c_void) -> i32 { controller::toggle_panel(); 0 }

fn register_hotkey() {
    unsafe {
        let spec = EventTypeSpec { event_class: u32::from_be_bytes(*b"keyb"), event_kind: 5 };
        let target = GetEventDispatcherTarget();
        let mut hr: *mut c_void = std::ptr::null_mut();
        InstallEventHandler(target, hotkey_cb, 1, &spec, std::ptr::null_mut(), &mut hr);
        let mut hkr: *mut c_void = std::ptr::null_mut();
        RegisterEventHotKey(9, 0x0300, EventHotKeyID { signature: u32::from_be_bytes(*b"HST!"), id: 1 }, target, 0, &mut hkr);
    }
}

fn mouse_monitor() -> Option<Retained<ProtocolObject<dyn NSObjectProtocol>>> {
    let block = RcBlock::new(|_: NonNull<NSEvent>| { if controller::is_panel_visible() { controller::hide_panel(); } });
    let raw: *mut AnyObject = unsafe { msg_send![AnyClass::get(c"NSEvent")?, addGlobalMonitorForEventsMatchingMask: MASK_LMOUSE, handler: &*block] };
    if raw.is_null() { return None; }
    unsafe { Retained::retain(raw).map(|r| Retained::cast_unchecked(r)) }
}

fn key_monitor() -> Option<Retained<ProtocolObject<dyn NSObjectProtocol>>> {
    let block = RcBlock::new(|event: NonNull<NSEvent>| -> Option<NonNull<NSEvent>> {
        if handle_key(unsafe { event.as_ref() }) { None } else { Some(event) }
    });
    let raw: *mut AnyObject = unsafe { msg_send![AnyClass::get(c"NSEvent")?, addLocalMonitorForEventsMatchingMask: MASK_KEY_DOWN, handler: &*block] };
    if raw.is_null() { return None; }
    unsafe { Retained::retain(raw).map(|r| Retained::cast_unchecked(r)) }
}

fn handle_key(e: &NSEvent) -> bool {
    if !controller::is_panel_visible() { return false; }
    let code: u16 = unsafe { msg_send![e, keyCode] };
    if code == 53 { controller::hide_panel(); return true; }
    let flags: u64 = unsafe { msg_send![e, modifierFlags] };
    if flags & MASK_CMD == 0 { return false; }
    let Some(idx) = KEYCODES.iter().position(|&k| k == code) else { return false };
    controller::copy_item_at_display_index(idx);
    true
}
