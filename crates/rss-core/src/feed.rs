use chrono::{DateTime, Utc};
use feed_rs::parser;

use reqwest::blocking::Client;

use crate::error::{Error, Result};
use crate::fetch::{adapters, generic as g};

/// 从 feed 解析出的待入库数据（内部结构）
pub(crate) struct NewFeed {
    pub title: String,
    pub url: String,
    pub site_url: Option<String>,
    pub description: Option<String>,
    pub favicon_url: Option<String>,
    pub entries: Vec<NewArticle>,
}

pub(crate) struct NewArticle {
    pub title: Option<String>,
    pub url: Option<String>,
    pub author: Option<String>,
    pub summary: Option<String>,
    pub content: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub guid: Option<String>,
}

pub(crate) fn parse_feed_body(body: &str, feed_url: &str) -> Result<NewFeed> {
    let feed = parser::parse(body.as_bytes()).map_err(|e| Error::FeedParse(e.to_string()))?;

    let title = feed
        .title
        .as_ref()
        .map(|t| t.content.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| feed_url.to_string());

    let site_url = feed
        .links
        .iter()
        .find(|l| {
            l.rel.as_deref() == Some("alternate")
                || (l.title.is_none() && l.media_type.as_deref() != Some("application/atom+xml"))
        })
        .or_else(|| feed.links.first())
        .map(|l| l.href.clone());

    let description = feed
        .description
        .as_ref()
        .map(|d| d.content.trim().to_string())
        .filter(|s| !s.is_empty());

    let favicon_url = favicon_for(site_url.as_deref(), feed_url);

    let entries = feed
        .entries
        .into_iter()
        .map(|e| {
            let published = e.published.or(e.updated);
            let guid = if e.id.trim().is_empty() {
                e.links.first().map(|l| l.href.clone())
            } else {
                Some(e.id.trim().to_string())
            };
            NewArticle {
                title: e.title.as_ref().map(|t| t.content.trim().to_string()),
                url: e.links.first().map(|l| l.href.clone()),
                author: e
                    .authors
                    .first()
                    .map(|a| a.name.trim().to_string())
                    .filter(|s| !s.is_empty()),
                summary: e.summary.as_ref().map(|s| s.content.clone()),
                content: e.content.as_ref().and_then(|c| c.body.clone()),
                published_at: published,
                guid,
            }
        })
        .collect();

    Ok(NewFeed {
        title,
        url: feed_url.to_string(),
        site_url,
        description,
        favicon_url,
        entries,
    })
}

pub(crate) fn favicon_for(site_url: Option<&str>, feed_url: &str) -> Option<String> {
    let base = site_url.filter(|s| !s.is_empty()).or(Some(feed_url))?;
    let host = url::Url::parse(base).ok()?.host_str()?.to_string();
    Some(format!("https://www.google.com/s2/favicons?domain={host}&sz=64"))
}

/// 浏览器 UA，用于抓取全文/原文/图片时模拟真实浏览器，提升可访问性。
pub(crate) const BROWSER_UA: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";

/// 构造带浏览器头（UA / Accept / 语言）的 GET 请求。
pub(crate) fn browser_get(client: &reqwest::blocking::Client, url: &str) -> reqwest::blocking::RequestBuilder {
    use reqwest::header::REFERER;
    // 同域 Referer：CSDN 等反爬站对无 Referer 请求返回 521 安全验证（实测带 Referer 才放行）。
    let referer = url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|h| format!("https://{h}/")));
    let mut req = client
        .get(url)
        .header(reqwest::header::USER_AGENT, BROWSER_UA)
        .header(
            reqwest::header::ACCEPT,
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8",
        )
        .header(reqwest::header::ACCEPT_LANGUAGE, "zh-CN,zh;q=0.9,en;q=0.8");
    if let Some(r) = referer {
        req = req.header(REFERER, r);
    }
    req
}

/// 用 readability 抓取网页正文（返回 HTML）。readability 结果明显短于正文容器时，
/// 改用容器提取（对"多卡片/聚合"类文章更完整，如 sspai 派早报）。
pub(crate) fn fetch_full_content(
    client: &Client,
    page_url: &str,
    render_fallback: bool,
) -> Result<Option<String>> {
    tracing::info!(target: "rss_core::fetch", "[fetch_full] start url={page_url}");
    let resp = browser_get(client, page_url).send()?;
    let status = resp.status();
    if !status.is_success() {
        tracing::warn!(target: "rss_core::fetch", "[fetch_full] http_non_success url={page_url} status={status}");
        return Ok(None);
    }
    let body = resp.text().unwrap_or_default();
    if body.trim().is_empty() || g::detect_cloudflare(&body) {
        tracing::warn!(target: "rss_core::fetch", "[fetch_full] empty_or_cf url={page_url} body_len={}", body.len());
        // CF 挑战页/空响应：抓不到正文，交由"打开原文"兜底
        return Ok(None);
    }
    tracing::debug!(target: "rss_core::fetch", "[fetch_full] body_len={}", body.len());

    let url = url::Url::parse(page_url)?;

    // 站点适配器优先（如 gcores API 正文）：比 readability/容器可靠（JS 站静态 HTML 无正文）
    if let Some(adapter) = adapters::find_adapter(page_url) {
        if let Ok(Some(html)) = adapter.fetch_full(client, page_url) {
            let normalized = g::normalize_images(&html, page_url);
            let len = g::text_len(&normalized);
            tracing::info!(target: "rss_core::fetch", "[fetch_full] adapter={} text_len={len}", adapter.name());
            if len >= 200 {
                return Ok(Some(normalized));
            }
        }
    }

    // 通用 Flashcat statuspage 兜底：无 host 适配器时，按页面内容特征探测
    //（DeepSeek 等多家公司的状态页都走 Flashcat，正文容器 class 通用，readability 常选不中）
    if adapters::flashcat_status::is_flashcat_page(&body) {
        if let Some(html) = adapters::flashcat_status::extract_flashcat_page(&body) {
            let normalized = g::normalize_images(&html, page_url);
            let len = g::text_len(&normalized);
            tracing::info!(target: "rss_core::fetch", "[fetch_full] flashcat_status text_len={len}");
            if len >= 200 {
                return Ok(Some(normalized));
            }
        }
    }

    let mut cursor = std::io::Cursor::new(body.clone().into_bytes());
    let readability_html = readability::extractor::extract(&mut cursor, &url)
        .map(|p| p.content)
        .ok();

    // 正文容器提取（取文本量最大者，并清洗噪声）
    let container_html = g::extract_content_container(&body);

    let readability_len = readability_html.as_deref().map(g::text_len).unwrap_or(0);
    let container_len = container_html.as_deref().map(g::text_len).unwrap_or(0);
    let readability_imgs = readability_html
        .as_deref()
        .map(|h| h.matches("<img").count())
        .unwrap_or(0);
    let container_imgs = container_html
        .as_deref()
        .map(|h| h.matches("<img").count())
        .unwrap_or(0);

    tracing::debug!(
        target: "rss_core::fetch",
        "[fetch_full] readability_len={readability_len} readability_imgs={readability_imgs} container_len={container_len} container_imgs={container_imgs}"
    );

    let (chosen, branch) = if container_len >= 200 && container_len as f64 >= readability_len as f64 * 0.8 {
        (container_html.unwrap(), "container>=readability*0.8")
    } else if container_len >= 200 && readability_len >= 200 && container_imgs > 0 && readability_imgs == 0 {
        (container_html.unwrap(), "container_has_imgs")
    } else if readability_len >= 200 {
        (readability_html.unwrap(), "readability")
    } else if container_len >= 200 {
        (container_html.unwrap(), "container")
    } else {
        // 都不够 200 字，readability 也没拿到 → 返回 readability（可能为 None）
        match readability_html {
            Some(h) => (g::normalize_images(&h, page_url), "readability_short"),
            None => {
                tracing::warn!(target: "rss_core::fetch", "[fetch_full] no_content readability_len={readability_len} container_len={container_len}");
                return Ok(None);
            }
        }
    };

    let normalized = g::fix_protocol_relative(&g::normalize_images(&chosen, page_url));
    tracing::info!(
        target: "rss_core::fetch",
        "[fetch_full] ok branch={branch} text_len={}",
        g::text_len(&normalized)
    );
    // 通用提取不足（<200 字，如 JS 渲染站）→ 可选用 headless 浏览器渲染后再提取
    if render_fallback && g::text_len(&normalized) < 200 {
        if let Some(rendered) = crate::fetch::render::render_dom(page_url) {
            let mut cur2 = std::io::Cursor::new(rendered.as_bytes().to_vec());
            let r2 = readability::extractor::extract(&mut cur2, &url)
                .map(|p| p.content)
                .ok();
            let c2 = g::extract_content_container(&rendered);
            let rl2 = r2.as_deref().map(g::text_len).unwrap_or(0);
            let cl2 = c2.as_deref().map(g::text_len).unwrap_or(0);
            let best = if cl2 >= 200 && cl2 >= rl2 { c2 } else { r2 };
            if let Some(b) = best {
                let rb = g::fix_protocol_relative(&g::normalize_images(&b, page_url));
                let len = g::text_len(&rb);
                if len >= 200 {
                    tracing::info!(target: "rss_core::fetch", "[fetch_full] headless_render text_len={len} url={page_url}");
                    return Ok(Some(rb));
                }
            }
        }
    }
    Ok(Some(normalized))
}

/// 网页资源（用于"应用内打开原文"）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PageResource {
    pub kind: String, // "html" | "file"
    pub content_type: String,
    pub content: String,
    /// 站点是否允许被 iframe 内嵌（X-Frame-Options / CSP frame-ancestors 检测）。
    /// 为 false 时前端应提示"用浏览器打开"而非白屏。
    pub allow_embed: bool,
}

/// 根据响应头判断站点是否允许被第三方 iframe 内嵌。
fn allow_embed(headers: &reqwest::header::HeaderMap) -> bool {
    if let Some(v) = headers.get("x-frame-options") {
        if let Ok(s) = v.to_str() {
            let up = s.to_uppercase();
            if up.contains("DENY") || up.contains("SAMEORIGIN") {
                return false;
            }
        }
    }
    if let Some(v) = headers.get("content-security-policy") {
        if let Ok(s) = v.to_str() {
            let low = s.to_lowercase();
            if low.contains("frame-ancestors 'none'")
                || low.contains("frame-ancestors 'self'")
                || low.contains("frame-ancestors 'sameorigin'")
            {
                return false;
            }
        }
    }
    true
}

/// 抓取网页资源：HTML 会去掉 <script> 并做图片归一化；文件（PDF 等）仅标记类型。
/// 对 Cloudflare 挑战页等"非 200 但含挑战 HTML"的响应，也返回 body 供前端识别，
/// 避免打开原文白屏无提示。
pub(crate) fn fetch_page_resource(
    client: &reqwest::blocking::Client,
    page_url: &str,
) -> Result<PageResource> {
    let resp = browser_get(client, page_url).send()?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().unwrap_or_default();
        if crate::fetch::generic::detect_cloudflare(&body) {
            return Ok(PageResource {
                kind: "html".into(),
                content_type: "text/html".into(),
                content: body,
                allow_embed: true,
            });
        }
        return Err(Error::Fetch(format!("HTTP {}", status)));
    }
    let headers = resp.headers().clone();
    let content_type = headers
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();
    let is_html = content_type.contains("text/html")
        || content_type.contains("application/xhtml")
        || content_type.contains("application/rss+xml")
        || content_type.contains("application/atom+xml")
        || content_type.is_empty();
    let embed = allow_embed(&headers);
    if !is_html && is_file_url(page_url) {
        return Ok(PageResource {
            kind: "file".into(),
            content_type,
            content: String::new(),
            allow_embed: embed,
        });
    }
    let body = resp.text().unwrap_or_default();
    Ok(PageResource {
        kind: "html".into(),
        content_type,
        content: g::normalize_images(&strip_scripts(&body), page_url),
        allow_embed: embed,
    })
}

/// 根据 URL 扩展名判断是否为需要按文件处理的资源（pdf/office/压缩包等）。
fn is_file_url(url: &str) -> bool {
    let path = url.split('?').next().unwrap_or(url).to_lowercase();
    [
        ".pdf", ".doc", ".docx", ".xls", ".xlsx", ".ppt", ".pptx", ".epub", ".zip", ".rar",
        ".7z", ".gz", ".tar", ".mp3", ".mp4", ".avi", ".mov", ".png", ".jpg", ".jpeg",
        ".gif", ".webp", ".svg", ".txt", ".csv", ".json", ".xml",
    ]
    .iter()
    .any(|ext| path.ends_with(ext))
}

/// 轻量探测网页资源类型（只读响应头，不抓 body），用于判断 HTML 还是文件（PDF 等）。
pub(crate) fn probe_page_resource(
    client: &reqwest::blocking::Client,
    page_url: &str,
) -> Result<PageResource> {
    let resp = browser_get(client, page_url).send()?;
    if !resp.status().is_success() {
        return Err(Error::Fetch(format!("HTTP {}", resp.status())));
    }
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();
    let is_html = content_type.contains("text/html")
        || content_type.contains("application/xhtml")
        || content_type.contains("application/rss+xml")
        || content_type.contains("application/atom+xml")
        || content_type.is_empty();
    let kind = if !is_html && is_file_url(page_url) {
        "file"
    } else {
        "html"
    };
    Ok(PageResource {
        kind: kind.into(),
        content_type,
        content: String::new(),
        allow_embed: allow_embed(resp.headers()),
    })
}

/// 移除 <script>…</script> 与 <script … />，降低内联渲染风险。
fn strip_scripts(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let bytes = html.as_bytes();
    let mut i = 0;
    let lower = html.to_ascii_lowercase();
    while i < bytes.len() {
        if lower[i..].starts_with("<script") {
            // 找 </script>
            if let Some(end) = lower[i..].find("</script>") {
                let real_end = i + end + "</script>".len();
                // 检查是否自闭合 <script .../>
                let tag_part = &lower[i..real_end];
                if tag_part.contains("/>") && !tag_part[..tag_part.len() - 9].contains(">") {
                    // 自闭合
                    i = real_end;
                } else {
                    i = real_end;
                }
                continue;
            }
        }
        // 直接复制当前字节
        let ch_len = g::utf8_len(bytes[i]);
        out.push_str(&html[i..i + ch_len]);
        i += ch_len;
    }
    out
}