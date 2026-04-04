use std::cell::RefCell;

use objc2::ffi::NSInteger;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{define_class, msg_send, ClassType, DefinedClass, MainThreadOnly};
use objc2_app_kit::{
    NSCollectionView, NSCollectionViewDataSource, NSCollectionViewFlowLayout, NSCollectionViewItem,
    NSCollectionViewScrollDirection, NSColor, NSScrollView,
};
use objc2_foundation::{MainThreadMarker, NSIndexPath, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize};

use super::card::CardItem;
use crate::hud::state::{ClipboardItem, ItemKind, VISIBLE_CARD_COUNT};

const GAP: f64 = 6.0;
const PAD: f64 = 4.0;

pub(crate) struct DSIvars {
    card_size: RefCell<NSSize>,
    filter: RefCell<String>,
    filtered: RefCell<Vec<usize>>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = DSIvars]
    #[name = "HSTDataSource"]
    pub(crate) struct DataSource;
    unsafe impl NSObjectProtocol for DataSource {}
    unsafe impl NSCollectionViewDataSource for DataSource {
        #[unsafe(method(collectionView:numberOfItemsInSection:))]
        fn count(&self, _: &NSCollectionView, _: NSInteger) -> NSInteger { self.ivars().filtered.borrow().len() as NSInteger }
        #[unsafe(method_id(collectionView:itemForRepresentedObjectAtIndexPath:))]
        fn item_for(&self, cv: &NSCollectionView, ip: &NSIndexPath) -> Retained<NSCollectionViewItem> {
            let id = objc2_app_kit::NSUserInterfaceItemIdentifier::from_str("CardItem");
            let item = cv.makeItemWithIdentifier_forIndexPath(&id, ip);
            let di: NSInteger = unsafe { msg_send![ip, item] };
            let f = self.ivars().filtered.borrow();
            if let Some(&ai) = f.get(di as usize) {
                if let Some(ci) = item.downcast_ref::<CardItem>() {
                    if let Some(clip) = crate::hud::persistence::read_item(ai) {
                        ci.configure(&clip, di as usize, *self.ivars().card_size.borrow());
                    }
                }
            }
            item
        }
    }
);

impl DataSource {
    fn new(mtm: MainThreadMarker, card_size: NSSize) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(DSIvars {
            card_size: RefCell::new(card_size), filter: RefCell::new(String::new()),
            filtered: RefCell::new(crate::hud::persistence::active_indices()),
        });
        unsafe { msg_send![super(this), init] }
    }

    pub fn item_at_display_index(&self, di: usize) -> Option<ClipboardItem> {
        let f = self.ivars().filtered.borrow();
        crate::hud::persistence::read_item(*f.get(di)?)
    }

    pub fn remove_item_by_id(&self, id: usize) { crate::hud::persistence::remove_by_id(id); self.refresh(); }
    pub fn remove_all(&self) { crate::hud::persistence::clear(); self.refresh(); }
    pub fn set_filter(&self, q: &str) { *self.ivars().filter.borrow_mut() = q.to_owned(); self.refresh(); }

    pub fn add_item(&self, content: String, app_name: String, kind: ItemKind, img: Option<(String, Vec<u8>)>) {
        let r = img.as_ref().map(|(t, d)| (t.as_str(), d.as_slice()));
        crate::hud::persistence::add_item(content, app_name, kind, r);
        self.refresh();
    }

    pub fn enforce_limits(&self, max: usize, age: Option<u64>) { crate::hud::persistence::enforce_limits(max, age); self.refresh(); }

    fn refresh(&self) {
        let q = self.ivars().filter.borrow().to_lowercase();
        *self.ivars().filtered.borrow_mut() = if q.is_empty() { crate::hud::persistence::active_indices() } else { crate::hud::persistence::search(&q) };
    }
}

pub fn card_width(w: f64) -> f64 { (w - PAD * 2.0 - GAP * (VISIBLE_CARD_COUNT as f64 - 1.0)) / VISIBLE_CARD_COUNT as f64 }
pub fn card_height(w: f64) -> f64 { card_width(w).mul_add(0.75, 0.0).clamp(80.0, 160.0) }

pub fn create_scroll_view(mtm: MainThreadMarker, frame: NSRect) -> (Retained<NSScrollView>, Retained<DataSource>, Retained<NSCollectionView>) {
    let cw = card_width(frame.size.width);
    let ch = card_height(frame.size.width);
    let layout = NSCollectionViewFlowLayout::new(mtm);
    layout.setScrollDirection(NSCollectionViewScrollDirection::Horizontal);
    layout.setItemSize(NSSize::new(cw, ch));
    layout.setMinimumInteritemSpacing(GAP);
    layout.setMinimumLineSpacing(GAP);
    let cv = NSCollectionView::initWithFrame(NSCollectionView::alloc(mtm), NSRect::new(NSPoint::ZERO, frame.size));
    cv.setCollectionViewLayout(Some(&layout));
    let id = objc2_app_kit::NSUserInterfaceItemIdentifier::from_str("CardItem");
    unsafe { cv.registerClass_forItemWithIdentifier(Some(CardItem::class()), &id); let _: () = msg_send![&cv, setWantsLayer: true]; }
    cv.setBackgroundColors(Some(&objc2_foundation::NSArray::from_retained_slice(&[NSColor::clearColor()])));
    let ds = DataSource::new(mtm, NSSize::new(cw, ch));
    cv.setDataSource(Some(ProtocolObject::from_ref(&*ds)));
    let sv = NSScrollView::initWithFrame(NSScrollView::alloc(mtm), NSRect::new(NSPoint::new(PAD, PAD), NSSize::new(frame.size.width - PAD * 2.0, ch)));
    sv.setDocumentView(Some(&cv));
    sv.setHasHorizontalScroller(false); sv.setHasVerticalScroller(false); sv.setDrawsBackground(false);
    unsafe { let _: () = msg_send![&sv, setWantsLayer: true]; }
    sv.setBackgroundColor(&NSColor::clearColor());
    (sv, ds, cv)
}
