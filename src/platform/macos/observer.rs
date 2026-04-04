use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_app_kit::{NSWorkspace, NSWorkspaceActiveSpaceDidChangeNotification};
use objc2_foundation::{NSNotification, NSNotificationCenter, NSObjectProtocol, NSOperationQueue};
use std::ptr::NonNull;

pub fn setup_observers() -> Vec<Retained<ProtocolObject<dyn NSObjectProtocol>>> {
    let noop = RcBlock::new(|_notif: NonNull<NSNotification>| {});
    let queue = NSOperationQueue::mainQueue();

    let ws_token = unsafe {
        NSWorkspace::sharedWorkspace()
            .notificationCenter()
            .addObserverForName_object_queue_usingBlock(
                Some(NSWorkspaceActiveSpaceDidChangeNotification),
                None,
                Some(&queue),
                &noop,
            )
    };

    let screen_name =
        objc2_foundation::ns_string!("NSApplicationDidChangeScreenParametersNotification");
    let screen_token = unsafe {
        NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
            Some(screen_name),
            None,
            Some(&queue),
            &noop,
        )
    };

    vec![ws_token, screen_token]
}
