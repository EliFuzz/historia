use std::cell::RefCell;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{define_class, msg_send, DefinedClass, MainThreadOnly};
use objc2_app_kit::{NSCollectionViewItem, NSView};
use objc2_foundation::{MainThreadMarker, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize};

use super::card_preview;
use crate::hud::state::{ClipboardItem, VISIBLE_CARD_COUNT};
use crate::platform::macos::objc_utils::{self, label, button, rgba};

const PAD: f64 = 10.0;
const FOOTER_H: f64 = 16.0;
const FOOTER_PAD: f64 = 6.0;
const DEL_SIZE: f64 = 22.0;
const DEL_MARGIN: f64 = 3.0;

pub(crate) struct CardIvars {
    content: RefCell<Option<Retained<objc2_app_kit::NSTextField>>>,
    footer: RefCell<Option<Retained<objc2_app_kit::NSTextField>>>,
    shortcut: RefCell<Option<Retained<objc2_app_kit::NSTextField>>>,
    image: RefCell<Option<Retained<NSView>>>,
    del_btn: RefCell<Option<Retained<objc2_app_kit::NSButton>>>,
    click_btn: RefCell<Option<Retained<objc2_app_kit::NSButton>>>,
}

impl Default for CardIvars {
    fn default() -> Self {
        Self { content: RefCell::new(None), footer: RefCell::new(None), shortcut: RefCell::new(None), image: RefCell::new(None), del_btn: RefCell::new(None), click_btn: RefCell::new(None) }
    }
}

define_class!(
    #[unsafe(super(NSCollectionViewItem, objc2_app_kit::NSViewController, objc2_app_kit::NSResponder, NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = CardIvars]
    #[name = "HSTCardItem"]
    pub(crate) struct CardItem;
    unsafe impl NSObjectProtocol for CardItem {}
    impl CardItem {
        #[unsafe(method_id(init))]
        fn init(this: objc2::rc::Allocated<Self>) -> Retained<Self> {
            let this = this.set_ivars(CardIvars::default());
            unsafe { msg_send![super(this), init] }
        }
        #[unsafe(method_id(initWithNibName:bundle:))]
        fn init_nib(this: objc2::rc::Allocated<Self>, nib: Option<&objc2_foundation::NSString>, bundle: Option<&NSObject>) -> Retained<Self> {
            let this = this.set_ivars(CardIvars::default());
            unsafe { msg_send![super(this), initWithNibName: nib, bundle: bundle] }
        }
        #[unsafe(method(loadView))]
        fn load_view(&self) {
            let mtm = self.mtm();
            let container = create_container(mtm);
            let c = label(mtm, 11.0, 0.86, true);
            let f = label(mtm, 9.5, 0.55, false);
            let s = label(mtm, 9.5, 0.55, false);
            s.setAlignment(objc2_app_kit::NSTextAlignment::Right);
            let iv = card_preview::create_image_view(mtm);
            let cb = button(mtm, NSSize::new(100.0, 100.0), true);
            let db = create_del_btn(mtm);
            for v in [&*c as &NSView, &*f, &*s, &iv, &*cb as &NSView, &*db as &NSView] { container.addSubview(v); }
            let ivars = self.ivars();
            *ivars.content.borrow_mut() = Some(c); *ivars.footer.borrow_mut() = Some(f);
            *ivars.shortcut.borrow_mut() = Some(s); *ivars.image.borrow_mut() = Some(iv);
            *ivars.del_btn.borrow_mut() = Some(db); *ivars.click_btn.borrow_mut() = Some(cb);
            unsafe { let _: () = msg_send![self, setView: &*container]; }
        }
    }
);

impl CardItem {
    pub fn configure(&self, item: &ClipboardItem, index: usize, sz: NSSize) {
        let iv = self.ivars();
        self.view().setFrameSize(sz);
        unsafe { objc_utils::set_layer_corner(&*self.view(), 8.0); }
        let cw = sz.width - PAD * 2.0;
        let ch = sz.height - PAD - FOOTER_PAD - FOOTER_H - 4.0;
        let cf = NSRect::new(NSPoint::new(PAD, FOOTER_H + 4.0 + FOOTER_PAD), NSSize::new(cw, ch));
        let is_text = item.kind.is_text();
        if let Some(lbl) = iv.content.borrow().as_ref() {
            lbl.setFrame(cf); lbl.setHidden(!is_text);
            if is_text {
                let lines: String = item.content.lines().take(6).collect::<Vec<_>>().join("\n");
                let preview = if lines.len() > 200 { format!("{}...", &lines[..197]) } else { lines };
                lbl.setStringValue(&objc2_foundation::NSString::from_str(&preview));
            }
        }
        if let Some(lbl) = iv.footer.borrow().as_ref() {
            lbl.setFrame(NSRect::new(NSPoint::new(PAD, FOOTER_PAD), NSSize::new(cw * 0.7, FOOTER_H)));
            lbl.setStringValue(&objc2_foundation::NSString::from_str(&item.display_name()));
        }
        if let Some(lbl) = iv.shortcut.borrow().as_ref() {
            lbl.setFrame(NSRect::new(NSPoint::new(sz.width - PAD - cw * 0.3, FOOTER_PAD), NSSize::new(cw * 0.3, FOOTER_H)));
            let txt = if index < VISIBLE_CARD_COUNT { format!("\u{2318}{}", index + 1) } else { String::new() };
            lbl.setStringValue(&objc2_foundation::NSString::from_str(&txt));
        }
        if let Some(v) = iv.image.borrow().as_ref() {
            v.setFrame(cf); v.setHidden(is_text);
            if is_text { unsafe { let _: () = msg_send![&**v, setImage: std::ptr::null::<AnyObject>()]; } }
            else { card_preview::configure_preview(v, item); }
        }
        wire_btn(iv.del_btn.borrow().as_ref(), item.id as isize, NSRect::new(NSPoint::new(sz.width - DEL_SIZE - DEL_MARGIN, sz.height - DEL_SIZE - DEL_MARGIN), NSSize::new(DEL_SIZE, DEL_SIZE)), c"deleteItem:");
        wire_btn(iv.click_btn.borrow().as_ref(), index as isize, NSRect::new(NSPoint::ZERO, sz), c"selectItem:");
    }
}

fn wire_btn(btn: Option<&Retained<objc2_app_kit::NSButton>>, tag: isize, frame: NSRect, sel: &std::ffi::CStr) {
    let Some(btn) = btn else { return };
    btn.setFrame(frame);
    let Some(target) = super::controller::action_target() else { return };
    unsafe { objc_utils::wire_action(&target, btn, sel, tag); }
}

fn create_container(mtm: MainThreadMarker) -> Retained<NSView> {
    let v = objc_utils::view_with_frame(mtm, NSRect::new(NSPoint::ZERO, NSSize::new(100.0, 100.0)));
    unsafe {
        objc_utils::set_layer_bg(&v, 30.0 / 255.0, 30.0 / 255.0, 30.0 / 255.0, 0.78);
        objc_utils::set_layer_corner(&v, 8.0);
        objc_utils::set_layer_border(&v, 60.0 / 255.0, 60.0 / 255.0, 60.0 / 255.0, 0.47, 0.5);
    }
    v
}

fn create_del_btn(mtm: MainThreadMarker) -> Retained<objc2_app_kit::NSButton> {
    let btn = objc_utils::symbol_button(mtm, DEL_SIZE, "xmark", 10.0, &rgba(1.0, 1.0, 1.0, 0.80));
    unsafe { objc_utils::set_layer_bg(&btn, 0.0, 0.0, 0.0, 0.50); objc_utils::set_layer_corner(&btn, DEL_SIZE / 2.0); }
    btn
}
