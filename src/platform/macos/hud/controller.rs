use std::cell::{Cell, RefCell};

use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject};
use objc2::{MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSCollectionView, NSPanel, NSTextField};
use objc2_foundation::{MainThreadMarker, NSNotification, NSObject, NSObjectProtocol, NSPoint, NSString};

use super::collection::DataSource;
use crate::hud::state::ItemKind;
use crate::platform::macos::objc_utils;

thread_local! {
    static PANEL: RefCell<Option<Retained<NSPanel>>> = const { RefCell::new(None) };
    static DS: RefCell<Option<Retained<DataSource>>> = const { RefCell::new(None) };
    static CV: RefCell<Option<Retained<NSCollectionView>>> = const { RefCell::new(None) };
    static TARGET: RefCell<Option<Retained<ActionTarget>>> = const { RefCell::new(None) };
    static SEARCH_DEL: RefCell<Option<Retained<SearchDelegate>>> = const { RefCell::new(None) };
    static SEARCH_FIELD: RefCell<Option<Retained<NSTextField>>> = const { RefCell::new(None) };
    static SKIP: Cell<bool> = const { Cell::new(false) };
}

pub fn init_targets(mtm: MainThreadMarker) {
    TARGET.with(|t| *t.borrow_mut() = Some(ActionTarget::new(mtm)));
    SEARCH_DEL.with(|d| *d.borrow_mut() = Some(SearchDelegate::new(mtm)));
}

pub fn init(panel: Retained<NSPanel>, ds: Retained<DataSource>, cv: Retained<NSCollectionView>) {
    PANEL.with(|p| *p.borrow_mut() = Some(panel));
    DS.with(|d| *d.borrow_mut() = Some(ds));
    CV.with(|c| *c.borrow_mut() = Some(cv));
}

pub fn set_search_field(f: Retained<NSTextField>) { SEARCH_FIELD.with(|sf| *sf.borrow_mut() = Some(f)); }
pub fn set_skip_next_capture() { SKIP.with(|s| s.set(true)); }

pub fn should_skip_capture() -> bool {
    SKIP.with(|s| { let v = s.get(); s.set(false); v })
}

pub fn toggle_panel() { if is_panel_visible() { hide_panel(); } else { show_panel(); } }

pub fn show_panel() {
    with_panel(|panel| {
        let mtm = MainThreadMarker::new().unwrap();
        super::panel::reposition_panel(panel, mtm);
        panel.orderFrontRegardless();
        panel.makeKeyWindow();
    });
    with_search_field(|f| f.setStringValue(&NSString::from_str("")));
    filter_items("");
    scroll_to_start();
    PANEL.with(|p| SEARCH_FIELD.with(|f| {
        let (Some(panel), Some(field)) = (p.borrow().clone(), f.borrow().clone()) else { return };
        unsafe { let _: bool = msg_send![&*panel, makeFirstResponder: &*field]; }
    }));
}

pub fn hide_panel() {
    with_panel(|p| p.orderOut(None));
    unsafe { let _: () = msg_send![objc_utils::ns_app(), hide: std::ptr::null::<AnyObject>()]; }
}

pub fn is_panel_visible() -> bool {
    PANEL.with(|p| p.borrow().as_ref().is_some_and(|panel| panel.isVisible()))
}

pub fn copy_item_at_display_index(idx: usize) {
    DS.with(|d| {
        let Some(ds) = d.borrow().clone() else { return };
        let Some(item) = ds.item_at_display_index(idx) else { return };
        set_skip_next_capture();
        restore_to_clipboard(&item);
    });
    hide_panel();
}

pub fn delete_item_by_id(id: usize) { with_ds_reload(|ds| ds.remove_item_by_id(id)); }
pub fn delete_all() { with_ds_reload(|ds| ds.remove_all()); }
pub fn filter_items(q: &str) { with_ds_reload(|ds| ds.set_filter(q)); }

pub fn add_clipboard_item(content: String, app_name: String, kind: ItemKind, image_data: Option<(String, Vec<u8>)>) {
    let settings = crate::hud::settings::get();
    with_ds_reload(|ds| {
        ds.add_item(content, app_name, kind, image_data);
        ds.enforce_limits(settings.items_limit.value(), settings.retention_period.as_secs());
    });
}

pub fn action_target() -> Option<Retained<ActionTarget>> { TARGET.with(|t| t.borrow().clone()) }
pub fn search_delegate_ref() -> Option<Retained<SearchDelegate>> { SEARCH_DEL.with(|d| d.borrow().clone()) }

fn with_panel(f: impl FnOnce(&NSPanel)) { PANEL.with(|p| { if let Some(panel) = p.borrow().clone() { f(&panel); } }); }
fn with_search_field(f: impl FnOnce(&NSTextField)) { SEARCH_FIELD.with(|sf| { if let Some(field) = sf.borrow().clone() { f(&field); } }); }
fn with_ds_reload(f: impl FnOnce(&DataSource)) {
    DS.with(|d| { if let Some(ds) = d.borrow().clone() { f(&ds); } });
    CV.with(|c| { if let Some(cv) = c.borrow().clone() { cv.reloadData(); } });
}

fn scroll_to_start() {
    CV.with(|c| {
        let Some(cv) = c.borrow().clone() else { return };
        unsafe {
            let sv: *mut AnyObject = msg_send![&*cv, enclosingScrollView];
            if sv.is_null() { return; }
            let clip: *mut AnyObject = msg_send![sv, contentView];
            if clip.is_null() { return; }
            let _: () = msg_send![clip, scrollToPoint: NSPoint::ZERO];
            let _: () = msg_send![sv, reflectScrolledClipView: clip];
        }
    });
}

fn restore_to_clipboard(item: &crate::hud::state::ClipboardItem) {
    unsafe {
        let pb: *mut AnyObject = msg_send![AnyClass::get(c"NSPasteboard").unwrap(), generalPasteboard];
        let _: i64 = msg_send![pb, clearContents];
        if let Some(ref blob) = item.blob_path {
            let Some((uti, data)) = crate::hud::persistence::load_blob_data(blob) else { return };
            let ns_type = NSString::from_str(&uti);
            let ns_data: *mut AnyObject = msg_send![AnyClass::get(c"NSData").unwrap(), dataWithBytes: data.as_ptr() as *const std::ffi::c_void, length: data.len()];
            if !ns_data.is_null() { let _: bool = msg_send![pb, setData: ns_data, forType: &*ns_type]; }
            return;
        }
        let (s, uti) = match &item.kind {
            ItemKind::Text | ItemKind::Color(_) => (item.content.clone(), crate::hud::state::UTI_PLAIN_TEXT),
            _ => (format!("file://{}", item.content), crate::hud::state::UTI_FILE_URL),
        };
        let ns = NSString::from_str(&s);
        let key = NSString::from_str(uti);
        let _: bool = msg_send![pb, setString: &*ns, forType: &*key];
    }
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    #[name = "HSTActionTarget"]
    pub(crate) struct ActionTarget;
    unsafe impl NSObjectProtocol for ActionTarget {}
    impl ActionTarget {
        #[unsafe(method(deleteItem:))]
        fn delete_item(&self, sender: &AnyObject) { let tag: isize = unsafe { msg_send![sender, tag] }; delete_item_by_id(tag as usize); }
        #[unsafe(method(deleteAllItems:))]
        fn delete_all_items(&self, _: &AnyObject) { delete_all(); }
        #[unsafe(method(selectItem:))]
        fn select_item(&self, sender: &AnyObject) { let tag: isize = unsafe { msg_send![sender, tag] }; copy_item_at_display_index(tag as usize); }
        #[unsafe(method(showSettingsMenu:))]
        fn show_settings(&self, sender: &AnyObject) { crate::platform::macos::settings_menu::show_menu(sender); }
        #[unsafe(method(handleRetention:))]
        fn retention(&self, sender: &AnyObject) {
            let tag: isize = unsafe { msg_send![sender, tag] };
            let Some(&p) = crate::hud::settings::RetentionPeriod::ALL.get(tag as usize) else { return };
            crate::hud::settings::set_retention_period(p);
            let s = crate::hud::settings::get();
            with_ds_reload(|ds| ds.enforce_limits(s.items_limit.value(), s.retention_period.as_secs()));
        }
        #[unsafe(method(handleItemsLimit:))]
        fn items_limit(&self, sender: &AnyObject) {
            let tag: isize = unsafe { msg_send![sender, tag] };
            let Some(&l) = crate::hud::settings::ItemsLimit::ALL.get(tag as usize) else { return };
            crate::hud::settings::set_items_limit(l);
            let s = crate::hud::settings::get();
            with_ds_reload(|ds| ds.enforce_limits(s.items_limit.value(), s.retention_period.as_secs()));
        }
        #[unsafe(method(handleQuit:))]
        fn quit(&self, _: &AnyObject) { unsafe { let _: () = msg_send![objc_utils::ns_app(), terminate: std::ptr::null::<AnyObject>()]; } }
    }
);

impl ActionTarget {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    #[name = "HSTSearchDelegate"]
    pub(crate) struct SearchDelegate;
    unsafe impl NSObjectProtocol for SearchDelegate {}
    impl SearchDelegate {
        #[unsafe(method(controlTextDidChange:))]
        fn text_changed(&self, n: &NSNotification) {
            unsafe {
                let obj: *mut AnyObject = msg_send![n, object];
                if obj.is_null() { return; }
                let s: *const NSString = msg_send![&*obj, stringValue];
                if s.is_null() { return; }
                filter_items(&(*s).to_string());
            }
        }
    }
);

impl SearchDelegate {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}
