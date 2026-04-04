pub mod hud;
mod platform;
mod updater;

#[cfg(test)]
mod tests;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    hud::settings::init();
    hud::persistence::init();
    let (tx, rx) = std::sync::mpsc::channel();
    updater::start(tx);
    std::thread::spawn(move || {
        for msg in rx {
            match msg {
                updater::Msg::UpdateAvailable(v) => eprintln!("historia v{VERSION} → v{v} available"),
            }
        }
    });
    platform::run();
}
