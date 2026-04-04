use self_update::backends::github::ReleaseList;
use self_update::version::bump_is_greater;
use std::time::Duration;

const CURRENT_VERSION: &str = crate::VERSION;
const REPO_OWNER: &str = "EliFuzz";
const REPO_NAME: &str = "historia";
const CHECK_INTERVAL: Duration = Duration::from_secs(86400);

pub enum Msg {
    UpdateAvailable(String),
}

pub fn start(tx: std::sync::mpsc::Sender<Msg>) {
    std::thread::spawn(move || {
        loop {
            if let Some(version) = fetch_latest_version() {
                if bump_is_greater(CURRENT_VERSION, &version).unwrap_or(false) {
                    tx.send(Msg::UpdateAvailable(version)).ok();
                }
            }
            std::thread::sleep(CHECK_INTERVAL);
        }
    });
}

fn fetch_latest_version() -> Option<String> {
    ReleaseList::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .build()
        .ok()?
        .fetch()
        .ok()?
        .into_iter()
        .next()
        .map(|r| r.version)
}
