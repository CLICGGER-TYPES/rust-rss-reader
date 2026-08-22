use std::sync::Arc;

use rss_core::{
    AddFeedResult, Article, ArticleFilter, Feed, Folder, OpmlImportResult, RefreshResult,
    RssReader, UnreadStats,
};
use tauri::{Manager, State, WebviewUrl, WebviewWindowBuilder};

struct Reader(Arc<RssReader>);

type CmdResult<T> = Result<T, String>;

use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// 初始化日志：写入 `{data_dir}/logs/app.log`（按天滚动），同时输出到 stdout。
/// 返回的 guard 必须存活到进程结束，否则日志管道会被提前关闭。
fn init_logging(data_dir: &std::path::Path) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let log_dir = data_dir.join("logs");
    if let Err(e) = std::fs::create_dir_all(&log_dir) {
        eprintln!("[rss-desktop] failed to create log dir {}: {e}", log_dir.display());
        return None;
    }
    let file_appender = tracing_appender::rolling::daily(&log_dir, "app.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_target(false);
    let stdout_layer = fmt::layer().with_writer(std::io::stdout).with_target(false);
    tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .with(stdout_layer)
        .init();
    tracing::info!(
        "[startup] log file: {}/app.log (RUST_LOG can raise verbosity)",
        log_dir.display()
    );
    Some(guard)
}

/// 在异步 command 内把同步的 RssReader 调用放到阻塞线程池执行，避免阻塞 UI。
async fn blocking<T, F>(reader: Arc<RssReader>, f: F) -> rss_core::Result<T>
where
    T: Send + 'static,
    F: FnOnce(&RssReader) -> rss_core::Result<T> + Send + 'static,
{
    let name = std::any::type_name::<F>();
    tracing::debug!("[cmd] start: {name}");
    let result = tauri::async_runtime::spawn_blocking(move || f(&reader))
        .await
        .unwrap_or_else(|e| panic!("blocking task panicked: {e}"));
    if let Err(e) = &result {
        tracing::error!("[cmd] FAILED: {name}: {e}");
    }
    result
}

// ---------- folders ----------

#[tauri::command]
async fn list_folders(reader: State<'_, Reader>) -> CmdResult<Vec<Folder>> {
    Ok(blocking(reader.inner().0.clone(), |r| r.list_folders()).await?)
}

#[tauri::command]
async fn add_folder(reader: State<'_, Reader>, name: String) -> CmdResult<i64> {
    blocking(reader.inner().0.clone(), move |r| r.add_folder(&name))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn rename_folder(reader: State<'_, Reader>, id: i64, name: String) -> CmdResult<()> {
    blocking(reader.inner().0.clone(), move |r| r.rename_folder(id, &name))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn remove_folder(reader: State<'_, Reader>, id: i64) -> CmdResult<()> {
    blocking(reader.inner().0.clone(), move |r| r.remove_folder(id))
        .await
        .map_err(|e| e.to_string())
}

// ---------- feeds ----------

#[tauri::command]
async fn list_feeds(reader: State<'_, Reader>) -> CmdResult<Vec<Feed>> {
    Ok(blocking(reader.inner().0.clone(), |r| r.list_feeds()).await?)
}

#[tauri::command]
async fn add_feed(
    reader: State<'_, Reader>,
    url: String,
    folder_id: Option<i64>,
    fetch_full: bool,
) -> CmdResult<AddFeedResult> {
    tracing::info!("[cmd] add_feed url={url} folder_id={folder_id:?} fetch_full={fetch_full}");
    blocking(reader.inner().0.clone(), move |r| r.add_feed(&url, folder_id, fetch_full))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn remove_feed(reader: State<'_, Reader>, id: i64) -> CmdResult<()> {
    blocking(reader.inner().0.clone(), move |r| r.remove_feed(id))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_feed_folder(
    reader: State<'_, Reader>,
    feed_id: i64,
    folder_id: Option<i64>,
) -> CmdResult<()> {
    blocking(reader.inner().0.clone(), move |r| r.set_feed_folder(feed_id, folder_id))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn rename_feed(reader: State<'_, Reader>, feed_id: i64, title: String) -> CmdResult<()> {
    blocking(reader.inner().0.clone(), move |r| r.rename_feed(feed_id, &title))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_feed_refresh_interval(
    reader: State<'_, Reader>,
    feed_id: i64,
    minutes: Option<i64>,
) -> CmdResult<()> {
    blocking(reader.inner().0.clone(), move |r| r.set_feed_refresh_interval(feed_id, minutes))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_feed_use_proxy(
    reader: State<'_, Reader>,
    feed_id: i64,
    use_proxy: bool,
) -> CmdResult<()> {
    blocking(reader.inner().0.clone(), move |r| {
        r.set_feed_use_proxy(feed_id, use_proxy)
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_feed_default_original(
    reader: State<'_, Reader>,
    feed_id: i64,
    on: bool,
) -> CmdResult<()> {
    blocking(reader.inner().0.clone(), move |r| {
        r.set_feed_default_original(feed_id, on)
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_feed_url(
    reader: State<'_, Reader>,
    feed_id: i64,
    url: String,
) -> CmdResult<()> {
    blocking(reader.inner().0.clone(), move |r| {
        r.update_feed_url(feed_id, &url)
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn prune_articles(
    reader: State<'_, Reader>,
    days: i64,
    include_unread: bool,
) -> CmdResult<usize> {
    blocking(reader.inner().0.clone(), move |r| r.prune_articles(days, include_unread))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn test_connection(reader: State<'_, Reader>) -> CmdResult<String> {
    tracing::info!("[cmd] test_connection");
    blocking(reader.inner().0.clone(), |r| r.test_connection())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn refresh(reader: State<'_, Reader>, fetch_full: bool) -> CmdResult<RefreshResult> {
    tracing::info!("[cmd] refresh fetch_full={fetch_full}");
    blocking(reader.inner().0.clone(), move |r| r.refresh_all(fetch_full))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn refresh_feed(
    reader: State<'_, Reader>,
    id: i64,
    fetch_full: bool,
) -> CmdResult<usize> {
    tracing::info!("[cmd] refresh_feed id={id} fetch_full={fetch_full}");
    blocking(reader.inner().0.clone(), move |r| r.refresh_feed(id, fetch_full))
        .await
        .map_err(|e| e.to_string())
}

// ---------- articles ----------

#[allow(clippy::too_many_arguments)]
#[tauri::command]
async fn list_articles(
    reader: State<'_, Reader>,
    feed_id: Option<i64>,
    folder_id: Option<i64>,
    unread_only: bool,
    starred_only: bool,
    search: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
    sort: Option<String>,
) -> CmdResult<Vec<Article>> {
    tracing::info!(
        "[cmd] list_articles feed_id={feed_id:?} folder_id={folder_id:?} unread_only={unread_only} starred_only={starred_only} search={search:?} limit={limit:?} offset={offset:?} sort={sort:?}"
    );
    let mut filter = ArticleFilter {
        feed_id,
        folder_id,
        unread_only,
        starred_only,
        search,
        sort: sort
            .as_deref()
            .map(rss_core::models::ArticleSort::parse)
            .unwrap_or(rss_core::models::ArticleSort::TimeDesc),
        ..ArticleFilter::default()
    };
    filter.limit = limit.unwrap_or(200);
    filter.offset = offset.unwrap_or(0);
    Ok(blocking(reader.inner().0.clone(), move |r| r.list_articles(&filter)).await?)
}

#[tauri::command]
async fn get_article(reader: State<'_, Reader>, id: i64) -> CmdResult<Option<Article>> {
    blocking(reader.inner().0.clone(), move |r| r.get_article(id))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn mark_read(reader: State<'_, Reader>, ids: Vec<i64>, read: bool) -> CmdResult<()> {
    blocking(reader.inner().0.clone(), move |r| r.mark_read(&ids, read))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn mark_feed_read(reader: State<'_, Reader>, feed_id: i64) -> CmdResult<()> {
    blocking(reader.inner().0.clone(), move |r| r.mark_feed_read(feed_id))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn mark_folder_read(reader: State<'_, Reader>, folder_id: i64) -> CmdResult<()> {
    blocking(reader.inner().0.clone(), move |r| r.mark_folder_read(folder_id))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn mark_all_read(reader: State<'_, Reader>) -> CmdResult<()> {
    blocking(reader.inner().0.clone(), |r| r.mark_all_read())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn toggle_star(reader: State<'_, Reader>, id: i64) -> CmdResult<bool> {
    blocking(reader.inner().0.clone(), move |r| r.toggle_star(id))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn fetch_full_content(reader: State<'_, Reader>, id: i64) -> CmdResult<bool> {
    tracing::info!("[cmd] fetch_full_content id={id}");
    let result = blocking(reader.inner().0.clone(), move |r| r.fetch_article_full_content(id))
        .await
        .map_err(|e| e.to_string())?;
    tracing::info!("[cmd] fetch_full_content id={id} result={result}");
    Ok(result)
}

// ---------- stats ----------

#[tauri::command]
async fn unread_stats(reader: State<'_, Reader>) -> CmdResult<UnreadStats> {
    Ok(blocking(reader.inner().0.clone(), |r| r.unread_stats()).await?)
}

// ---------- OPML ----------

#[tauri::command]
async fn import_opml(reader: State<'_, Reader>, content: String) -> CmdResult<OpmlImportResult> {
    blocking(reader.inner().0.clone(), move |r| r.import_opml(&content))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn export_opml(reader: State<'_, Reader>) -> CmdResult<String> {
    blocking(reader.inner().0.clone(), |r| r.export_opml())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn import_opml_from(reader: State<'_, Reader>, path: String) -> CmdResult<OpmlImportResult> {
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    blocking(reader.inner().0.clone(), move |r| r.import_opml(&content))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn export_opml_to(reader: State<'_, Reader>, path: String) -> CmdResult<()> {
    let xml = blocking(reader.inner().0.clone(), |r| r.export_opml())
        .await
        .map_err(|e| e.to_string())?;
    std::fs::write(&path, xml).map_err(|e| e.to_string())
}

// ---------- settings ----------

#[tauri::command]
async fn get_setting(reader: State<'_, Reader>, key: String) -> CmdResult<Option<String>> {
    Ok(blocking(reader.inner().0.clone(), move |r| r.get_setting(&key)).await?)
}

#[tauri::command]
async fn set_setting(reader: State<'_, Reader>, key: String, value: String) -> CmdResult<()> {
    tracing::info!("[cmd] set_setting key={key} value_len={}", value.chars().count());
    blocking(reader.inner().0.clone(), move |r| r.set_setting(&key, &value))
        .await
        .map_err(|e| e.to_string())
}

// ---------- 代理 / 缓存 / 数据目录 / 关于 ----------

#[tauri::command]
async fn get_proxy(reader: State<'_, Reader>) -> CmdResult<Option<String>> {
    Ok(blocking(reader.inner().0.clone(), |r| r.get_proxy()).await?)
}

#[tauri::command]
async fn set_proxy(reader: State<'_, Reader>, proxy: Option<String>) -> CmdResult<()> {
    tracing::info!("[cmd] set_proxy proxy={proxy:?}");
    blocking(reader.inner().0.clone(), move |r| r.set_proxy(proxy.as_deref()))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn clear_content_cache(reader: State<'_, Reader>) -> CmdResult<usize> {
    tracing::info!("[cmd] clear_content_cache");
    let n = blocking(reader.inner().0.clone(), |r| r.clear_content_cache()).await?;
    blocking(reader.inner().0.clone(), |r| {
        r.clear_image_cache();
        Ok(())
    })
    .await?;
    Ok(n)
}

#[tauri::command]
async fn get_data_dir() -> CmdResult<String> {
    let dir = match rss_core::config::load_data_dir() {
        Some(d) => d,
        None => dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("rss-reader"),
    };
    Ok(dir.display().to_string())
}

/// 设置数据目录并迁移 DB。返回新路径。注意：迁移后需要重启应用才能生效。
#[tauri::command]
async fn set_data_dir(new_dir: String, migrate: bool) -> CmdResult<String> {
    tracing::info!("[cmd] set_data_dir new_dir={new_dir} migrate={migrate}");
    let path = std::path::PathBuf::from(&new_dir);
    std::fs::create_dir_all(&path).map_err(|e| format!("create dir failed: {e}"))?;
    let new_db = path.join("rss.db");
    if migrate {
        let old_dir = rss_core::config::load_data_dir().unwrap_or_else(|| {
            dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("rss-reader")
        });
        let old_db = old_dir.join("rss.db");
        if old_db != new_db && old_db.exists() {
            std::fs::copy(&old_db, &new_db).map_err(|e| format!("copy db failed: {e}"))?;
        }
    }
    rss_core::config::save_data_dir(Some(path))?;
    Ok(new_db.display().to_string())
}

#[derive(serde::Serialize)]
struct AppInfo {
    name: String,
    version: String,
    tauri_version: String,
    license: String,
    homepage: String,
}

#[tauri::command]
async fn get_app_info() -> CmdResult<AppInfo> {
    Ok(AppInfo {
        name: "Rust RSS Reader".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        tauri_version: tauri::VERSION.to_string(),
        license: "MIT".to_string(),
        homepage: "https://github.com/yang991178/fluent-reader".to_string(),
    })
}

/// 在应用内打开原文：探测资源类型（HTML 或 PDF 等文件），供前端决定内嵌真实网页或文件视图。
#[tauri::command]
async fn probe_url(reader: State<'_, Reader>, url: String) -> CmdResult<rss_core::PageResource> {
    tracing::info!("[cmd] probe_url url={url}");
    blocking(reader.inner().0.clone(), move |r| r.probe_page_resource(&url))
        .await
        .map_err(|e| e.to_string())
}

/// 在应用内打开原文（抓整页，保留用于需要内容时）。
#[tauri::command]
async fn fetch_original_html(reader: State<'_, Reader>, url: String) -> CmdResult<rss_core::PageResource> {
    tracing::info!("[cmd] fetch_original_html url={url}");
    blocking(reader.inner().0.clone(), move |r| r.fetch_page_resource(&url))
        .await
        .map_err(|e| e.to_string())
}

/// 把文本写入指定路径（用于导出 Markdown/HTML/TXT 等）。
#[tauri::command]
async fn write_text_file(path: String, content: String) -> CmdResult<()> {
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

/// 图片代理：带 Referer/UA 抓图，返回 base64，前端转 blob 渲染。
/// `referer` 可选：文章源站 URL，供防盗链（CDN 通常校验引用页域而非图片域）。
#[tauri::command]
async fn fetch_image(
    reader: State<'_, Reader>,
    url: String,
    referer: Option<String>,
) -> CmdResult<rss_core::FetchedImage> {
    tracing::info!("[cmd] fetch_image url={url} referer={referer:?}");
    blocking(reader.inner().0.clone(), move |r| {
        r.fetch_image(&url, referer.as_deref())
    })
    .await
    .map_err(|e| e.to_string())
}

/// 在独立的隔离 webview 窗口打开媒体播放器（网易云/YouTube/B站等），与主阅读区沙箱隔离。
/// `width`/`height` 可选：不传时用默认 1100×800；音乐后台小窗可传 420×620。
#[tauri::command]
async fn open_media_window(
    app: tauri::AppHandle,
    url: String,
    width: Option<f64>,
    height: Option<f64>,
) -> CmdResult<()> {
    tracing::info!("[cmd] open_media_window url_len={} width={width:?} height={height:?}", url.len());
    let label = format!("media-{}", url.len());
    if app.get_webview_window(&label).is_some() {
        return Ok(());
    }
    let parsed = url.parse::<url::Url>().map_err(|e| e.to_string())?;
    WebviewWindowBuilder::new(&app, &label, WebviewUrl::External(parsed))
        .title("媒体播放")
        .inner_size(width.unwrap_or(1100.0), height.unwrap_or(800.0))
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 前端日志透传：把渲染进程发来的日志行写进同一份日志文件。
#[tauri::command]
fn log_to_file(line: String) {
    tracing::info!("{line}");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 数据目录：优先用配置文件里的覆盖，否则默认目录。
    let data_dir = rss_core::config::load_data_dir().unwrap_or_else(|| {
        dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("rss-reader")
    });
    let _log_guard = init_logging(&data_dir);
    tracing::info!("[startup] data_dir={}", data_dir.display());
    tracing::info!(
        "[startup] version={} tauri={}",
        env!("CARGO_PKG_VERSION"),
        tauri::VERSION
    );
    let reader = RssReader::with_data_dir(data_dir).expect("failed to open rss-reader database");
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Reader(Arc::new(reader)))
        .invoke_handler(tauri::generate_handler![
            list_folders,
            add_folder,
            rename_folder,
            remove_folder,
            list_feeds,
            add_feed,
            remove_feed,
            set_feed_folder,
            rename_feed,
            set_feed_refresh_interval,
            set_feed_use_proxy,
            set_feed_default_original,
            update_feed_url,
            prune_articles,
            test_connection,
            refresh,
            refresh_feed,
            list_articles,
            get_article,
            mark_read,
            mark_feed_read,
            mark_folder_read,
            mark_all_read,
            toggle_star,
            fetch_full_content,
            unread_stats,
            import_opml,
            export_opml,
            import_opml_from,
            export_opml_to,
            get_setting,
            set_setting,
            get_proxy,
            set_proxy,
            clear_content_cache,
            get_data_dir,
            set_data_dir,
            get_app_info,
            fetch_original_html,
            probe_url,
            write_text_file,
            fetch_image,
            open_media_window,
            log_to_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
