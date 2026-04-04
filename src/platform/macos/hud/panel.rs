use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSBackingStoreType, NSCollectionView, NSColor, NSPanel, NSResponder,
    NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView,
    NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_foundation::{MainThreadMarker, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize};

use super::collection::{card_height, create_scroll_view};
use super::search::{create_top_bar, top_bar_height};
use crate::platform::macos::{DisplayInfo, active_display_info, objc_utils};

const PAD: f64 = 4.0;

define_class!(
    #[unsafe(super(NSPanel, NSWindow, NSResponder, NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    #[name = "HSTPanel"]
    pub(crate) struct KeyPanel;
    unsafe impl NSObjectProtocol for KeyPanel {}
    impl KeyPanel {
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key(&self) -> bool { true }
    }
);

pub fn create_hud_panel(mtm: MainThreadMarker) -> (Retained<NSPanel>, Retained<super::collection::DataSource>, Retained<NSCollectionView>) {
    let info = active_display_info(mtm);
    let sz = panel_size(&info);
    let panel: Retained<NSPanel> = {
        let this = KeyPanel::alloc(mtm).set_ivars(());
        let mask = NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel | NSWindowStyleMask::HUDWindow;
        let kp: Retained<KeyPanel> = unsafe { msg_send![super(this), initWithContentRect: NSRect::new(panel_origin(&info), sz), styleMask: mask, backing: NSBackingStoreType::Buffered, defer: false] };
        Retained::into_super(kp)
    };
    configure(&panel);
    let cv = panel.contentView().expect("content view");
    cv.addSubview(&glass_bg(mtm, cv.frame()));
    let ch = card_height(info.width);
    cv.addSubview(&create_top_bar(mtm, info.width, PAD + ch + PAD));
    let (sv, ds, colv) = create_scroll_view(mtm, NSRect::new(NSPoint::ZERO, NSSize::new(info.width, ch + PAD * 2.0)));
    cv.addSubview(&sv);
    (panel, ds, colv)
}

pub fn reposition_panel(panel: &NSPanel, mtm: MainThreadMarker) {
    let info = active_display_info(mtm);
    panel.setFrame_display(NSRect::new(panel_origin(&info), panel_size(&info)), true);
}

fn configure(panel: &NSPanel) {
    unsafe {
        let _: () = msg_send![panel, setReleasedWhenClosed: false];
        for t in 0..3_usize { let b: *mut AnyObject = msg_send![panel, standardWindowButton: t]; if !b.is_null() { let _: () = msg_send![b, setHidden: true]; } }
    }
    panel.setLevel(101);
    panel.setCollectionBehavior(NSWindowCollectionBehavior::CanJoinAllSpaces | NSWindowCollectionBehavior::Stationary | NSWindowCollectionBehavior::FullScreenAuxiliary | NSWindowCollectionBehavior::IgnoresCycle);
    panel.setOpaque(false); panel.setHasShadow(false); panel.setBackgroundColor(Some(&NSColor::clearColor()));
    unsafe { let _: () = msg_send![panel, setTitlebarAppearsTransparent: true]; }
}

fn panel_size(info: &DisplayInfo) -> NSSize { NSSize::new(info.width, top_bar_height() + card_height(info.width) + PAD * 3.0) }
fn panel_origin(info: &DisplayInfo) -> NSPoint { NSPoint::new(info.x, info.y) }

fn glass_bg(mtm: MainThreadMarker, frame: NSRect) -> Retained<objc2_app_kit::NSView> {
    let resize = objc2_app_kit::NSAutoresizingMaskOptions::ViewWidthSizable | objc2_app_kit::NSAutoresizingMaskOptions::ViewHeightSizable;
    if let Some(cls) = objc2::runtime::AnyClass::get(c"NSGlassEffectView") {
        unsafe {
            let alloc: *mut AnyObject = msg_send![cls, alloc];
                let inst: *mut AnyObject = msg_send![alloc, initWithFrame: frame];
            if !inst.is_null() {
                let _: () = msg_send![inst, setAutoresizingMask: resize];
                objc_utils::set_dark_appearance(inst);
                if let Some(r) = Retained::retain(inst) { return Retained::cast_unchecked(r); }
            }
        }
    }
    let v = NSVisualEffectView::initWithFrame(NSVisualEffectView::alloc(mtm), frame);
    v.setMaterial(NSVisualEffectMaterial::HUDWindow);
    v.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
    v.setState(NSVisualEffectState::Active);
    v.setAutoresizingMask(resize);
    unsafe { objc_utils::set_dark_appearance(&*v as *const _ as *mut AnyObject); }
    Retained::into_super(v)
}
