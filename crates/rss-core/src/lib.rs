pub mod config;
pub mod error;
pub mod image;
pub mod models;

mod feed;
mod fetch;
mod opml;
mod storage;

#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::sync::RwLock;
use std::time::Duration;

use chrono::Utc;
use reqwest::blocking::Client;
use reqwest::header::{IF_MODIFIED_SINCE, IF_NONE_MATCH};

pub use crate::error::{Error, Result};
pub use crate::feed::PageResource;
pub use crate::image::FetchedImage;
pub use crate::models::{
    Article, ArticleFilter, Feed, FeedUnread, Folder, UnreadStats,
};

use crate::feed::{fetch_full_content, parse_feed_body, NewFeed};
use crate::storage::Storage;

/// 模拟浏览器 UA：不少站点（CSDN 等）对非浏览器 UA 反爬拦截，导致部分文章抓全文失败。
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct RefreshResult {
    pub feeds_checked: usize,
    pub articles_new: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AddFeedResult {
    pub feed: Feed,
    pub articles_new: usize,
    pub existed: bool,
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct OpmlImportResult {
    pub feeds_added: usize,
    pub feeds_existing: usize,
    pub errors: Vec<String>,
}

/// 抓取一次 feed 的中间结果
struct FetchOutcome {
    not_modified: bool,
    feed: Option<NewFeed>,
    etag: Option<String>,
    last_modified: Option<String>,
    error: Option<String>,
}

pub struct RssReader {
    storage: Storage,
    client: RwLock<Client>,
    /// 直连客户端（无代理）：供 `use_proxy=false` 的源使用，不跟随全局代理。
    direct_client: RwLock<Client>,
    data_dir: PathBuf,
}

/// 构建 HTTP 客户端。`proxy` 为 `None` 时直连；支持 `http://`、`https://`、`socks5://` 等。
fn build_client(proxy: Option<&str>) -> Result<Client> {
    use reqwest::header::{ACCEPT, ACCEPT_LANGUAGE};
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        ACCEPT,
        reqwest::header::HeaderValue::from_static(
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8",
        ),
    );
    headers.insert(
        ACCEPT_LANGUAGE,
        reqwest::header::HeaderValue::from_static("zh-CN,zh;q=0.9,en;q=0.8"),
    );
    let mut builder = Client::builder()
        .user_agent(USER_AGENT)
        .default_headers(headers)
        .timeout(Duration::from_secs(30))
        .gzip(true)
        .brotli(true)
        .deflate(true);
    if let Some(proxy_url) = proxy {
        let proxy = reqwest::Proxy::all(proxy_url)
            .map_err(|e| Error::Invalid(format!("invalid proxy: {e}")))?;
        builder = builder.proxy(proxy);
    }
    Ok(builder.build()?)
}

impl RssReader {
    /// 默认数据目录：`~/.local/share/rss-reader/`（Linux），其他平台遵循 dirs 约定。
    pub fn new() -> Result<Self> {
        let dir = dirs::data_dir()
            .map(|d| d.join("rss-reader"))
            .unwrap_or_else(|| PathBuf::from("rss-reader"));
        Self::with_data_dir(dir)
    }

    /// 指定数据目录（内含 rss.db）。
    pub fn with_data_dir(dir: PathBuf) -> Result<Self> {
        let db_path = dir.join("rss.db");
        Self::with_db_path(db_path)
    }

    /// 直接指定数据库文件路径。
    pub fn with_db_path(path: PathBuf) -> Result<Self> {
        let storage = Storage::open(&path)?;
        let proxy = storage.get_setting("proxy")?.unwrap_or_default();
        let proxy_str: Option<&str> = Some(proxy.as_str()).filter(|s| !s.trim().is_empty());
        let client = build_client(proxy_str)?;
        let data_dir = path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        Ok(RssReader {
            storage,
            client: RwLock::new(client),
            direct_client: RwLock::new(build_client(None)?),
            data_dir,
        })
    }

    // ---------- settings ----------

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        self.storage.get_setting(key)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.storage.set_setting(key, value)
    }

    /// 当前代理地址（`None` 表示直连）。
    pub fn get_proxy(&self) -> Result<Option<String>> {
        let proxy = self.storage.get_setting("proxy")?.unwrap_or_default();
        Ok(Some(proxy).filter(|s| !s.trim().is_empty()))
    }

    /// 设置代理并重建 HTTP 客户端（立即生效）。传空串或 `None` 表示直连。
    pub fn set_proxy(&self, proxy: Option<&str>) -> Result<()> {
        let value = proxy.unwrap_or("").trim();
        self.storage.set_setting("proxy", value)?;
        let client = build_client(Some(value).filter(|s| !s.is_empty()))?;
        *self.client.write().unwrap() = client;
        Ok(())
    }

    /// 清除全部已抓取的网页全文缓存（保留标题/摘要/已读/星标等元数据）。
    pub fn clear_content_cache(&self) -> Result<usize> {
        self.storage.clear_content_cache()
    }

    // ---------- folders ----------

    pub fn add_folder(&self, name: &str) -> Result<i64> {
        if name.trim().is_empty() {
            return Err(Error::Invalid("folder name is empty".into()));
        }
        self.storage.add_folder(name.trim())
    }

    /// 同名文件夹已存在则返回其 id，否则新建（供 OPML 导入 / TUI 移动 feed 等场景）。
    pub fn add_or_get_folder(&self, name: &str) -> Result<i64> {
        if name.trim().is_empty() {
            return Err(Error::Invalid("folder name is empty".into()));
        }
        self.storage.add_or_get_folder(name.trim())
    }

    pub fn rename_folder(&self, id: i64, name: &str) -> Result<()> {
        self.storage.rename_folder(id, name)
    }

    pub fn remove_folder(&self, id: i64) -> Result<()> {
        self.storage.remove_folder(id)
    }

    pub fn list_folders(&self) -> Result<Vec<Folder>> {
        self.storage.list_folders()
    }

    // ---------- feeds ----------

    pub fn add_feed(
        &self,
        url: &str,
        folder_id: Option<i64>,
        fetch_full: bool,
    ) -> Result<AddFeedResult> {
        let url = url.trim();
        if url.is_empty() {
            return Err(Error::Invalid("feed url is empty".into()));
        }
        let existed = self.storage.get_feed_by_url(url)?.is_some();
        let outcome = self.fetch_feed(&self.client.read().unwrap(), url, None, None)?;
        if let Some(e) = outcome.error {
            return Err(Error::Fetch(e));
        }
        let parsed = outcome.feed.ok_or_else(|| {
            Error::FeedParse("unexpected 304 Not Modified on first fetch".into())
        })?;

        let folder = if folder_id.is_some() {
            folder_id
        } else {
            self.storage.get_feed_by_url(url)?.and_then(|f| f.folder_id)
        };

        let feed_id = self.storage.insert_feed(
            folder,
            &parsed.title,
            &parsed.url,
            parsed.site_url.as_deref(),
            parsed.description.as_deref(),
            parsed.favicon_url.as_deref(),
            outcome.etag.as_deref(),
            outcome.last_modified.as_deref(),
        )?;
        self.storage.set_feed_error(feed_id, None)?;

        let inserted = self.storage.insert_articles(feed_id, &parsed.entries)?;

        if fetch_full {
            self.backfill_full_content(feed_id, &self.direct_client.read().unwrap())?;
        }

        let feed = self
            .storage
            .get_feed(feed_id)?
            .ok_or_else(|| Error::NotFound("feed".into()))?;
        Ok(AddFeedResult {
            feed,
            articles_new: inserted,
            existed,
        })
    }

    pub fn list_feeds(&self) -> Result<Vec<Feed>> {
        self.storage.list_feeds()
    }

    pub fn remove_feed(&self, id: i64) -> Result<()> {
        self.storage.remove_feed(id)
    }

    pub fn set_feed_folder(&self, feed_id: i64, folder_id: Option<i64>) -> Result<()> {
        self.storage.set_feed_folder(feed_id, folder_id)
    }

    pub fn rename_feed(&self, feed_id: i64, title: &str) -> Result<()> {
        if title.trim().is_empty() {
            return Err(Error::Invalid("feed title is empty".into()));
        }
        self.storage.rename_feed(feed_id, title.trim())
    }

    /// 设置某订阅源的独立抓取频率（分钟）；None=跟随全局。
    pub fn set_feed_refresh_interval(&self, feed_id: i64, minutes: Option<i64>) -> Result<()> {
        self.storage.set_feed_refresh_interval(feed_id, minutes)
    }

    /// 设置某订阅源是否强制走代理（true=代理，false=直连不跟随全局）。
    pub fn set_feed_use_proxy(&self, feed_id: i64, use_proxy: bool) -> Result<()> {
        self.storage.set_feed_use_proxy(feed_id, use_proxy)
    }

    /// 设置某订阅源是否默认"应用内阅读原文"。
    pub fn set_feed_default_original(&self, feed_id: i64, on: bool) -> Result<()> {
        self.storage.set_feed_default_original(feed_id, on)
    }

    /// 修改订阅源地址（同时清掉 error/etag 以便重抓）。
    pub fn update_feed_url(&self, feed_id: i64, url: &str) -> Result<()> {
        let url = url.trim();
        if url.is_empty() {
            return Err(Error::Invalid("feed url is empty".into()));
        }
        self.storage.update_feed_url(feed_id, url)
    }

    /// 清理 N 天前的旧文章。`include_unread=false` 时保留未读。
    pub fn prune_articles(&self, days: i64, include_unread: bool) -> Result<usize> {
        self.storage.prune_articles(days, include_unread)
    }

    /// 用当前代理/直连实测外网连通性（抓 https://example.com）。
    pub fn test_connection(&self) -> Result<String> {
        let start = std::time::Instant::now();
        let resp = self.client.read().unwrap().get("https://example.com").send()?;
        let ms = start.elapsed().as_millis();
        if resp.status().is_success() {
            Ok(format!("OK {}ms (HTTP {})", ms, resp.status().as_u16()))
        } else {
            Ok(format!("HTTP {} ({}ms)", resp.status().as_u16(), ms))
        }
    }

    /// 全局抓取频率（分钟），默认 30。
    pub fn global_refresh_interval(&self) -> i64 {
        self.get_setting("refresh_interval")
            .ok()
            .flatten()
            .and_then(|s| s.parse().ok())
            .filter(|n| *n > 0)
            .unwrap_or(30)
    }

    pub fn refresh_feed(&self, id: i64, fetch_full: bool) -> Result<usize> {
        let feed = self
            .storage
            .get_feed(id)?
            .ok_or_else(|| Error::NotFound("feed".into()))?;
        let client = self.client_for(&feed);
        tracing::info!(target: "rss_core::refresh", "[refresh_feed] feed_id={id} url={} fetch_full={fetch_full} use_proxy={}", feed.url, feed.use_proxy);
        let (etag, last_modified) = self.storage.get_feed_headers(id)?;
        let outcome = self.fetch_feed(&client, &feed.url, etag.as_deref(), last_modified.as_deref())?;
        if let Some(e) = outcome.error {
            tracing::warn!(target: "rss_core::refresh", "[refresh_feed] feed_id={id} error={e}");
            let _ = self.storage.set_feed_error(id, Some(&e));
            return Ok(0);
        }
        if outcome.not_modified {
            tracing::debug!(target: "rss_core::refresh", "[refresh_feed] feed_id={id} not_modified");
            return Ok(0);
        }
        let parsed = match outcome.feed {
            Some(p) => p,
            None => return Ok(0),
        };
        self.storage.update_feed_meta(
            id,
            &parsed.title,
            parsed.site_url.as_deref(),
            parsed.description.as_deref(),
            outcome.etag.as_deref(),
            outcome.last_modified.as_deref(),
            None,
        )?;
        let inserted = self.storage.insert_articles(id, &parsed.entries)?;
        tracing::info!(target: "rss_core::refresh", "[refresh_feed] feed_id={id} articles_new={inserted}");
        if fetch_full {
            let n = self.backfill_full_content(id, &client)?;
            tracing::info!(target: "rss_core::refresh", "[refresh_feed] feed_id={id} backfill_fetched={n}");
        }
        Ok(inserted)
    }

    pub fn refresh_all(&self, fetch_full: bool) -> Result<RefreshResult> {
        let feeds = self.storage.list_feeds()?;
        let global_interval = self.global_refresh_interval();
        let now = Utc::now();
        let mut result = RefreshResult::default();
        let mut skipped = 0usize;
        for feed in feeds {
            // 频率控制：未到刷新时间则跳过（源单独设了用源的，否则用全局）
            let interval = feed.refresh_interval.unwrap_or(global_interval);
            if let Some(last) = feed.last_updated {
                let elapsed_min = (now - last).num_minutes();
                if elapsed_min < interval {
                    skipped += 1;
                    continue;
                }
            }
            match self.refresh_feed(feed.id, fetch_full) {
                Ok(n) => {
                    result.feeds_checked += 1;
                    result.articles_new += n;
                }
                Err(e) => {
                    result.errors.push(format!("{}: {e}", feed.title));
                    let _ = self.storage.set_feed_error(feed.id, Some(&e.to_string()));
                }
            }
        }
        tracing::info!(
            target: "rss_core::refresh",
            "[refresh_all] done checked={} new={} skipped={} errors={}",
            result.feeds_checked, result.articles_new, skipped, result.errors.len()
        );
        Ok(result)
    }

    /// 该源抓取用哪个 client：`use_proxy=false` 强制直连，否则用全局（可能带代理）。
    fn client_for(&self, feed: &models::Feed) -> reqwest::blocking::Client {
        if !feed.use_proxy {
            self.direct_client.read().unwrap().clone()
        } else {
            self.client.read().unwrap().clone()
        }
    }

    /// 条件 GET：带上 ETag / Last-Modified，若服务端返回 304 则跳过解析。
    fn fetch_feed(
        &self,
        client: &reqwest::blocking::Client,
        url: &str,
        etag: Option<&str>,
        last_modified: Option<&str>,
    ) -> Result<FetchOutcome> {
        let mut req = client.get(url);
        if let Some(etag) = etag {
            req = req.header(IF_NONE_MATCH, etag);
        }
        if let Some(lm) = last_modified {
            req = req.header(IF_MODIFIED_SINCE, lm);
        }

        let resp = match req.send() {
            Ok(r) => r,
            Err(e) => {
                return Ok(FetchOutcome {
                    not_modified: false,
                    feed: None,
                    etag: None,
                    last_modified: None,
                    error: Some(e.to_string()),
                });
            }
        };

        if resp.status() == reqwest::StatusCode::NOT_MODIFIED {
            return Ok(FetchOutcome {
                not_modified: true,
                feed: None,
                etag: None,
                last_modified: None,
                error: None,
            });
        }
        if !resp.status().is_success() {
            return Ok(FetchOutcome {
                not_modified: false,
                feed: None,
                etag: None,
                last_modified: None,
                error: Some(format!("HTTP {}", resp.status())),
            });
        }

        let etag = resp
            .headers()
            .get(reqwest::header::ETAG)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let last_modified = resp
            .headers()
            .get(reqwest::header::LAST_MODIFIED)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let body = match resp.text() {
            Ok(b) => b,
            Err(e) => {
                return Ok(FetchOutcome {
                    not_modified: false,
                    feed: None,
                    etag: None,
                    last_modified: None,
                    error: Some(e.to_string()),
                });
            }
        };

        match parse_feed_body(&body, url) {
            Ok(parsed) => Ok(FetchOutcome {
                not_modified: false,
                feed: Some(parsed),
                etag,
                last_modified,
                error: None,
            }),
            Err(e) => Ok(FetchOutcome {
                not_modified: false,
                feed: None,
                etag,
                last_modified,
                error: Some(e.to_string()),
            }),
        }
    }

    /// 对新入库且正文缺失的文章抓取网页全文（跳过已尝试过的）。
    /// 每源最多抓最近 `BACKFILL_LIMIT` 篇，避免网络风暴卡死。
    fn backfill_full_content(&self, feed_id: i64, client: &reqwest::blocking::Client) -> Result<usize> {
        const BACKFILL_LIMIT: usize = 20;
        let filter = ArticleFilter {
            feed_id: Some(feed_id),
            limit: BACKFILL_LIMIT,
            ..ArticleFilter::default()
        };
        let articles = self.storage.list_articles(&filter)?;
        let total = articles.len();
        let mut done = 0usize;
        let mut skipped = 0usize;
        let mut failed = 0usize;
        for a in articles {
            let has_content = a.content.as_deref().map(|c| !c.trim().is_empty()).unwrap_or(false);
            if has_content || a.content_fetched {
                skipped += 1;
                continue;
            }
            let Some(page_url) = a.url.as_deref() else { continue };
            tracing::info!(target: "rss_core::backfill", "[backfill] article_id={} url={page_url}", a.id);
            match fetch_full_content(client, page_url, self.render_fallback_enabled()) {
                Ok(Some(content)) => {
                    let _ = self.storage.update_article_content(a.id, &content);
                    let _ = self.storage.mark_content_fetched(a.id);
                    done += 1;
                }
                Ok(None) => {
                    failed += 1;
                    tracing::warn!(target: "rss_core::backfill", "[backfill] article_id={} fetch_none (反爬/截断/提取短)", a.id);
                }
                Err(e) => {
                    failed += 1;
                    tracing::warn!(target: "rss_core::backfill", "[backfill] article_id={} fetch_err={e}", a.id);
                }
            }
        }
        tracing::info!(
            target: "rss_core::backfill",
            "[backfill] feed_id={feed_id} total={total} fetched={done} skipped={skipped} failed={failed}"
        );
        Ok(done)
    }

    // ---------- articles ----------

    pub fn list_articles(&self, filter: &ArticleFilter) -> Result<Vec<Article>> {
        self.storage.list_articles(filter)
    }

    pub fn get_article(&self, id: i64) -> Result<Option<Article>> {
        self.storage.get_article(id)
    }

    pub fn mark_read(&self, ids: &[i64], read: bool) -> Result<()> {
        self.storage.mark_read(ids, read)
    }

    pub fn mark_feed_read(&self, feed_id: i64) -> Result<()> {
        self.storage.mark_feed_read(feed_id)
    }

    pub fn mark_folder_read(&self, folder_id: i64) -> Result<()> {
        self.storage.mark_folder_read(folder_id)
    }

    pub fn mark_all_read(&self) -> Result<()> {
        self.storage.mark_all_read()
    }

    pub fn toggle_star(&self, id: i64) -> Result<bool> {
        self.storage.toggle_star(id)
    }

    /// 是否启用 headless 浏览器渲染兜底（设置 headless_enabled=1；系统需有 Chromium/Edge/Firefox）。
    fn render_fallback_enabled(&self) -> bool {
        self.get_setting("headless_enabled")
            .ok()
            .flatten()
            .map(|s| s == "1" || s == "true")
            .unwrap_or(false)
    }

    /// 为单篇文章抓取网页全文，成功返回 true。
    pub fn fetch_article_full_content(&self, id: i64) -> Result<bool> {
        let article = self
            .storage
            .get_article(id)?
            .ok_or_else(|| Error::NotFound("article".into()))?;
        let Some(page_url) = article.url.as_deref() else {
            tracing::info!(target: "rss_core::fetch", "[fetch_article] id={id} no_url");
            return Ok(false);
        };
        // 按该源是否走代理选 client
        let feed = self.storage.get_feed(article.feed_id).ok().flatten();
        let use_proxy = feed.as_ref().map(|f| f.use_proxy).unwrap_or(false);
        let client = feed
            .as_ref()
            .map(|f| self.client_for(f))
            .unwrap_or_else(|| self.client.read().unwrap().clone());
        tracing::info!(
            target: "rss_core::fetch",
            "[fetch_article] id={id} url={page_url} use_proxy={use_proxy} feed_id={:?}",
            feed.as_ref().map(|f| f.id)
        );
        match fetch_full_content(&client, page_url, self.render_fallback_enabled()) {
            Ok(Some(content)) => {
                let len = content.len();
                self.storage.update_article_content(id, &content)?;
                // 成功才标记 content_fetched（避免下次重复抓取）；失败不标记，允许重试
                let _ = self.storage.mark_content_fetched(id);
                tracing::info!(target: "rss_core::fetch", "[fetch_article] id={id} ok len={len}");
                Ok(true)
            }
            Ok(None) => {
                tracing::warn!(target: "rss_core::fetch", "[fetch_article] id={id} result_none (反爬/截断/提取<200字)");
                // 失败不永久标记：允许下次打开自动重试（前端会话内防重），避免偶发失败后永远不重抓
                Ok(false)
            }
            Err(e) => {
                tracing::warn!(target: "rss_core::fetch", "[fetch_article] id={id} err={e}");
                Ok(false)
            }
        }
    }

    /// 抓取网页资源：HTML（去 script + 图片归一化）或文件（PDF 等仅标记类型）。
    pub fn fetch_page_resource(&self, url: &str) -> Result<crate::feed::PageResource> {
        crate::feed::fetch_page_resource(&self.client.read().unwrap(), url)
    }

    /// headless 浏览器渲染整页，返回渲染后的完整 HTML（用于 WebView 打不开的站，
    /// 如强制后量子 TLS 的 status.deepseek.com）。剥离 <script>/<link>：iframe srcDoc 的
    /// base 是 about:srcdoc，渲染页里的相对资源（/_next/...）会全部加载失败导致报错页。
    pub fn fetch_page_rendered(&self, url: &str) -> Result<crate::feed::PageResource> {
        let html = crate::fetch::render::render_dom(url)
            .ok_or_else(|| Error::Invalid("no headless-renderable browser available (need Chromium/Edge/Firefox)".into()))?;
        let html = strip_scripts_and_links(&html);
        Ok(crate::feed::PageResource {
            kind: "html".into(),
            content_type: "text/html".into(),
            content: html,
            allow_embed: true,
        })
    }

    /// 轻量探测资源类型（HTML 还是文件），不抓 body。
    pub fn probe_page_resource(&self, url: &str) -> Result<crate::feed::PageResource> {
        crate::feed::probe_page_resource(&self.client.read().unwrap(), url)
    }

    /// 抓取图片字节（带浏览器 UA + 智能 Referer + 磁盘缓存），供前端渲染。
    /// `referer` 为可选源站 URL（更贴合 CDN 防盗链校验）；无则回退图片 host。
    pub fn fetch_image(&self, url: &str, referer: Option<&str>, max_width: Option<u32>) -> Result<FetchedImage> {
        crate::image::fetch_image(
            &self.client.read().unwrap(),
            &self.data_dir,
            url,
            referer,
            max_width,
        )
    }

    /// 清除图片磁盘缓存。
    pub fn clear_image_cache(&self) {
        crate::image::clear_image_cache(&self.data_dir);
    }

    /// 文章 HTML 转纯文本（供 TUI 渲染）。
    pub fn article_to_text(&self, id: i64) -> Result<Option<String>> {
        let article = match self.storage.get_article(id)? {
            Some(a) => a,
            None => return Ok(None),
        };
        let html = article
            .content
            .or(article.summary)
            .unwrap_or_default();
        Ok(Some(html2md::parse_html(&html)))
    }

    // ---------- stats ----------

    pub fn unread_stats(&self) -> Result<UnreadStats> {
        self.storage.unread_stats()
    }

    // ---------- OPML ----------

    pub fn import_opml(&self, content: &str) -> Result<OpmlImportResult> {
        let outlines = opml::parse_opml(content)?;
        let mut result = OpmlImportResult::default();
        for node in outlines {
            self.import_node(&node, None, &mut result);
        }
        Ok(result)
    }

    /// 递归导入 OPML 节点：有 xml_url → 直接加订阅；无 → 视为分组并递归子节点
    /// （支持嵌套分组，避免旧实现"分组里套分组被跳过"导致导入不全）。
    fn import_node(
        &self,
        node: &opml::OpmlOutline,
        folder_id: Option<i64>,
        result: &mut OpmlImportResult,
    ) {
        if let Some(xml_url) = node.xml_url.as_deref() {
            match self.add_feed(xml_url, folder_id, false) {
                Ok(res) => {
                    if res.existed {
                        result.feeds_existing += 1;
                    } else {
                        result.feeds_added += 1;
                    }
                }
                Err(e) => result.errors.push(format!("{xml_url}: {e}")),
            }
            return;
        }
        let name = node
            .text
            .clone()
            .unwrap_or_else(|| "Imported".to_string());
        let child_folder = match self.storage.add_or_get_folder(&name) {
            Ok(id) => id,
            Err(e) => {
                result.errors.push(format!("folder {name}: {e}"));
                return;
            }
        };
        for child in &node.children {
            self.import_node(child, Some(child_folder), result);
        }
    }

    /// 导出为 OPML 2.0 文本（可写文件或直接返回）。
    pub fn export_opml(&self) -> Result<String> {
        use crate::opml::{FeedEntry, FeedGroup};

        let folders = self.storage.list_folders()?;
        let feeds = self.storage.list_feeds()?;

        let mut groups: Vec<FeedGroup> = folders
            .iter()
            .map(|f| (f.name.clone(), Vec::new()))
            .collect();
        let mut ungrouped: Vec<FeedEntry> = Vec::new();

        for feed in feeds {
            let item = (feed.title.clone(), feed.url.clone(), feed.site_url.clone());
            if let Some(fid) = feed.folder_id {
                if let Some(idx) = folders.iter().position(|f| f.id == fid) {
                    groups[idx].1.push(item);
                    continue;
                }
            }
            ungrouped.push(item);
        }

        opml::export_opml("rss-reader export", &groups, &ungrouped)
    }
}

/// 剥离 <script> 与 <link> 标签（保留 SSR 内容与内联 <style>），避免 iframe srcDoc
/// 里相对资源（/_next/...）在 about:srcdoc base 下全部加载失败报错。
fn strip_scripts_and_links(html: &str) -> String {
    let bytes = html.as_bytes();
    let lower = html.to_ascii_lowercase();
    let mut out = String::with_capacity(html.len());
    let mut i = 0;
    while i < bytes.len() {
        if lower[i..].starts_with("<script") {
            let Some(gt) = lower[i..].find('>') else { break };
            let tag_end = i + gt;
            let is_self_closed = lower[i..tag_end].trim_end().ends_with('/');
            i = tag_end + 1;
            if !is_self_closed {
                if let Some(close) = lower[i..].find("</script") {
                    i += close + "</script".len();
                }
            }
            continue;
        }
        if lower[i..].starts_with("<link") {
            if let Some(gt) = lower[i..].find('>') {
                i += gt + 1;
                continue;
            }
        }
        let cl = crate::fetch::generic::utf8_len(bytes[i]);
        out.push_str(&html[i..i + cl]);
        i += cl;
    }
    out
}
