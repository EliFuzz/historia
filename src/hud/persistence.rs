use std::cell::RefCell;
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use memmap2::Mmap;
use serde::{Deserialize, Serialize};
use super::state::{ClipboardItem, ItemKind, now_unix};

const COMPACTION_THRESHOLD: usize = 64;
thread_local! { static STORE: RefCell<Option<DiskStore>> = const { RefCell::new(None) }; }

pub fn init() { STORE.with(|s| *s.borrow_mut() = Some(DiskStore::open(data_path()))); }
pub fn active_indices() -> Vec<usize> { STORE.with(|s| s.borrow().as_ref().map_or(Vec::new(), |st| st.active.clone())) }
pub fn read_item(idx: usize) -> Option<ClipboardItem> { STORE.with(|s| s.borrow().as_ref().and_then(|st| st.read_at(idx))) }
pub fn add_item(content: String, app_name: String, kind: ItemKind, image_data: Option<(&str, &[u8])>) { with_store(|st| st.add(content, app_name, kind, image_data)); }
pub fn remove_by_id(id: usize) { with_store(|st| st.remove(id as u32)); }
pub fn clear() { with_store(|st| st.clear()); }
pub fn enforce_limits(max_items: usize, max_age_secs: Option<u64>) { with_store(|st| st.enforce_limits(max_items, max_age_secs)); }
pub fn search(query: &str) -> Vec<usize> { STORE.with(|s| s.borrow().as_ref().map_or(Vec::new(), |st| st.search(query))) }

pub fn load_blob_data(path: &str) -> Option<(String, Vec<u8>)> {
    let data = fs::read(path).ok()?;
    Some((ItemKind::blob_uti(path.rsplit('.').next().unwrap_or("bin")).into(), data))
}

pub fn blobs_dir() -> PathBuf { super::settings::exe_dir().join("blobs") }
fn data_path() -> PathBuf { super::settings::exe_dir().join("clipboard.jsonl") }
fn with_store(f: impl FnOnce(&mut DiskStore)) { STORE.with(|s| { if let Some(st) = s.borrow_mut().as_mut() { f(st); } }); }

#[derive(Serialize, Deserialize)]
struct Entry { content: String, app_name: String, kind: String, captured_at: u64, #[serde(skip_serializing_if = "Option::is_none", default)] blob: Option<String> }
struct Meta { id: u32, offset: u64, len: u32, captured_at: u64 }
struct DiskStore { path: PathBuf, index: Vec<Meta>, deleted: HashSet<u32>, pub active: Vec<usize>, next_id: u32, mmap: Option<Mmap> }

impl DiskStore {
    fn open(path: PathBuf) -> Self {
        let (index, mmap, next_id) = scan(&path);
        let mut s = Self { path, index, deleted: HashSet::new(), active: Vec::new(), next_id, mmap };
        s.rebuild_active();
        s
    }

    fn read_at(&self, idx: usize) -> Option<ClipboardItem> {
        let m = self.index.get(idx).filter(|m| !self.deleted.contains(&m.id))?;
        let e: Entry = serde_json::from_slice(self.line(m)?).ok()?;
        Some(ClipboardItem { id: m.id as usize, content: e.content, app_name: e.app_name, captured_at: e.captured_at, kind: ItemKind::deserialize(&e.kind), blob_path: e.blob })
    }

    fn add(&mut self, content: String, app_name: String, kind: ItemKind, image_data: Option<(&str, &[u8])>) {
        let deduped = self.dedup(&content);
        let (id, ts) = (self.next_id, now_unix());
        self.next_id += 1;
        let blob = image_data.map(|(tn, data)| {
            let ext = tn.rsplit('.').next().unwrap_or("bin");
            let dir = blobs_dir();
            let _ = fs::create_dir_all(&dir);
            let p = dir.join(format!("pb_{id}.{ext}"));
            let _ = fs::write(&p, data);
            p.to_string_lossy().into_owned()
        });
        let Ok(json) = serde_json::to_string(&Entry { content, app_name, kind: kind.serialize(), captured_at: ts, blob }) else { return };
        self.mmap = None;
        let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&self.path) else { return };
        let offset = f.metadata().map_or(0, |m| m.len());
        if writeln!(f, "{json}").is_err() { return; }
        drop(f);
        self.index.push(Meta { id, offset, len: json.len() as u32, captured_at: ts });
        self.refresh_mmap();
        self.rebuild_active();
        if deduped { self.compact(); }
    }

    fn remove(&mut self, id: u32) {
        if let Some(m) = self.index.iter().find(|m| m.id == id) {
            if let Some(e) = self.line(m).and_then(|l| serde_json::from_slice::<Entry>(l).ok()) {
                if let Some(ref p) = e.blob { let _ = fs::remove_file(p); }
            }
        }
        self.deleted.insert(id);
        self.rebuild_active();
        self.maybe_compact();
    }

    fn clear(&mut self) {
        let _ = fs::remove_dir_all(blobs_dir());
        self.index.clear(); self.deleted.clear(); self.active.clear(); self.mmap = None;
        let _ = File::create(&self.path);
    }

    fn enforce_limits(&mut self, max_items: usize, max_age: Option<u64>) {
        let mut changed = false;
        if let Some(age) = max_age {
            let cutoff = now_unix().saturating_sub(age);
            for m in &self.index {
                if !self.deleted.contains(&m.id) && m.captured_at < cutoff { self.deleted.insert(m.id); changed = true; }
            }
        }
        if changed { self.rebuild_active(); }
        if self.active.len() > max_items {
            self.active[max_items..].iter().for_each(|&i| { self.deleted.insert(self.index[i].id); });
            changed = true;
        }
        if changed { self.rebuild_active(); self.maybe_compact(); }
    }

    fn search(&self, query: &str) -> Vec<usize> {
        let ql = query.to_lowercase();
        let qb = ql.as_bytes();
        let Some(ref mmap) = self.mmap else { return Vec::new() };
        self.active.iter().filter_map(|&i| {
            let line = self.line_in(mmap, &self.index[i])?;
            if !ci_contains(line, qb) { return None; }
            serde_json::from_slice::<Entry>(line).ok().filter(|e| e.content.to_lowercase().contains(&ql)).map(|_| i)
        }).collect()
    }

    fn dedup(&mut self, content: &str) -> bool {
        let mmap = match self.mmap.as_ref() { Some(m) => m, None => return false };
        let ids: Vec<u32> = self.index.iter()
            .filter(|m| !self.deleted.contains(&m.id))
            .filter_map(|m| self.line_in(mmap, m).and_then(|l| serde_json::from_slice::<Entry>(l).ok()).filter(|e| e.content == content).map(|_| m.id))
            .collect();
        let d = !ids.is_empty();
        ids.into_iter().for_each(|id| { self.deleted.insert(id); });
        d
    }

    fn rebuild_active(&mut self) {
        let mut a: Vec<usize> = self.index.iter().enumerate().filter(|(_, m)| !self.deleted.contains(&m.id)).map(|(i, _)| i).collect();
        a.sort_by(|&x, &y| self.index[y].captured_at.cmp(&self.index[x].captured_at));
        self.active = a;
    }

    fn refresh_mmap(&mut self) {
        self.mmap = File::open(&self.path).ok().filter(|f| f.metadata().is_ok_and(|m| m.len() > 0)).and_then(|f| unsafe { Mmap::map(&f).ok() });
    }

    fn line(&self, m: &Meta) -> Option<&[u8]> { self.mmap.as_ref().and_then(|mm| self.line_in(mm, m)) }
    fn line_in<'a>(&self, mmap: &'a Mmap, m: &Meta) -> Option<&'a [u8]> {
        let (s, n) = (m.offset as usize, m.len as usize);
        (s + n <= mmap.len()).then_some(&mmap[s..s + n])
    }

    fn maybe_compact(&mut self) {
        let d = self.deleted.len();
        if d > 0 && (d >= COMPACTION_THRESHOLD || d > self.index.len() / 2) { self.compact(); }
    }

    fn compact(&mut self) {
        let Some(ref mmap) = self.mmap else { return };
        let tmp = self.path.with_extension("jsonl.tmp");
        let Ok(mut f) = File::create(&tmp) else { return };
        let mut blobs = Vec::new();
        for m in &self.index {
            let Some(line) = self.line_in(mmap, m) else { continue };
            if self.deleted.contains(&m.id) {
                if let Ok(e) = serde_json::from_slice::<Entry>(line) { blobs.extend(e.blob); }
                continue;
            }
            if f.write_all(line).is_err() || f.write_all(b"\n").is_err() { let _ = fs::remove_file(&tmp); return; }
        }
        drop(f);
        self.mmap = None;
        if fs::rename(&tmp, &self.path).is_err() { let _ = fs::remove_file(&tmp); return; }
        blobs.iter().for_each(|p| { let _ = fs::remove_file(p); });
        let (index, mm, next_id) = scan(&self.path);
        (self.index, self.mmap, self.next_id) = (index, mm, next_id);
        self.deleted.clear();
        self.rebuild_active();
    }
}

fn scan(path: &PathBuf) -> (Vec<Meta>, Option<Mmap>, u32) {
    let Some(mmap) = File::open(path).ok().filter(|f| f.metadata().map_or(false, |m| m.len() > 0)).and_then(|f| unsafe { Mmap::map(&f).ok() }) else { return (Vec::new(), None, 0) };
    let base = mmap.as_ptr() as usize;
    let mut entries = Vec::new();
    let mut nid = 0u32;
    for line in mmap.split(|&b| b == b'\n') {
        if line.is_empty() || !line.contains(&b'{') { continue; }
        entries.push(Meta { id: nid, offset: (line.as_ptr() as usize - base) as u64, len: line.len() as u32, captured_at: extract_ts(line) });
        nid += 1;
    }
    (entries, Some(mmap), nid)
}

fn extract_ts(line: &[u8]) -> u64 {
    const K: &[u8] = b"\"captured_at\":";
    line.windows(K.len()).position(|w| w == K).and_then(|i| {
        let r = &line[i + K.len()..];
        let s = r.iter().position(|&b| b.is_ascii_digit())?;
        let e = s + r[s..].iter().take_while(|&&b| b.is_ascii_digit()).count();
        std::str::from_utf8(&r[s..e]).ok()?.parse().ok()
    }).unwrap_or(0)
}

fn ci_contains(haystack: &[u8], needle: &[u8]) -> bool {
    needle.is_empty() || (haystack.len() >= needle.len() && haystack.windows(needle.len()).any(|w| w.iter().zip(needle).all(|(&a, &b)| a.to_ascii_lowercase() == b)))
}
