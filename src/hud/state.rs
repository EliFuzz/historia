const EXT_MAP: &[(&[&str], fn() -> ItemKind)] = &[
    (&["jpg", "jpeg", "png", "gif", "bmp", "tiff", "tif", "webp", "heic", "heif", "svg", "ico"], || ItemKind::Image),
    (&["mp4", "mov", "avi", "mkv", "wmv", "flv", "webm", "m4v"], || ItemKind::Video),
    (&["mp3", "wav", "aac", "flac", "ogg", "wma", "m4a", "aiff"], || ItemKind::Audio),
    (&["pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "pages", "numbers", "keynote", "rtf", "odt", "ods"], || ItemKind::Document),
    (&["rs", "py", "js", "ts", "c", "cpp", "h", "java", "go", "rb", "swift", "sh", "html", "css", "json", "yaml", "yml", "toml", "xml", "sql"], || ItemKind::Code),
    (&["zip", "tar", "gz", "bz2", "xz", "7z", "rar", "dmg", "iso"], || ItemKind::Archive),
    (&["exe", "app", "msi", "deb", "rpm", "bin"], || ItemKind::Executable),
];

#[derive(Clone)]
pub enum ItemKind {
    Text, Color(String), File(usize), Image, Video, Audio, Document, Code, Archive, Executable,
}

impl ItemKind {
    pub fn from_filename(name: &str) -> Self {
        let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
        for &(exts, ctor) in EXT_MAP {
            if exts.contains(&ext.as_str()) { return ctor(); }
        }
        Self::File(1)
    }

    pub fn from_text(content: &str) -> Self {
        let t = content.trim();
        if t.starts_with('#') && (t.len() == 7 || t.len() == 9) && t[1..].chars().all(|c| c.is_ascii_hexdigit()) {
            return Self::Color(t.to_owned());
        }
        Self::Text
    }

    pub fn serialize(&self) -> String {
        match self {
            Self::Text => "text".into(), Self::Color(h) => format!("color:{h}"),
            Self::File(n) => format!("file:{n}"), Self::Image => "image".into(),
            Self::Video => "video".into(), Self::Audio => "audio".into(),
            Self::Document => "document".into(), Self::Code => "code".into(),
            Self::Archive => "archive".into(), Self::Executable => "executable".into(),
        }
    }

    pub fn deserialize(s: &str) -> Self {
        match s {
            "text" => Self::Text, "image" => Self::Image, "video" => Self::Video,
            "audio" => Self::Audio, "document" => Self::Document, "code" => Self::Code,
            "archive" => Self::Archive, "executable" => Self::Executable,
            o if o.starts_with("color:") => Self::Color(o[6..].into()),
            o if o.starts_with("file:") => Self::File(o[5..].parse().unwrap_or(1)),
            _ => Self::Text,
        }
    }

    pub fn is_text(&self) -> bool { matches!(self, Self::Text) }
    pub fn is_inline(&self) -> bool { matches!(self, Self::Text | Self::Color(_) | Self::Image) }

    pub fn blob_uti(ext: &str) -> &'static str {
        match ext { "png" => UTI_PNG, "tiff" | "tif" => UTI_TIFF, "jpeg" | "jpg" => "public.jpeg", _ => "public.data" }
    }
}

pub const UTI_TIFF: &str = "public.tiff";
pub const UTI_PNG: &str = "public.png";
pub const UTI_FILE_URL: &str = "public.file-url";
pub const UTI_PLAIN_TEXT: &str = "public.utf8-plain-text";
pub const VISIBLE_CARD_COUNT: usize = 9;

pub fn now_unix() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn format_bytes(bytes: usize) -> String {
    if bytes < 1024 { return format!("{bytes} B"); }
    if bytes < 1048576 { return format!("{:.1} KB", bytes as f64 / 1024.0); }
    format!("{:.1} MB", bytes as f64 / 1048576.0)
}

#[derive(Clone)]
pub struct ClipboardItem {
    pub id: usize,
    pub content: String,
    pub app_name: String,
    pub captured_at: u64,
    pub kind: ItemKind,
    pub blob_path: Option<String>,
}

impl ClipboardItem {
    pub fn relative_time(&self) -> String {
        let s = now_unix().saturating_sub(self.captured_at);
        match s { 0..5 => "Just now".into(), 5..60 => format!("{s}s ago"), 60..3600 => format!("{}m ago", s / 60), 3600..86400 => format!("{}h ago", s / 3600), _ => format!("{}d ago", s / 86400) }
    }

    pub fn display_name(&self) -> String {
        if self.kind.is_inline() { return format!("{}  {}", self.app_name, self.relative_time()); }
        let name = self.content.rsplit('/').find(|s| !s.is_empty()).unwrap_or(&self.content);
        format!("{name}  {}", self.relative_time())
    }
}
