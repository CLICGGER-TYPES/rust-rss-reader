use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Folder {
    pub id: i64,
    pub name: String,
    pub position: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Feed {
    pub id: i64,
    pub folder_id: Option<i64>,
    pub title: String,
    pub url: String,
    pub site_url: Option<String>,
    pub description: Option<String>,
    pub favicon_url: Option<String>,
    pub last_updated: Option<DateTime<Utc>>,
    pub error: Option<String>,
    /// 该源独立抓取频率（分钟），None 表示跟随全局
    pub refresh_interval: Option<i64>,
    /// 该源抓取是否走代理：true=强制走代理，false=直连（不跟随全局代理）
    pub use_proxy: bool,
    /// 该源默认"应用内阅读原文"（跳过正文视图）：爬虫抓不全的源用
    pub default_original: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Article {
    pub id: i64,
    pub feed_id: i64,
    pub title: Option<String>,
    pub url: Option<String>,
    pub author: Option<String>,
    /// 摘要，HTML
    pub summary: Option<String>,
    /// 正文，HTML
    pub content: Option<String>,
    /// 是否已尝试过抓取全文（无论成败）
    pub content_fetched: bool,
    pub published_at: Option<DateTime<Utc>>,
    pub fetched_at: DateTime<Utc>,
    pub is_read: bool,
    pub is_starred: bool,
    pub guid: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FeedUnread {
    pub feed_id: i64,
    pub folder_id: Option<i64>,
    pub title: String,
    pub unread: i64,
}

/// 文章排序方式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArticleSort {
    /// 时间新到旧
    TimeDesc,
    /// 时间旧到新
    TimeAsc,
    /// 未读优先（同未读再按时间新到旧）
    Unread,
    /// 星标优先
    Starred,
    /// 按标题字母序
    Title,
}

impl ArticleSort {
    pub fn parse(s: &str) -> Self {
        match s {
            "asc" => Self::TimeAsc,
            "unread" => Self::Unread,
            "starred" => Self::Starred,
            "title" => Self::Title,
            _ => Self::TimeDesc,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ArticleFilter {
    pub feed_id: Option<i64>,
    pub folder_id: Option<i64>,
    pub unread_only: bool,
    pub starred_only: bool,
    pub search: Option<String>,
    pub limit: usize,
    pub offset: usize,
    /// 排序方式
    pub sort: ArticleSort,
}

impl Default for ArticleFilter {
    fn default() -> Self {
        Self {
            feed_id: None,
            folder_id: None,
            unread_only: false,
            starred_only: false,
            search: None,
            limit: 200,
            offset: 0,
            sort: ArticleSort::TimeDesc,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct UnreadStats {
    pub total: i64,
    pub per_feed: Vec<FeedUnread>,
}

/// 清理旧文章参数
#[derive(Debug, Clone, Copy)]
pub struct PruneOptions {
    /// 保留天数（N 天之前的删除）
    pub days: i64,
    /// 是否连同未读一起清理
    pub include_unread: bool,
}
