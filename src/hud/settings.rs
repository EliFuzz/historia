use std::cell::RefCell;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

thread_local! { static SETTINGS: RefCell<Settings> = RefCell::new(Settings::default()); }

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RetentionPeriod { OneDay, SevenDays, ThirtyDays, Never }

impl RetentionPeriod {
    pub const ALL: &[Self] = &[Self::OneDay, Self::SevenDays, Self::ThirtyDays, Self::Never];
    pub fn as_secs(&self) -> Option<u64> { match self { Self::OneDay => Some(86400), Self::SevenDays => Some(604800), Self::ThirtyDays => Some(2592000), Self::Never => None } }
    pub fn label(&self) -> &str { match self { Self::OneDay => "1 Day", Self::SevenDays => "7 Days", Self::ThirtyDays => "30 Days", Self::Never => "Never" } }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ItemsLimit { Ten, Twenty, Fifty, Hundred }

impl ItemsLimit {
    pub const ALL: &[Self] = &[Self::Ten, Self::Twenty, Self::Fifty, Self::Hundred];
    pub fn value(&self) -> usize { match self { Self::Ten => 10, Self::Twenty => 20, Self::Fifty => 50, Self::Hundred => 100 } }
    pub fn label(&self) -> &str { match self { Self::Ten => "10", Self::Twenty => "20", Self::Fifty => "50", Self::Hundred => "100" } }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Settings { pub retention_period: RetentionPeriod, pub items_limit: ItemsLimit }

impl Default for Settings {
    fn default() -> Self { Self { retention_period: RetentionPeriod::ThirtyDays, items_limit: ItemsLimit::Fifty } }
}

pub fn exe_dir() -> PathBuf {
    std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.to_path_buf())).unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
}

pub fn init() {
    let s = std::fs::read_to_string(exe_dir().join("settings.json")).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    SETTINGS.with(|st| *st.borrow_mut() = s);
}

pub fn get() -> Settings { SETTINGS.with(|s| s.borrow().clone()) }

pub fn set_retention_period(p: RetentionPeriod) { SETTINGS.with(|s| s.borrow_mut().retention_period = p); save(); }
pub fn set_items_limit(l: ItemsLimit) { SETTINGS.with(|s| s.borrow_mut().items_limit = l); save(); }

fn save() {
    SETTINGS.with(|s| { if let Ok(j) = serde_json::to_string_pretty(&*s.borrow()) { let _ = std::fs::write(exe_dir().join("settings.json"), j); } });
}
