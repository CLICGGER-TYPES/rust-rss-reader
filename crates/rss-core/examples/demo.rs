use rss_core::{ArticleFilter, RssReader};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::var("RSS_DEMO_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("rss-reader-demo"));
    let reader = RssReader::with_data_dir(dir)?;

    let res = reader.add_feed("https://hnrss.org/frontpage", None, false)?;
    println!("feed added: '{}' (+{} articles)", res.feed.title, res.articles_new);

    let res = reader.add_feed("https://blog.rust-lang.org/feed.xml", None, false)?;
    println!("feed added: '{}' (+{} articles)", res.feed.title, res.articles_new);

    let articles = reader.list_articles(&ArticleFilter::default())?;
    println!("total articles: {}", articles.len());
    for a in articles.iter().take(5) {
        let title = a.title.as_deref().unwrap_or("(untitled)");
        let ts = a.published_at.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default();
        println!("  - [{ts}] {title}");
    }

    // 已读 / 星标
    let ids: Vec<i64> = articles.iter().take(2).map(|a| a.id).collect();
    reader.mark_read(&ids, true)?;
    reader.toggle_star(ids[0])?;
    println!("unread stats: {}", reader.unread_stats()?.total);

    // 搜索
    let mut f = ArticleFilter::default();
    f.search = Some("rust".to_string());
    println!("search 'rust': {} results", reader.list_articles(&f)?.len());

    // OPML 导出
    let xml = reader.export_opml()?;
    println!("OPML export ({} bytes):", xml.len());
    println!("{}", xml);
    Ok(())
}
