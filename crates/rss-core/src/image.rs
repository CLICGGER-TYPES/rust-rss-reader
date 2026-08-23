//! 图片代理与磁盘缓存。
//!
//! 职责：
//! - `fetch_image`：带浏览器 UA + 智能 Referer（优先源站域）抓图，失败自动重试
//! - 图片磁盘缓存：`<data_dir>/img_cache/{hash}.bin + .type`，二次打开秒出、省网络
//!
//! 该模块不感知业务（订阅/文章），只负责"URL → 图片字节"。

use std::path::{Path, PathBuf};

use reqwest::blocking::Client;
use reqwest::header::CONTENT_TYPE;

use crate::error::{Error, Result};

#[derive(Debug, Clone, serde::Serialize)]
pub struct FetchedImage {
    pub content_type: String,
    pub data_b64: String,
}

/// 稳定 64 位 FNV-1a 哈希（不依赖平台 hash 算法，跨进程一致，供磁盘缓存文件名）。
fn hash64(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn cache_paths(dir: &Path, url: &str) -> (PathBuf, PathBuf) {
    let h = format!("{:016x}", hash64(url));
    (dir.join(format!("{h}.bin")), dir.join(format!("{h}.type")))
}

fn read_cache(dir: &Path, url: &str) -> Option<FetchedImage> {
    let (bin, typ) = cache_paths(dir, url);
    let bytes = std::fs::read(&bin).ok()?;
    let ct = std::fs::read_to_string(&typ).ok()?;
    Some(FetchedImage {
        content_type: ct,
        data_b64: base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &bytes,
        ),
    })
}

fn write_cache(dir: &Path, url: &str, bytes: &[u8], ct: &str) {
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let (bin, typ) = cache_paths(dir, url);
    let _ = std::fs::write(&bin, bytes);
    let _ = std::fs::write(&typ, ct);
}

/// 清除全部图片磁盘缓存。
pub fn clear_image_cache(data_dir: &Path) {
    let _ = std::fs::remove_dir_all(data_dir.join("img_cache"));
}

/// 从 URL 提取 host（无则返回 None）。
fn host_of(url: &str) -> Option<String> {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
}

/// 图片 URL 补全：正文里常出现相对路径（如 `/media/xxx.png`），需用源站（referer）域名拼绝对地址。
fn resolve_image_url(url: &str, referer: Option<&str>) -> String {
    if url.starts_with("http://") || url.starts_with("https://") || url.starts_with("data:") {
        return url.to_string();
    }
    if let Some(page) = referer {
        if let Ok(base) = url::Url::parse(page) {
            if let Ok(u) = base.join(url) {
                return u.to_string();
            }
        }
    }
    url.to_string()
}

/// 抓取图片字节。`referer` 为可选源站 URL（防盗链更准：多数 CDN 要求 Referer=引用页面域，
/// 而非图片自身域）。无 referer 时回退到图片 host。带磁盘缓存：命中直接返回，失败重试 3 次。
/// `max_width`：若图宽超过该值则压缩到该宽度（WebView 解码 4K 大图会卡死，压缩后流畅）。
pub fn fetch_image(
    client: &Client,
    data_dir: &Path,
    url: &str,
    referer: Option<&str>,
    max_width: Option<u32>,
) -> Result<FetchedImage> {
    let url = resolve_image_url(url, referer);
    let cache_dir = data_dir.join("img_cache");
    if let Some(hit) = read_cache(&cache_dir, &url) {
        tracing::debug!(target: "rss_core::image", "[fetch_image] cache_hit url={url}");
        return Ok(hit);
    }
    let img_host = host_of(&url);
    let ref_host = referer
        .and_then(host_of)
        .or_else(|| img_host.clone());
    tracing::debug!(
        target: "rss_core::image",
        "[fetch_image] start url={url} img_host={img_host:?} ref_host={ref_host:?}"
    );
    let mut last_err: Option<Error> = None;
    for attempt in 0..3 {
        let mut req = client
            .get(&url)
            .header(reqwest::header::USER_AGENT, crate::feed::BROWSER_UA)
            .header(
                reqwest::header::ACCEPT,
                "image/avif,image/webp,image/apng,image/*,*/*;q=0.8",
            );
        if let Some(h) = &ref_host {
            req = req.header("Referer", format!("https://{h}/"));
        }
        match req.send() {
            Ok(resp) if resp.status().is_success() => {
                let mut content_type = resp
                    .headers()
                    .get(CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("image/*")
                    .to_string();
                let mut bytes = resp.bytes()?.to_vec();
                if let Some(mw) = max_width {
                    if let Some((b, ct)) = compress_if_large(&bytes, &content_type, mw) {
                        bytes = b;
                        content_type = ct;
                    }
                }
                write_cache(&cache_dir, &url, &bytes, &content_type);
                tracing::info!(
                    target: "rss_core::image",
                    "[fetch_image] ok attempt={} url={url} bytes={} ct={content_type}",
                    attempt + 1,
                    bytes.len()
                );
                return Ok(FetchedImage {
                    content_type,
                    data_b64: base64::Engine::encode(
                        &base64::engine::general_purpose::STANDARD,
                        &bytes,
                    ),
                });
            }
            Ok(resp) => {
                last_err = Some(Error::Fetch(format!("HTTP {}", resp.status())));
                tracing::warn!(
                    target: "rss_core::image",
                    "[fetch_image] http_non_success attempt={} url={url} status={}",
                    attempt + 1,
                    resp.status()
                );
            }
            Err(e) => {
                last_err = Some(Error::Http(e));
                tracing::warn!(
                    target: "rss_core::image",
                    "[fetch_image] net_err attempt={} url={url} err={last_err:?}",
                    attempt + 1
                );
            }
        }
    }
    Err(last_err.unwrap_or_else(|| Error::Fetch("image fetch failed".into())))
}

/// 若图片宽超过 `max_width` 则等比压缩，返回 (新bytes, 新content_type)。PNG 保留 PNG，
/// 其余（含透明）转 JPEG。失败返回 None（保留原图）。
fn compress_if_large(bytes: &[u8], ct: &str, max_width: u32) -> Option<(Vec<u8>, String)> {
    let img = image::load_from_memory(bytes).ok()?;
    if img.width() <= max_width {
        return None;
    }
    let ratio = max_width as f32 / img.width() as f32;
    let h = ((img.height() as f32) * ratio).round().max(1.0) as u32;
    let resized = img.resize(max_width, h, image::imageops::FilterType::Lanczos3);
    let mut out = Vec::new();
    if ct.contains("png") && img.color().has_alpha() {
        resized
            .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .ok()?;
        return Some((out, "image/png".into()));
    }
    let rgb = resized.to_rgb8();
    rgb.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Jpeg)
        .ok()?;
    Some((out, "image/jpeg".into()))
}
