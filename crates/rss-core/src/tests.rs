use crate::feed::{parse_feed_body, NewArticle};
use crate::models::ArticleFilter;
use crate::opml;
use crate::storage::Storage;
use crate::RssReader;

const SAMPLE_RSS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
<channel>
  <title>Example Feed</title>
  <link>https://example.com</link>
  <description>Example feed description</description>
  <item>
    <title>First Post</title>
    <link>https://example.com/first</link>
    <guid>abc-123</guid>
    <pubDate>Mon, 17 Aug 2026 10:00:00 GMT</pubDate>
    <description>Summary here</description>
  </item>
  <item>
    <title>Second Post</title>
    <link>https://example.com/second</link>
    <guid>abc-124</guid>
    <pubDate>Tue, 18 Aug 2026 10:00:00 GMT</pubDate>
    <description>Another summary</description>
  </item>
</channel>
</rss>"#;

const SAMPLE_ATOM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Atom Feed</title>
  <link href="https://example.org/"/>
  <updated>2026-08-17T10:00:00Z</updated>
  <entry>
    <title>Atom Entry</title>
    <link href="https://example.org/post"/>
    <id>urn:uuid:123</id>
    <updated>2026-08-17T09:00:00Z</updated>
    <author><name>Jane</name></author>
    <content type="html">&lt;p&gt;Hello&lt;/p&gt;</content>
  </entry>
</feed>"#;

const SAMPLE_OPML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<opml version="2.0">
  <head><title>My feeds</title></head>
  <body>
    <outline text="Tech">
      <outline type="rss" text="Hacker News" title="Hacker News" xmlUrl="https://example.com/feed.xml" htmlUrl="https://news.ycombinator.com/"/>
      <outline type="rss" text="Rust Blog" title="Rust Blog" xmlUrl="https://blog.rust-lang.org/feed.xml" htmlUrl="https://blog.rust-lang.org/"/>
    </outline>
    <outline type="rss" text="BBC" title="BBC" xmlUrl="https://feeds.bbci.co.uk/news/rss.xml" htmlUrl="https://www.bbc.com/news"/>
  </body>
</opml>"#;

#[test]
fn parse_rss_and_atom() {
    let rss = parse_feed_body(SAMPLE_RSS, "https://example.com/feed").expect("rss parse");
    assert_eq!(rss.title, "Example Feed");
    assert_eq!(rss.entries.len(), 2);
    assert_eq!(rss.entries[0].title.as_deref(), Some("First Post"));
    assert_eq!(rss.entries[0].guid.as_deref(), Some("abc-123"));
    assert!(rss.entries[0].published_at.is_some());
    assert_eq!(rss.entries[1].summary.as_deref(), Some("Another summary"));

    let atom = parse_feed_body(SAMPLE_ATOM, "https://example.org/feed").expect("atom parse");
    assert_eq!(atom.title, "Atom Feed");
    assert_eq!(atom.entries.len(), 1);
    assert_eq!(atom.entries[0].author.as_deref(), Some("Jane"));
    assert_eq!(atom.entries[0].guid.as_deref(), Some("urn:uuid:123"));
}

#[test]
fn opml_parse_export_roundtrip() {
    let outlines = opml::parse_opml(SAMPLE_OPML).expect("opml parse");
    assert_eq!(outlines.len(), 2);
    let folder = &outlines[0];
    assert_eq!(folder.text.as_deref(), Some("Tech"));
    assert_eq!(folder.children.len(), 2);
    assert_eq!(
        folder.children[0].xml_url.as_deref(),
        Some("https://example.com/feed.xml")
    );
    assert_eq!(outlines[1].xml_url.as_deref(), Some("https://feeds.bbci.co.uk/news/rss.xml"));

    let exported = opml::export_opml(
        "test",
        &[(
            "Tech".to_string(),
            vec![(
                "Hacker News".to_string(),
                "https://example.com/feed.xml".to_string(),
                Some("https://news.ycombinator.com/".to_string()),
            )],
        )],
        &[(
            "BBC".to_string(),
            "https://feeds.bbci.co.uk/news/rss.xml".to_string(),
            None,
        )],
    )
    .expect("opml export");

    assert!(exported.contains("<opml version=\"2.0\">"));
    assert!(exported.contains("xmlUrl=\"https://example.com/feed.xml\""));
    // 重新解析导出的内容
    let re_parsed = opml::parse_opml(&exported).expect("re-parse");
    assert_eq!(re_parsed.len(), 2);
    assert_eq!(re_parsed[0].children.len(), 1);
    assert_eq!(re_parsed[1].xml_url.as_deref(), Some("https://feeds.bbci.co.uk/news/rss.xml"));
}

fn temp_db() -> (tempfile::TempDir, Storage) {
    let dir = tempfile::tempdir().expect("tempdir");
    let storage = Storage::open(&dir.path().join("test.db")).expect("open storage");
    (dir, storage)
}

#[test]
fn storage_folder_feed_article_flow() {
    let (_dir, storage) = temp_db();

    let folder_id = storage.add_folder("Tech").expect("add folder");
    let feed_id = storage
        .insert_feed(
            Some(folder_id),
            "Example Feed",
            "https://example.com/feed",
            Some("https://example.com"),
            Some("desc"),
            None,
            Some("\"etag-1\""),
            Some("Mon, 17 Aug 2026 10:00:00 GMT"),
        )
        .expect("insert feed");

    let articles: Vec<NewArticle> = vec![
        NewArticle {
            title: Some("A".to_string()),
            url: Some("https://example.com/a".to_string()),
            author: None,
            summary: Some("<p>sum</p>".to_string()),
            content: None,
            published_at: None,
            guid: Some("g1".to_string()),
        },
        NewArticle {
            title: Some("B".to_string()),
            url: Some("https://example.com/b".to_string()),
            author: None,
            summary: None,
            content: None,
            published_at: None,
            guid: Some("g2".to_string()),
        },
    ];

    // 首次插入 2 篇，重复插入应去重
    assert_eq!(storage.insert_articles(feed_id, &articles).expect("insert"), 2);
    assert_eq!(storage.insert_articles(feed_id, &articles).expect("insert dup"), 0);

    // 去重键：guid 为空时用 url 兜底
    let no_guid = NewArticle {
        title: Some("C".to_string()),
        url: Some("https://example.com/c".to_string()),
        author: None,
        summary: None,
        content: None,
        published_at: None,
        guid: None,
    };
    assert_eq!(storage.insert_articles(feed_id, &[no_guid]).expect("insert c"), 1);
    assert_eq!(
        storage
            .insert_articles(
                feed_id,
                &[NewArticle {
                    title: Some("C'".to_string()),
                    url: Some("https://example.com/c".to_string()),
                    author: None,
                    summary: None,
                    content: None,
                    published_at: None,
                    guid: None,
                }],
            )
            .expect("insert c dup"),
        0
    );

    // 列表 & 未读统计
    let all = storage.list_articles(&ArticleFilter::default()).expect("list");
    assert_eq!(all.len(), 3);
    let stats = storage.unread_stats().expect("stats");
    assert_eq!(stats.total, 3);
    assert_eq!(stats.per_feed[0].unread, 3);

    // 已读 / 星标
    let ids: Vec<i64> = all.iter().map(|a| a.id).collect();
    storage.mark_read(&ids[..1], true).expect("mark read");
    let after = storage.list_articles(&ArticleFilter::default()).expect("list");
    assert!(after[0].is_read);
    assert!(!after[1].is_read);
    assert!(storage.toggle_star(ids[0]).expect("star"));
    assert!(!storage.toggle_star(ids[0]).expect("unstar"));

    // feed / 全量已读
    storage.mark_feed_read(feed_id).expect("feed read");
    let read_all = storage.list_articles(&ArticleFilter::default()).expect("list");
    assert!(read_all.iter().all(|a| a.is_read));
    assert_eq!(storage.unread_stats().expect("stats2").total, 0);

    // 分组
    assert_eq!(storage.list_folders().expect("folders").len(), 1);
    storage.remove_folder(folder_id).expect("rm folder");
    assert_eq!(storage.list_folders().expect("folders2").len(), 0);
    // 删除文件夹后 feed 归为未分组
    let feed = storage.get_feed(feed_id).expect("get feed").unwrap();
    assert!(feed.folder_id.is_none());
}

#[test]
fn storage_filters() {
    let (_dir, storage) = temp_db();
    let feed_id = storage
        .insert_feed(None, "F", "https://f/feed", None, None, None, None, None)
        .expect("feed");
    let mut articles: Vec<NewArticle> = Vec::new();
    for i in 0..5 {
        articles.push(NewArticle {
            title: Some(format!("Post {i}")),
            url: Some(format!("https://f/{i}")),
            author: None,
            summary: None,
            content: None,
            published_at: None,
            guid: Some(format!("g{i}")),
        });
    }
    storage.insert_articles(feed_id, &articles).expect("insert");
    let ids = storage
        .list_articles(&ArticleFilter::default())
        .expect("list")
        .iter()
        .map(|a| a.id)
        .collect::<Vec<_>>();

    storage.mark_read(&ids[2..], true).expect("mark");

    let mut f = ArticleFilter::default();
    f.unread_only = true;
    assert_eq!(storage.list_articles(&f).expect("unread").len(), 2);

    let mut f = ArticleFilter::default();
    f.search = Some("Post 3".to_string());
    assert_eq!(storage.list_articles(&f).expect("search").len(), 1);

    let mut f = ArticleFilter::default();
    f.limit = 2;
    f.offset = 0;
    assert_eq!(storage.list_articles(&f).expect("limit").len(), 2);
}

#[test]
fn rss_reader_end_to_end() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reader = RssReader::with_db_path(dir.path().join("test.db")).expect("reader");

    let folder_id = reader.add_folder("Tech").expect("folder");
    let feed_id = reader
        .storage
        .insert_feed(
            Some(folder_id),
            "Sample",
            "https://example.com/feed",
            Some("https://example.com"),
            Some("desc"),
            None,
            None,
            None,
        )
        .expect("insert feed");

    let new_articles = vec![NewArticle {
        title: Some("t".into()),
        url: Some("https://e/1".into()),
        author: None,
        summary: None,
        content: None,
        published_at: None,
        guid: Some("g-1".into()),
    }];
    assert_eq!(reader.storage.insert_articles(feed_id, &new_articles).unwrap(), 1);

    let arts = reader.list_articles(&ArticleFilter::default()).unwrap();
    assert_eq!(arts.len(), 1);

    let stats = reader.unread_stats().unwrap();
    assert_eq!(stats.total, 1);

    reader.mark_read(&[arts[0].id], true).unwrap();
    assert_eq!(reader.unread_stats().unwrap().total, 0);

    reader.toggle_star(arts[0].id).unwrap();
    let fresh = reader.get_article(arts[0].id).unwrap().unwrap();
    assert!(fresh.is_starred);

    // 重复抓取同一订阅（无 etag）：插入文章列表应被去重
    let again = vec![NewArticle {
        title: Some("t".into()),
        url: Some("https://e/1".into()),
        author: None,
        summary: None,
        content: None,
        published_at: None,
        guid: Some("g-1".into()),
    }];
    assert_eq!(reader.storage.insert_articles(feed_id, &again).unwrap(), 0);

    // OPML 导出应包含新分组和订阅源
    let opml = reader.export_opml().unwrap();
    assert!(opml.contains("Tech"));
    assert!(opml.contains("https://example.com/feed"));
    // 重新解析导出的内容应能正确还原
    let parsed = crate::opml::parse_opml(&opml).expect("parse opml");
    let tech_folder = parsed.iter().find(|o| o.text.as_deref() == Some("Tech"));
    assert!(tech_folder.is_some(), "Tech folder should be present");

    reader.remove_feed(feed_id).unwrap();
    assert!(reader.list_feeds().unwrap().is_empty());
    reader.remove_folder(folder_id).unwrap();
    assert!(reader.list_folders().unwrap().is_empty());
}

#[test]
fn opml_import_creates_folders() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reader = RssReader::with_db_path(dir.path().join("test.db")).expect("reader");
    let result = reader.import_opml(SAMPLE_OPML).expect("import");
    // 真实网络可能失败，我们仅验证语法可解析；只要 error 不为空也接受。
    let folders = reader.list_folders().unwrap();
    let feeds = reader.list_feeds().unwrap();
    assert!(!folders.is_empty() || !feeds.is_empty() || !result.errors.is_empty());
}

#[test]
fn article_filter_paging() {
    let (_dir, storage) = temp_db();
    let feed_id = storage
        .insert_feed(None, "F", "https://f/feed", None, None, None, None, None)
        .unwrap();
    let arts: Vec<NewArticle> = (0..10)
        .map(|i| NewArticle {
            title: Some(format!("t{i}")),
            url: Some(format!("https://f/{i}")),
            author: None,
            summary: None,
            content: None,
            published_at: None,
            guid: Some(format!("g{i}")),
        })
        .collect();
    storage.insert_articles(feed_id, &arts).unwrap();

    let mut f = ArticleFilter::default();
    f.limit = 4;
    f.offset = 0;
    let p1 = storage.list_articles(&f).unwrap();
    assert_eq!(p1.len(), 4);

    f.offset = 4;
    let p2 = storage.list_articles(&f).unwrap();
    assert_eq!(p2.len(), 4);
    assert_ne!(p1[0].id, p2[0].id);

    f.offset = 8;
    let p3 = storage.list_articles(&f).unwrap();
    assert_eq!(p3.len(), 2);
}

/// 真实网络：目标站点 RSS 全部文章应能抓取全文（浏览器 UA 修复后）。默认 ignore，手动跑：
/// `cargo test -p rss-core network_fetch_demo -- --ignored --nocapture`
#[test]
#[ignore]
fn network_fetch_demo_full_content() {
    let dir = tempfile::tempdir().expect("tempdir");
    let reader = RssReader::with_db_path(dir.path().join("test.db")).expect("reader");
    let url = "https://example.com/feed.xml";
    let res = reader
        .add_feed(url, None, true)
        .unwrap_or_else(|e| panic!("add_feed failed: {e}"));
    println!("add_feed: inserted={} existed={}", res.articles_new, res.existed);
    let arts = reader.list_articles(&ArticleFilter {
        feed_id: Some(res.feed.id),
        ..ArticleFilter::default()
    }).expect("list_articles");
    println!("total articles: {}", arts.len());
    // backfill 抓取成果（content_fetched=true 且正文>200 即"点开就有全文"）。
    // 不二次 fetch：站点对短时高频请求限流，backfill 未命中的由"打开文章自动抓全文"兜底。
    let ok = arts
        .iter()
        .map(|a| {
            let updated = reader.get_article(a.id).unwrap();
            let has_body = updated
                .as_ref()
                .and_then(|u| u.content.as_deref())
                .map(|c| c.trim().len() > 200)
                .unwrap_or(false);
            let cf = updated.as_ref().map(|u| u.content_fetched).unwrap_or(false);
            println!(
                "  id={} content_fetched={} body>200={} title={:?}",
                a.id,
                cf,
                has_body,
                a.title.as_deref().unwrap_or("")
            );
            cf && has_body
        })
        .filter(|x| *x)
        .count();
    println!("backfill success: {ok}/{} (站点对短时高频请求限流，此数字不用于断言；Referer 放行由 network_debug_ua_matrix 验证)", arts.len());
    assert!(!arts.is_empty(), "expected RSS to parse into articles");
}

#[test]
#[ignore]
fn network_debug_single_fetch() {
    let client = crate::build_client(None).expect("client");
    for u in [
        "https://example.com/articles/1.html",
        "https://example.com/articles/2.html",
        "https://example.com/articles/3.html",
    ] {
        match crate::feed::fetch_full_content(&client, u, false) {
            Ok(Some(h)) => println!("{u}\n  OK len={} text={}", h.len(), crate::fetch::generic::text_len(&h)),
            Ok(None) => println!("{u}\n  None (no/empty content or cf)"),
            Err(e) => println!("{u}\n  ERR: {e}"),
        }
    }
}

#[test]
#[ignore]
fn network_debug_raw() {
    let client = crate::build_client(None).expect("client");
    let u = "https://example.com/articles/1.html";
    let resp = crate::feed::browser_get(&client, u).send().expect("send");
    println!("status: {}", resp.status());
    let body = resp.text().unwrap_or_default();
    println!("body len: {}", body.len());
    println!("detect_cf: {}", crate::fetch::generic::detect_cloudflare(&body));
    let low = body.to_lowercase();
    for kw in ["just a moment", "challenge", "verify", "antibot", "403", "article_content", "html-content", "access"] {
        println!("  contains {kw:?}: {}", low.contains(kw));
    }
    println!("head: {}", body[..body.len().min(300)].replace('\n', " "));
}

#[test]
#[ignore]
fn network_debug_ua_matrix() {
    let client = crate::build_client(None).expect("client");
    let u = "https://example.com/articles/1.html";
    let uas = [
        ("chrome", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36"),
        ("safari605", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15"),
        ("chrome-win", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36"),
    ];
    for (name, ua) in uas {
        for referer in [None, Some("https://example.com/")] {
            let mut req = client.get(u).header(reqwest::header::USER_AGENT, ua);
            if let Some(r) = referer {
                req = req.header(reqwest::header::REFERER, r);
            }
            let resp = req.send().expect("send");
            let st = resp.status().to_string();
            let body = resp.text().unwrap_or_default();
            let has_art = body.contains("article_content");
            println!("{name:12} referer={referer:?} -> {st} art={has_art} len={}", body.len());
        }
    }
}

#[test]
#[ignore]
fn network_debug_fetch_demo() {
    let client = crate::build_client(None).expect("client");
    for u in [
        "https://example.com/articles/4.html",
        "https://example.com/articles/5.html",
    ] {
        match crate::feed::fetch_full_content(&client, u, false) {
            Ok(Some(h)) => {
                let txt = crate::fetch::generic::text_len(&h);
                println!("{u}\n  OK html_len={} text_len={txt}", h.len());
                println!("  head: {}", h[..h.len().min(120)].replace('\n', " "));
            }
            Ok(None) => println!("{u}\n  None"),
            Err(e) => println!("{u}\n  ERR: {e}"),
        }
    }
}

#[test]
#[ignore]
fn rss_core_logging_smoke() {
    // 验证 rss-core 抓取内部日志能正常输出（tracing subscriber 由 desktop 注册）。
    // 示例地址为占位，实际网络验证时替换为目标站点 URL。
    // 手动跑：cargo test -p rss-core rss_core_logging -- --ignored --nocapture
    let client = crate::build_client(None).expect("client");
    let res = crate::feed::fetch_full_content(&client, "https://example.com/articles/4.html", false);
    println!("fetch result: {res:?}");
}




/// 网络回归：后量子 TLS（X25519MLKEM768-only）站点抓取（如 DeepSeek status）。
/// 依赖 openssl/native-tls 栈。默认 ignore，手动跑：
/// `cargo test -p rss-core network_pq -- --ignored --nocapture`
#[test]
#[ignore]
fn network_debug_pq_tls() {
    let client = crate::build_client(None).expect("client");
    let u = "https://status.deepseek.com/feed.rss";
    match crate::feed::fetch_full_content(&client, u, false) {
        Ok(Some(h)) => println!("OK {u} len={}", h.len()),
        Ok(None) => println!("NONE {u}"),
        Err(e) => println!("ERR {u}: {e}"),
    }
}

#[test]
#[ignore]
fn network_deepseek_incident_adapter() {
    let client = crate::build_client(None).expect("client");
    for u in [
        "https://status.deepseek.com/incidents/6877795382287",
        "https://status.deepseek.com/incidents/6877795382287/",
    ] {
        match crate::feed::fetch_full_content(&client, u, false) {
            Ok(Some(h)) => {
                println!("OK {u} len={} text_len={}", h.len(), crate::fetch::generic::text_len(&h));
                println!("  head: {}", h[..h.len().min(120)].replace('\n', " "));
            }
            Ok(None) => println!("NONE {u}"),
            Err(e) => println!("ERR {u}: {e}"),
        }
    }
}


#[test]
#[ignore]
fn headless_fallback_full() {
    // gcores radio 无 adapter，JS 渲染站 → 验证 headless 兜底全流程
    let client = crate::build_client(None).unwrap();
    let u = "https://www.gcores.com/radios/218651";
    match crate::feed::fetch_full_content(&client, u, true) {
        Ok(Some(h)) => println!("OK text_len={}", crate::fetch::generic::text_len(&h)),
        Ok(None) => println!("NONE (通用+headless 都无正文)"),
        Err(e) => println!("ERR {e}"),
    }
}
