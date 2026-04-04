use std::cell::OnceCell;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{DefinedClass, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate};
use objc2_foundation::{MainThreadMarker, NSNotification, NSObject, NSObjectProtocol};

use super::hud::{clipboard, controller, events, panel};
use super::setup_observers;

struct AppIvars {
    _tokens: OnceCell<Vec<Retained<ProtocolObject<dyn NSObjectProtocol>>>>,
    _monitor: OnceCell<Retained<clipboard::ClipboardMonitor>>,
}

impl Default for AppIvars {
    fn default() -> Self {
        Self { _tokens: OnceCell::new(), _monitor: OnceCell::new() }
    }
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = AppIvars]
    #[name = "HSTAppDelegate"]
    struct AppDelegate;
    unsafe impl NSObjectProtocol for AppDelegate {}
    unsafe impl NSApplicationDelegate for AppDelegate {
        #[unsafe(method(applicationDidFinishLaunching:))]
        fn launched(&self, _: &NSNotification) {
            let mtm = self.mtm();
            controller::init_targets(mtm);
            let (p, ds, cv) = panel::create_hud_panel(mtm);
            controller::init(p, ds, cv);
            let mut tokens = setup_observers();
            tokens.extend(events::setup_event_monitors());
            self.ivars()._tokens.set(tokens).ok();
            self.ivars()._monitor.set(clipboard::start_monitoring(mtm)).ok();
            NSApplication::sharedApplication(mtm)
                .setActivationPolicy(NSApplicationActivationPolicy::Accessory);
        }
    }
);

impl AppDelegate {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(AppIvars::default());
        unsafe { msg_send![super(this), init] }
    }
}

pub fn run() -> ! {
    let mtm = MainThreadMarker::new().unwrap();
    let app = NSApplication::sharedApplication(mtm);
    let delegate = AppDelegate::new(mtm);
    app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    app.run();
    unreachable!()
}
