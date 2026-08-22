use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::Instant;

use crate::lang::{fmt, t};

use rss_core::{
    Article, ArticleFilter, Feed, Folder, RefreshResult, RssReader, UnreadStats,
};


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Sidebar,
    List,
    Reader,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    All,
    Unread,
    Starred,
    Folder(i64),
    Feed(i64),
}

impl Selection {
    pub fn label(&self, folders: &[Folder], feeds: &[Feed]) -> String {
        match self {
            Selection::All => "All Articles".to_string(),
            Selection::Unread => "Unread".to_string(),
            Selection::Starred => "Starred".to_string(),
            Selection::Folder(id) => folders
                .iter()
                .find(|f| f.id == *id)
                .map(|f| f.name.clone())
                .unwrap_or_else(|| format!("Folder {id}")),
            Selection::Feed(id) => feeds
                .iter()
                .find(|f| f.id == *id)
                .map(|f| f.title.clone())
                .unwrap_or_else(|| format!("Feed {id}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMode {
    None,
    AddFeed { folder_id: Option<i64> },
    AddFolder,
    RenameFolder { id: i64 },
    MoveFeed { feed_id: i64 },
    Search,
    ImportOpml,
    ExportOpml,
}

impl InputMode {
    pub fn title(&self) -> String {
        match self {
            InputMode::None => String::new(),
            InputMode::AddFeed { .. } => t("input_add_feed"),
            InputMode::AddFolder => t("input_new_folder"),
            InputMode::RenameFolder { .. } => t("input_rename_folder"),
            InputMode::MoveFeed { .. } => t("input_move_feed"),
            InputMode::Search => t("input_search"),
            InputMode::ImportOpml => t("input_import"),
            InputMode::ExportOpml => t("input_export"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SidebarItem {
    All(i64),
    Unread(i64),
    Starred,
    Folder { id: i64, name: String, unread: i64 },
    Feed { id: i64, name: String, unread: i64 },
    Section(&'static str),
}

impl SidebarItem {
    pub fn selection(&self) -> Option<Selection> {
        match self {
            SidebarItem::All(_) => Some(Selection::All),
            SidebarItem::Unread(_) => Some(Selection::Unread),
            SidebarItem::Starred => Some(Selection::Starred),
            SidebarItem::Folder { id, .. } => Some(Selection::Folder(*id)),
            SidebarItem::Feed { id, .. } => Some(Selection::Feed(*id)),
            SidebarItem::Section(_) => None,
        }
    }

    pub fn unread(&self) -> i64 {
        match self {
            SidebarItem::All(n) | SidebarItem::Unread(n) => *n,
            SidebarItem::Folder { unread, .. } | SidebarItem::Feed { unread, .. } => *unread,
            _ => 0,
        }
    }

    pub fn is_section(&self) -> bool {
        matches!(self, SidebarItem::Section(_))
    }
}

pub struct App {
    pub reader: Arc<RssReader>,
    pub folders: Vec<Folder>,
    pub feeds: Vec<Feed>,
    pub unread: UnreadStats,
    pub selection: Selection,
    pub focus: Focus,
    pub sidebar: Vec<SidebarItem>,
    pub sidebar_index: usize,
    pub articles: Vec<Article>,
    pub list_index: usize,
    pub reader_text: String,
    pub reader_scroll: u16,
    pub current_article_id: Option<i64>,
    pub input: String,
    pub input_mode: InputMode,
    pub search: Option<String>,
    pub status: String,
    pub help: bool,
    pub refreshing: bool,
    pub last_refresh: Option<Instant>,
    refresh_tx: Sender<RefreshResult>,
    refresh_rx: Receiver<RefreshResult>,
}

impl App {
    pub fn new(reader: RssReader) -> Self {
        let (tx, rx) = mpsc::channel();
        let mut app = App {
            reader: Arc::new(reader),
            folders: Vec::new(),
            feeds: Vec::new(),
            unread: UnreadStats::default(),
            selection: Selection::All,
            focus: Focus::Sidebar,
            sidebar: Vec::new(),
            sidebar_index: 0,
            articles: Vec::new(),
            list_index: 0,
            reader_text: String::new(),
            reader_scroll: 0,
            current_article_id: None,
            input: String::new(),
            input_mode: InputMode::None,
            search: None,
            status: "press ? for help".to_string(),
            help: false,
            refreshing: false,
            last_refresh: None,
            refresh_tx: tx,
            refresh_rx: rx,
        };
        app.reload();
        app
    }

    pub fn data_dir() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("rss-reader")
    }

    // ---------- 数据加载 ----------

    pub fn reload(&mut self) {
        self.folders = self.reader.list_folders().unwrap_or_default();
        self.feeds = self.reader.list_feeds().unwrap_or_default();
        self.unread = self.reader.unread_stats().unwrap_or_default();
        self.build_sidebar();
        self.reload_articles();
        self.reload_reader();
    }

    fn build_sidebar(&mut self) {
        let mut items: Vec<SidebarItem> = Vec::new();
        items.push(SidebarItem::All(self.unread.total));
        items.push(SidebarItem::Unread(self.unread.total));
        items.push(SidebarItem::Starred);

        let mut folder_unread: Vec<(i64, i64)> = Vec::new();
        for f in &self.feeds {
            if let Some(fid) = f.folder_id {
                let unread = self
                    .unread
                    .per_feed
                    .iter()
                    .find(|u| u.feed_id == f.id)
                    .map(|u| u.unread)
                    .unwrap_or(0);
                if let Some(entry) = folder_unread.iter_mut().find(|(id, _)| *id == fid) {
                    entry.1 += unread;
                } else {
                    folder_unread.push((fid, unread));
                }
            }
        }

        if !self.folders.is_empty() {
            items.push(SidebarItem::Section("Folders"));
        }
        for folder in &self.folders {
            let unread = folder_unread
                .iter()
                .find(|(id, _)| *id == folder.id)
                .map(|(_, n)| *n)
                .unwrap_or(0);
            items.push(SidebarItem::Folder {
                id: folder.id,
                name: folder.name.clone(),
                unread,
            });
            for f in self.feeds.iter().filter(|f| f.folder_id == Some(folder.id)) {
                let unread = self
                    .unread
                    .per_feed
                    .iter()
                    .find(|u| u.feed_id == f.id)
                    .map(|u| u.unread)
                    .unwrap_or(0);
                items.push(SidebarItem::Feed {
                    id: f.id,
                    name: f.title.clone(),
                    unread,
                });
            }
        }

        let ungrouped = self.feeds.iter().filter(|f| f.folder_id.is_none()).count();
        if ungrouped > 0 {
            items.push(SidebarItem::Section("Feeds"));
            for f in self.feeds.iter().filter(|f| f.folder_id.is_none()) {
                let unread = self
                    .unread
                    .per_feed
                    .iter()
                    .find(|u| u.feed_id == f.id)
                    .map(|u| u.unread)
                    .unwrap_or(0);
                items.push(SidebarItem::Feed {
                    id: f.id,
                    name: f.title.clone(),
                    unread,
                });
            }
        }

        self.sidebar = items;
        if self.sidebar_index >= self.sidebar.len() {
            self.sidebar_index = self.sidebar.len().saturating_sub(1);
        }
    }

    fn filter_for(sel: &Selection, search: Option<&String>, reader: &RssReader) -> ArticleFilter {
        let mut f = ArticleFilter::default();
        // 订阅排序读共享设置（desc/asc/unread/starred/title）
        if let Ok(Some(v)) = reader.get_setting("sort") {
            f.sort = rss_core::models::ArticleSort::parse(&v);
        }
        match sel {
            Selection::All => {}
            Selection::Unread => f.unread_only = true,
            Selection::Starred => f.starred_only = true,
            Selection::Folder(id) => f.folder_id = Some(*id),
            Selection::Feed(id) => f.feed_id = Some(*id),
        }
        f.search = search.cloned();
        f.limit = 1000;
        f
    }

    pub fn reload_articles(&mut self) {
        let filter = Self::filter_for(&self.selection, self.search.as_ref(), &self.reader);
        self.articles = self.reader.list_articles(&filter).unwrap_or_default();
        if !self.articles.is_empty() {
            self.list_index = self.list_index.min(self.articles.len() - 1);
        } else {
            self.list_index = 0;
        }
    }

    pub fn reload_reader(&mut self) {
        let id = self.articles.get(self.list_index).map(|a| a.id);
        if id == self.current_article_id && !self.reader_text.is_empty() {
            return;
        }
        self.current_article_id = id;
        self.reader_scroll = 0;
        self.reader_text = match id {
            Some(aid) => self.reader.article_to_text(aid).ok().flatten().unwrap_or_default(),
            None => String::new(),
        };
    }

    // ---------- 刷新 ----------

    pub fn spawn_refresh(&mut self, feed_id: Option<i64>) {
        if self.refreshing {
            return;
        }
        self.refreshing = true;
        self.status = if feed_id.is_some() {
            t("refreshing_feed")
        } else {
            t("refreshing_all")
        };
        self.last_refresh = Some(Instant::now());
        let reader = Arc::clone(&self.reader);
        let tx = self.refresh_tx.clone();
        std::thread::spawn(move || {
            let result = match feed_id {
                Some(id) => {
                    let n = reader.refresh_feed(id, false).unwrap_or(0);
                    RefreshResult {
                        feeds_checked: 1,
                        articles_new: n,
                        errors: vec![],
                    }
                }
                None => reader.refresh_all(false).unwrap_or_default(),
            };
            let _ = tx.send(result);
        });
    }

    pub fn rename_folder_ui(&mut self, id: i64, name: &str) {
        if name.is_empty() {
            self.status = t("rename_cancelled");
            return;
        }
        match self.reader.rename_folder(id, name) {
            Ok(()) => self.status = fmt("renamed", &[("name", name)]),
            Err(e) => self.status = fmt("rename_failed", &[("e", &e.to_string())]),
        }
        self.current_article_id = None;
        self.reload();
    }

    pub fn move_feed_ui(&mut self, feed_id: i64, folder_name: &str) {
        let target = if folder_name.trim().is_empty() {
            None
        } else {
            match self.reader.add_or_get_folder(folder_name.trim()) {
                Ok(id) => Some(id),
                Err(e) => {
                    self.status = fmt("move_failed", &[("e", &e.to_string())]);
                    return;
                }
            }
        };
        match self.reader.set_feed_folder(feed_id, target) {
            Ok(()) => {
                let label = match target {
                    Some(id) => self
                        .folders
                        .iter()
                        .find(|f| f.id == id)
                        .map(|f| f.name.clone())
                        .unwrap_or_else(|| folder_name.to_string()),
                    None => "(ungrouped)".to_string(),
                };
                self.status = fmt("moved", &[("label", &label)]);
            }
            Err(e) => self.status = fmt("move_failed", &[("e", &e.to_string())]),
        }
        self.current_article_id = None;
        self.reload();
    }

    pub fn tick(&mut self) {
        // 处理刷新结果
        while let Ok(result) = self.refresh_rx.try_recv() {
            self.refreshing = false;
            let base = fmt("refreshed", &[("n", &result.articles_new.to_string())]);
            self.status = if result.errors.is_empty() {
                base
            } else {
                format!("{base}{}", fmt("errors", &[("n", &result.errors.len().to_string())]))
            };
            self.reload();
        }
        // 定时自动刷新（间隔读全局设置，默认 30 分钟）
        if let Some(last) = self.last_refresh {
            let interval_secs = self.reader.global_refresh_interval() * 60;
            if last.elapsed().as_secs() >= interval_secs as u64 {
                self.spawn_refresh(None);
            }
        }
    }

    // ---------- 操作 ----------

    pub fn open_article(&mut self) {
        let Some(article) = self.articles.get(self.list_index).cloned() else {
            return;
        };
        if !article.is_read {
            let _ = self.reader.mark_read(&[article.id], true);
        }
        self.current_article_id = None;
        self.focus = Focus::Reader;
        self.reload();
    }

    pub fn mark_current_read(&mut self, read: bool) {
        if let Some(a) = self.articles.get(self.list_index) {
            let _ = self.reader.mark_read(&[a.id], read);
            self.current_article_id = None;
            self.reload();
        }
    }

    pub fn toggle_star_current(&mut self) {
        if let Some(a) = self.articles.get(self.list_index) {
            let starred = self.reader.toggle_star(a.id).unwrap_or(false);
            self.status = if starred {
                t("starred_ok")
            } else {
                t("unstarred_ok")
            };
            self.current_article_id = None;
            self.reload();
        }
    }

    pub fn fetch_full_current(&mut self) {
        if let Some(a) = self.articles.get(self.list_index) {
            let ok = self.reader.fetch_article_full_content(a.id).unwrap_or(false);
            self.status = if ok {
                t("full_fetched")
            } else {
                t("no_full")
            };
            self.current_article_id = None;
            self.reload();
        }
    }

    pub fn mark_all_read_in_selection(&mut self) {
        let res = match &self.selection {
            Selection::Folder(id) => self.reader.mark_folder_read(*id),
            Selection::Feed(id) => self.reader.mark_feed_read(*id),
            _ => self.reader.mark_all_read(),
        };
        if res.is_ok() {
            self.status = t("marked_all_read");
            self.current_article_id = None;
            self.reload();
        }
    }

    pub fn delete_current(&mut self) {
        let target = self
            .sidebar
            .get(self.sidebar_index)
            .and_then(|s| s.selection());
        match target {
            Some(Selection::Feed(id)) => {
                let _ = self.reader.remove_feed(id);
                self.status = t("feed_removed");
            }
            Some(Selection::Folder(id)) => {
                let _ = self.reader.remove_folder(id);
                self.status = t("folder_removed");
            }
            _ => return,
        }
        self.selection = Selection::All;
        self.current_article_id = None;
        self.reload();
    }

    pub fn add_feed(&mut self, url: &str, folder_id: Option<i64>) {
        match self.reader.add_feed(url, folder_id, false) {
            Ok(res) => {
                self.status = fmt(
                    "feed_added",
                    &[("title", &res.feed.title), ("n", &res.articles_new.to_string())],
                );
            }
            Err(e) => {
                self.status = fmt("feed_add_failed", &[("e", &e.to_string())]);
            }
        }
        self.current_article_id = None;
        self.reload();
    }

    pub fn add_folder(&mut self, name: &str) {
        match self.reader.add_folder(name) {
            Ok(id) => {
                self.selection = Selection::Folder(id);
                self.status = fmt("folder_created", &[("name", name)]);
            }
            Err(e) => self.status = fmt("folder_failed", &[("e", &e.to_string())]),
        }
        self.current_article_id = None;
        self.reload();
    }

    pub fn import_opml(&mut self, path: &str) {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                self.status = fmt("import_failed", &[("e", &e.to_string())]);
                return;
            }
        };
        match self.reader.import_opml(&content) {
            Ok(res) => {
                self.status = fmt(
                    "imported",
                    &[
                        ("a", &res.feeds_added.to_string()),
                        ("e", &res.feeds_existing.to_string()),
                        ("err", &res.errors.len().to_string()),
                    ],
                );
            }
            Err(e) => self.status = fmt("import_failed", &[("e", &e.to_string())]),
        }
        self.current_article_id = None;
        self.reload();
    }

    pub fn export_opml(&mut self, path: &str) {
        match self.reader.export_opml() {
            Ok(xml) => match std::fs::write(path, xml) {
                Ok(_) => self.status = fmt("exported", &[("path", path)]),
                Err(e) => self.status = fmt("export_failed", &[("e", &e.to_string())]),
            },
            Err(e) => self.status = fmt("export_failed", &[("e", &e.to_string())]),
        }
    }
}

pub fn open_in_browser(url: &str) {
    let cmd = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "cmd"
    } else {
        "xdg-open"
    };
    let _ = match cmd {
        "cmd" => std::process::Command::new(cmd)
            .args(["/c", "start", "", url])
            .spawn(),
        other => std::process::Command::new(other).arg(url).spawn(),
    };
}

pub fn short_date(dt: &chrono::DateTime<chrono::Utc>) -> String {
    use chrono::{Datelike, Timelike};
    let local = dt.with_timezone(&chrono::Local);
    format!("{:02}-{:02} {:02}:{:02}", local.month(), local.day(), local.hour(), local.minute())
}
