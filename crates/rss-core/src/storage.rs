use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::Mutex;

use crate::error::Result;
use crate::models::{Article, ArticleFilter, Feed, Folder, FeedUnread, UnreadStats};

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS folders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    position INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS feeds (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    folder_id INTEGER REFERENCES folders(id) ON DELETE SET NULL,
    title TEXT NOT NULL,
    url TEXT NOT NULL UNIQUE,
    site_url TEXT,
    description TEXT,
    favicon_url TEXT,
    etag TEXT,
    last_modified TEXT,
    last_updated INTEGER,
    error TEXT,
    refresh_interval INTEGER,
    use_proxy INTEGER NOT NULL DEFAULT 0,
    default_original INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS articles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    feed_id INTEGER NOT NULL REFERENCES feeds(id) ON DELETE CASCADE,
    title TEXT,
    url TEXT,
    author TEXT,
    summary TEXT,
    content TEXT,
    content_fetched INTEGER NOT NULL DEFAULT 0,
    published_at INTEGER,
    fetched_at INTEGER NOT NULL,
    is_read INTEGER NOT NULL DEFAULT 0,
    is_starred INTEGER NOT NULL DEFAULT 0,
    guid TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_articles_feed_guid ON articles(feed_id, guid);
CREATE INDEX IF NOT EXISTS idx_articles_feed_pub ON articles(feed_id, published_at DESC);
CREATE INDEX IF NOT EXISTS idx_articles_read ON articles(is_read);
CREATE INDEX IF NOT EXISTS idx_articles_starred ON articles(is_starred);
CREATE INDEX IF NOT EXISTS idx_feeds_folder ON feeds(folder_id);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT
);
"#;

fn ts(dt: &DateTime<Utc>) -> i64 {
    dt.timestamp()
}

fn from_ts(v: Option<i64>) -> Option<DateTime<Utc>> {
    v.and_then(|x| DateTime::from_timestamp(x, 0))
}

pub(crate) struct Storage {
    conn: Mutex<Connection>,
}

fn feed_from_row(row: &rusqlite::Row) -> rusqlite::Result<Feed> {
    Ok(Feed {
        id: row.get(0)?,
        folder_id: row.get(1)?,
        title: row.get(2)?,
        url: row.get(3)?,
        site_url: row.get(4)?,
        description: row.get(5)?,
        favicon_url: row.get(6)?,
        last_updated: from_ts(row.get(7)?),
        error: row.get(8)?,
        refresh_interval: row.get(9)?,
        use_proxy: row.get::<_, i64>(10)? != 0,
        default_original: row.get::<_, i64>(11)? != 0,
    })
}

const FEED_COLUMNS: &str =
    "id, folder_id, title, url, site_url, description, favicon_url, last_updated, error, refresh_interval, use_proxy, default_original";

fn article_from_row(row: &rusqlite::Row) -> rusqlite::Result<Article> {
    Ok(Article {
        id: row.get(0)?,
        feed_id: row.get(1)?,
        title: row.get(2)?,
        url: row.get(3)?,
        author: row.get(4)?,
        summary: row.get(5)?,
        content: row.get(6)?,
        published_at: from_ts(row.get(7)?),
        fetched_at: DateTime::<Utc>::from_timestamp(row.get(8)?, 0).unwrap_or_else(Utc::now),
        is_read: row.get::<_, i64>(9)? != 0,
        is_starred: row.get::<_, i64>(10)? != 0,
        guid: row.get(11)?,
        content_fetched: row.get::<_, i64>(12)? != 0,
    })
}

const ARTICLE_COLUMNS: &str = "a.id, a.feed_id, a.title, a.url, a.author, a.summary, a.content, \
     a.published_at, a.fetched_at, a.is_read, a.is_starred, a.guid, a.content_fetched";
impl Storage {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(SCHEMA)?;
        // 轻量迁移：老库补 content_fetched 列
        let has_col: bool = conn
            .prepare("SELECT 1 FROM pragma_table_info('articles') WHERE name='content_fetched'")
            .and_then(|mut stmt| stmt.exists([]))
            .unwrap_or(false);
        if !has_col {
            conn.execute_batch(
                "ALTER TABLE articles ADD COLUMN content_fetched INTEGER NOT NULL DEFAULT 0",
            )?;
        }
        // 轻量迁移：老库补 feeds.refresh_interval 列
        let has_fi: bool = conn
            .prepare("SELECT 1 FROM pragma_table_info('feeds') WHERE name='refresh_interval'")
            .and_then(|mut stmt| stmt.exists([]))
            .unwrap_or(false);
        if !has_fi {
            conn.execute_batch("ALTER TABLE feeds ADD COLUMN refresh_interval INTEGER")?;
        }
        // 轻量迁移：老库补 feeds.use_proxy 列
        let has_up: bool = conn
            .prepare("SELECT 1 FROM pragma_table_info('feeds') WHERE name='use_proxy'")
            .and_then(|mut stmt| stmt.exists([]))
            .unwrap_or(false);
        if !has_up {
            conn.execute_batch(
                "ALTER TABLE feeds ADD COLUMN use_proxy INTEGER NOT NULL DEFAULT 0",
            )?;
        }
        // 轻量迁移：老库补 feeds.default_original 列
        let has_do: bool = conn
            .prepare("SELECT 1 FROM pragma_table_info('feeds') WHERE name='default_original'")
            .and_then(|mut stmt| stmt.exists([]))
            .unwrap_or(false);
        if !has_do {
            conn.execute_batch(
                "ALTER TABLE feeds ADD COLUMN default_original INTEGER NOT NULL DEFAULT 0",
            )?;
        }
        // 一次性迁移：重置历史 content_fetched 脏标记（旧逻辑"尝试过/失败也标记"），
        // 语义改为"成功抓取过"——未成功抓取的文章打开时会自动抓全文，不再被残留标记阻塞
        let _ = conn.execute("UPDATE articles SET content_fetched = 0", []);
        Ok(Storage {
            conn: Mutex::new(conn),
        })
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        Ok(conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()?)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO settings(key, value) VALUES(?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    // ---------- folders ----------

    pub fn add_folder(&self, name: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let pos = conn.query_row(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM folders",
            [],
            |r| r.get::<_, i64>(0),
        )?;
        conn.execute(
            "INSERT INTO folders(name, position) VALUES(?1, ?2)",
            params![name, pos],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn add_or_get_folder(&self, name: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        if let Some(id) = conn
            .query_row("SELECT id FROM folders WHERE name = ?1", params![name], |r| {
                r.get::<_, i64>(0)
            })
            .optional()?
        {
            return Ok(id);
        }
        let pos = conn.query_row(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM folders",
            [],
            |r| r.get::<_, i64>(0),
        )?;
        conn.execute(
            "INSERT INTO folders(name, position) VALUES(?1, ?2)",
            params![name, pos],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn rename_folder(&self, id: i64, name: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE folders SET name = ?1 WHERE id = ?2",
            params![name, id],
        )?;
        Ok(())
    }

    pub fn remove_folder(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM folders WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn list_folders(&self) -> Result<Vec<Folder>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, name, position FROM folders ORDER BY position, id")?;
        let rows = stmt.query_map([], |row| {
            Ok(Folder {
                id: row.get(0)?,
                name: row.get(1)?,
                position: row.get(2)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    // ---------- feeds ----------

    pub fn get_feed(&self, id: i64) -> Result<Option<Feed>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(&format!("SELECT {FEED_COLUMNS} FROM feeds WHERE id = ?1"))?;
        let feed = stmt.query_row(params![id], feed_from_row).optional()?;
        Ok(feed)
    }

    pub fn get_feed_by_url(&self, url: &str) -> Result<Option<Feed>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare(&format!("SELECT {FEED_COLUMNS} FROM feeds WHERE url = ?1"))?;
        let feed = stmt.query_row(params![url], feed_from_row).optional()?;
        Ok(feed)
    }

    pub fn list_feeds(&self) -> Result<Vec<Feed>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(&format!(
            "SELECT {FEED_COLUMNS} FROM feeds ORDER BY COALESCE(folder_id, 0), title"
        ))?;
        let rows = stmt.query_map([], feed_from_row)?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn insert_feed(
        &self,
        folder_id: Option<i64>,
        title: &str,
        url: &str,
        site_url: Option<&str>,
        description: Option<&str>,
        favicon_url: Option<&str>,
        etag: Option<&str>,
        last_modified: Option<&str>,
    ) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO feeds(folder_id, title, url, site_url, description, favicon_url, etag, last_modified)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(url) DO UPDATE SET
                folder_id = COALESCE(excluded.folder_id, feeds.folder_id),
                title = excluded.title,
                site_url = COALESCE(excluded.site_url, feeds.site_url),
                description = COALESCE(excluded.description, feeds.description),
                favicon_url = COALESCE(excluded.favicon_url, feeds.favicon_url),
                etag = excluded.etag,
                last_modified = excluded.last_modified",
            params![
                folder_id,
                title,
                url,
                site_url,
                description,
                favicon_url,
                etag,
                last_modified
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_feed_meta(
        &self,
        id: i64,
        title: &str,
        site_url: Option<&str>,
        description: Option<&str>,
        etag: Option<&str>,
        last_modified: Option<&str>,
        error: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE feeds SET title = ?1, site_url = COALESCE(?2, site_url),
                description = COALESCE(?3, description), etag = ?4, last_modified = ?5,
                error = ?6, last_updated = ?7 WHERE id = ?8",
            params![
                title,
                site_url,
                description,
                etag,
                last_modified,
                error,
                ts(&Utc::now()),
                id
            ],
        )?;
        Ok(())
    }

    pub fn set_feed_folder(&self, feed_id: i64, folder_id: Option<i64>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE feeds SET folder_id = ?1 WHERE id = ?2",
            params![folder_id, feed_id],
        )?;
        Ok(())
    }

    pub fn rename_feed(&self, feed_id: i64, title: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE feeds SET title = ?1 WHERE id = ?2",
            params![title, feed_id],
        )?;
        Ok(())
    }

    pub fn set_feed_refresh_interval(&self, feed_id: i64, minutes: Option<i64>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE feeds SET refresh_interval = ?1 WHERE id = ?2",
            params![minutes, feed_id],
        )?;
        Ok(())
    }

    /// 设置该源是否强制走代理（true=代理，false=直连）。
    pub fn set_feed_use_proxy(&self, feed_id: i64, use_proxy: bool) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE feeds SET use_proxy = ?1 WHERE id = ?2",
            params![i64::from(use_proxy), feed_id],
        )?;
        Ok(())
    }

    /// 设置该源是否默认"应用内阅读原文"。
    pub fn set_feed_default_original(&self, feed_id: i64, on: bool) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE feeds SET default_original = ?1 WHERE id = ?2",
            params![i64::from(on), feed_id],
        )?;
        Ok(())
    }

    /// 修改订阅源地址；同时清掉 error/etag/last_modified 以便立即重抓。
    pub fn update_feed_url(&self, feed_id: i64, url: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE feeds SET url = ?1, error = NULL, etag = NULL, last_modified = NULL WHERE id = ?2",
            params![url, feed_id],
        )?;
        Ok(())
    }

    /// 清理 N 天前的旧文章。`include_unread=false` 时保留未读。
    pub fn prune_articles(&self, days: i64, include_unread: bool) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let cutoff = Utc::now().timestamp() - days * 86400;
        let sql = if include_unread {
            "DELETE FROM articles WHERE COALESCE(published_at, fetched_at) < ?1"
        } else {
            "DELETE FROM articles WHERE is_read = 1 AND COALESCE(published_at, fetched_at) < ?1"
        };
        let n = conn.execute(sql, params![cutoff])?;
        Ok(n)
    }

    pub fn set_feed_error(&self, id: i64, error: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE feeds SET error = ?1, last_updated = ?2 WHERE id = ?3",
            params![error, ts(&Utc::now()), id],
        )?;
        Ok(())
    }

    /// 取回 ETag / Last-Modified，用于增量刷新（304 跳过解析）。
    pub fn get_feed_headers(&self, id: i64) -> Result<(Option<String>, Option<String>)> {
        let conn = self.conn.lock().unwrap();
        Ok(conn
            .query_row(
                "SELECT etag, last_modified FROM feeds WHERE id = ?1",
                params![id],
                |r| Ok((r.get::<_, Option<String>>(0)?, r.get::<_, Option<String>>(1)?)),
            )
            .optional()?
            .unwrap_or((None, None)))
    }

    pub fn remove_feed(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM feeds WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ---------- articles ----------

    pub fn insert_articles(&self, feed_id: i64, articles: &[crate::feed::NewArticle]) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let now = ts(&Utc::now());
        let mut inserted = 0usize;
        for a in articles {
            let guid = match a.guid.as_deref() {
                Some(g) if !g.trim().is_empty() => Some(g.trim().to_string()),
                _ => a
                    .url
                    .as_deref()
                    .map(|u| u.trim().to_string())
                    .filter(|u| !u.is_empty()),
            };
            let changed = conn.execute(
                "INSERT INTO articles(feed_id, title, url, author, summary, content, published_at, fetched_at, guid)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(feed_id, guid) DO NOTHING",
                params![
                    feed_id,
                    a.title,
                    a.url,
                    a.author,
                    a.summary,
                    a.content,
                    a.published_at.map(|d| ts(&d)),
                    now,
                    guid
                ],
            )?;
            inserted += changed;
        }
        Ok(inserted)
    }

    pub fn list_articles(&self, filter: &ArticleFilter) -> Result<Vec<Article>> {
        let mut sql = format!(
            "SELECT {ARTICLE_COLUMNS}
             FROM articles a
             INNER JOIN feeds f ON f.id = a.feed_id
             WHERE 1 = 1"
        );
        let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        let mut push = |arg: Box<dyn rusqlite::ToSql>| args.push(arg);

        if let Some(feed_id) = filter.feed_id {
            sql.push_str(" AND a.feed_id = ?");
            push(Box::new(feed_id));
        }
        if let Some(folder_id) = filter.folder_id {
            sql.push_str(" AND f.folder_id = ?");
            push(Box::new(folder_id));
        }
        if filter.unread_only {
            sql.push_str(" AND a.is_read = 0");
        }
        if filter.starred_only {
            sql.push_str(" AND a.is_starred = 1");
        }
        if let Some(search) = &filter.search {
            sql.push_str(" AND (a.title LIKE ? OR a.summary LIKE ? OR a.content LIKE ?)");
            let pat = format!("%{}%", search);
            push(Box::new(pat.clone()));
            push(Box::new(pat.clone()));
            push(Box::new(pat));
        }
        let order_by = match filter.sort {
            crate::models::ArticleSort::TimeAsc => "COALESCE(a.published_at, a.fetched_at) ASC",
            crate::models::ArticleSort::Unread => "a.is_read ASC, COALESCE(a.published_at, a.fetched_at) DESC",
            crate::models::ArticleSort::Starred => "a.is_starred DESC, COALESCE(a.published_at, a.fetched_at) DESC",
            crate::models::ArticleSort::Title => "a.title COLLATE NOCASE ASC",
            _ => "COALESCE(a.published_at, a.fetched_at) DESC",
        };
        sql.push_str(&format!(" ORDER BY {order_by} LIMIT ? OFFSET ?"));
        push(Box::new(filter.limit as i64));
        push(Box::new(filter.offset as i64));

        let params: Vec<&dyn rusqlite::ToSql> = args.iter().map(|b| b.as_ref()).collect();
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params.as_slice(), article_from_row)?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn get_article(&self, id: i64) -> Result<Option<Article>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(&format!(
            "SELECT {ARTICLE_COLUMNS} FROM articles a WHERE a.id = ?1"
        ))?;
        let article = stmt.query_row(params![id], article_from_row).optional()?;
        Ok(article)
    }

    pub fn mark_read(&self, article_ids: &[i64], read: bool) -> Result<()> {
        if article_ids.is_empty() {
            return Ok(());
        }
        let placeholders = article_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!("UPDATE articles SET is_read = ?1 WHERE id IN ({placeholders})");
        let mut args: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(read as i64)];
        for id in article_ids {
            args.push(Box::new(*id));
        }
        let params: Vec<&dyn rusqlite::ToSql> = args.iter().map(|b| b.as_ref()).collect();
        let conn = self.conn.lock().unwrap();
        conn.execute(&sql, params.as_slice())?;
        Ok(())
    }

    pub fn mark_feed_read(&self, feed_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE articles SET is_read = 1 WHERE feed_id = ?1 AND is_read = 0",
            params![feed_id],
        )?;
        Ok(())
    }

    pub fn mark_folder_read(&self, folder_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE articles SET is_read = 1
             WHERE feed_id IN (SELECT id FROM feeds WHERE folder_id = ?1) AND is_read = 0",
            params![folder_id],
        )?;
        Ok(())
    }

    pub fn mark_all_read(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("UPDATE articles SET is_read = 1 WHERE is_read = 0", [])?;
        Ok(())
    }

    pub fn toggle_star(&self, id: i64) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let cur: bool = conn
            .query_row(
                "SELECT is_starred FROM articles WHERE id = ?1",
                params![id],
                |r| Ok(r.get::<_, i64>(0)? != 0),
            )
            .optional()?
            .unwrap_or(false);
        let next = !cur;
        conn.execute(
            "UPDATE articles SET is_starred = ?1 WHERE id = ?2",
            params![next as i64, id],
        )?;
        Ok(next)
    }

    pub fn update_article_content(&self, id: i64, content: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE articles SET content = ?1, content_fetched = 1 WHERE id = ?2",
            params![content, id],
        )?;
        Ok(())
    }


    /// 清空所有已抓取的全文（content 字段），返回清除的条数。
    /// 标记某篇文章已成功抓取过全文（避免重复抓取）。失败不标记，允许下次重试。
    pub fn mark_content_fetched(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE articles SET content_fetched = 1 WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    /// 清缓存：整个软件清干净，只剩最基本（订阅源/分组/设置）。
    /// - 清空全部文章（含正文/摘要/已读星标，可整体重抓）
    /// - 清 feeds 的 HTTP 增量缓存（etag/last_modified）与刷新时间/错误，
    ///   使下一次强制刷新全量重抓所有源（不再 304/频率跳过）
    ///
    /// 返回清掉的文章数。
    pub fn clear_content_cache(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let n = conn.execute("DELETE FROM articles", [])?;
        conn.execute(
            "UPDATE feeds SET etag = NULL, last_modified = NULL, last_updated = NULL, error = NULL",
            [],
        )?;
        Ok(n)
    }

    pub fn unread_stats(&self) -> Result<UnreadStats> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT f.id, f.folder_id, f.title,
                    SUM(CASE WHEN a.is_read = 0 THEN 1 ELSE 0 END) AS unread
             FROM feeds f
             LEFT JOIN articles a ON a.feed_id = f.id
             GROUP BY f.id, f.folder_id, f.title",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(FeedUnread {
                feed_id: row.get(0)?,
                folder_id: row.get(1)?,
                title: row.get(2)?,
                unread: row.get::<_, Option<i64>>(3)?.unwrap_or(0),
            })
        })?;
        let per_feed: Vec<FeedUnread> = rows.collect::<std::result::Result<_, _>>()?;
        let total = per_feed.iter().map(|f| f.unread).sum();
        Ok(UnreadStats { total, per_feed })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_cache_really_clears() {
        use crate::image;
        let dir = std::env::temp_dir().join(format!("rss-clear-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("rss.db");
        let storage = Storage::open(&db).unwrap();
        {
            let conn = storage.conn.lock().unwrap();
            conn.execute(
                "INSERT INTO feeds(folder_id,title,url,etag,last_modified,last_updated,error) \
                 VALUES(NULL,'F','https://example.com/f','abc','def',strftime('%s','now'),'boom')",
                [],
            )
            .unwrap();
            let fid = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO articles(feed_id,title,url,summary,content,content_fetched,fetched_at) \
                 VALUES(?1,'A','https://example.com/a','s','<p>full</p>',1,strftime('%s','now'))",
                params![fid],
            )
            .unwrap();
        }
        let cache = dir.join("img_cache");
        std::fs::create_dir_all(&cache).unwrap();
        std::fs::write(cache.join("abc.bin"), vec![1, 2, 3]).unwrap();
        std::fs::write(cache.join("abc.type"), "image/png").unwrap();
        assert!(cache.join("abc.bin").exists());

        let n = storage.clear_content_cache().unwrap();
        assert_eq!(n, 1, "应清掉 1 篇文章");
        let conn = storage.conn.lock().unwrap();
        let art_left: i64 = conn
            .query_row("SELECT COUNT(*) FROM articles", [], |r| r.get(0))
            .unwrap();
        assert_eq!(art_left, 0, "文章应全部清空");
        let feed_cache: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM feeds WHERE etag IS NOT NULL OR last_modified IS NOT NULL OR last_updated IS NOT NULL OR error IS NOT NULL",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(feed_cache, 0, "feeds 的 etag/last_modified/last_updated/error 应清空");
        drop(conn);
        image::clear_image_cache(&dir);
        assert!(!cache.exists(), "img_cache 目录应被删除");

        let _ = std::fs::remove_dir_all(&dir);
    }
}

    #[test]
    fn mark_content_fetched_success_semantics() {
        let dir = std::env::temp_dir().join(format!("rss-mcf-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("rss.db");
        let storage = Storage::open(&db).unwrap();
        let conn = storage.conn.lock().unwrap();
        conn.execute("INSERT INTO feeds(folder_id,title,url) VALUES(NULL,'F','https://e.com/f')", []).unwrap();
        let fid = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO articles(feed_id,title,url,summary,content,fetched_at) VALUES(?1,'A','https://e.com/a','s','',strftime('%s','now'))",
            params![fid],
        ).unwrap();
        let aid = conn.last_insert_rowid();
        drop(conn);
        // 未标记前 = 0
        let before: i64 = storage
            .conn.lock().unwrap()
            .query_row("SELECT content_fetched FROM articles WHERE id=?1", params![aid], |r| r.get(0)).unwrap();
        assert_eq!(before, 0, "默认未标记");
        storage.mark_content_fetched(aid).unwrap();
        let after: i64 = storage
            .conn.lock().unwrap()
            .query_row("SELECT content_fetched FROM articles WHERE id=?1", params![aid], |r| r.get(0)).unwrap();
        assert_eq!(after, 1, "mark 后 content_fetched=1");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_articles_filters_by_feed_folder_and_flags() {
        use crate::models::ArticleFilter;
        let dir = std::env::temp_dir().join(format!("rss-filter-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("rss.db");
        let storage = Storage::open(&db).unwrap();
        {
            let conn = storage.conn.lock().unwrap();
            conn.execute("INSERT INTO folders(name) VALUES('Tech')", []).unwrap();
            let fid = conn.last_insert_rowid();
            conn.execute("INSERT INTO feeds(folder_id,title,url) VALUES(?1,'F1','https://f1')", params![fid]).unwrap();
            let f1 = conn.last_insert_rowid();
            conn.execute("INSERT INTO feeds(folder_id,title,url) VALUES(?1,'F2','https://f2')", params![fid]).unwrap();
            let f2 = conn.last_insert_rowid();
            conn.execute("INSERT INTO feeds(folder_id,title,url) VALUES(NULL,'F3','https://f3')", []).unwrap();
            let f3 = conn.last_insert_rowid();
            let ins = |feed: i64, title: &str, read: i64, star: i64| {
                conn.execute(
                    "INSERT INTO articles(feed_id,title,url,summary,content,fetched_at,is_read,is_starred) \
                     VALUES(?1,?2,?3,'s','',strftime('%s','now'),?4,?5)",
                    params![feed, title, format!("https://{title}"), read, star],
                ).unwrap();
            };
            ins(f1, "A1", 0, 0);
            ins(f1, "A2", 1, 0);
            ins(f2, "B1", 0, 1);
            ins(f3, "C1", 0, 0);
        }
        let f = |filter: ArticleFilter| storage.list_articles(&filter).unwrap().iter().map(|a| a.title.clone().unwrap_or_default()).collect::<Vec<_>>();
        // feed 过滤
        let by_feed = f(ArticleFilter { feed_id: Some(1), ..ArticleFilter::default() });
        assert_eq!(by_feed, vec!["A1".to_string(), "A2".to_string()], "feed_id 过滤");
        // folder 过滤
        let by_folder = f(ArticleFilter { folder_id: Some(1), ..ArticleFilter::default() });
        assert_eq!(by_folder.len(), 3, "folder 过滤（F1+F2 的文章）");
        // 未读
        let by_unread = f(ArticleFilter { unread_only: true, ..ArticleFilter::default() });
        assert_eq!(by_unread.len(), 3, "unread_only 过滤");
        assert!(by_unread.iter().all(|t| t != "A2"), "已读文章被排除");
        // 星标
        let by_star = f(ArticleFilter { starred_only: true, ..ArticleFilter::default() });
        assert_eq!(by_star, vec!["B1".to_string()], "starred_only 过滤");
        let _ = std::fs::remove_dir_all(&dir);
    }
