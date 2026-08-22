//! gcores（机核）专项适配器。
//!
//! 机核文章正文由前端 JS 通过 `gapi` 接口拉取（DraftJS raw JSON），静态 HTML 无正文，
//! 通用流水线拿不到内容。这里直接调 `GET /gapi/v1/articles/{id}` 并把 DraftJS blocks
//! 转成 HTML（段落 + 图片 + 嵌入播放器），交给上游继续走图片归一化与清洗。

use reqwest::blocking::Client;

use crate::error::{Error, Result};

use super::SiteAdapter;

pub struct GcoresAdapter;

impl SiteAdapter for GcoresAdapter {
    fn host(&self) -> &str {
        "gcores.com"
    }

    fn name(&self) -> &str {
        "gcores"
    }

    fn fetch_full(&self, client: &Client, url: &str) -> Result<Option<String>> {
        let Some(id) = article_id(url) else {
            return Ok(None);
        };
        let api = format!("https://www.gcores.com/gapi/v1/articles/{id}");
        let resp = client
            .get(&api)
            .header(reqwest::header::USER_AGENT, crate::feed::BROWSER_UA)
            .header(
                reqwest::header::ACCEPT,
                "application/json, text/plain, */*",
            )
            .header(reqwest::header::ACCEPT_LANGUAGE, "zh-CN,zh;q=0.9,en;q=0.8")
            .send()?;
        if !resp.status().is_success() {
            return Ok(None);
        }
        let text = resp.text().unwrap_or_default();
        let body: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| Error::Invalid(format!("gcores json: {e}")))?;
        let content = body
            .get("data")
            .and_then(|d| d.get("attributes"))
            .and_then(|a| a.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("");
        if content.trim().is_empty() {
            return Ok(None);
        }
        draftjs_to_html(content)
    }
}

fn article_id(url: &str) -> Option<&str> {
    url.rsplit('/')
        .find(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
}

/// DraftJS raw contentState → 简单 HTML。
fn draftjs_to_html(raw: &str) -> Result<Option<String>> {
    use serde_json::Value;

    let v: Value = serde_json::from_str(raw)
        .map_err(|e| Error::Invalid(format!("gcores draftjs: {e}")))?;
    let Some(blocks) = v.get("blocks").and_then(|b| b.as_array()) else {
        return Ok(None);
    };
    let entity_map = v.get("entityMap").cloned().unwrap_or(Value::Null);

    let mut out = String::with_capacity(raw.len());
    for blk in blocks {
        let typ = blk.get("type").and_then(|t| t.as_str()).unwrap_or("");
        let text = blk.get("text").and_then(|t| t.as_str()).unwrap_or("");
        if typ == "atomic" {
            if let Some(ranges) = blk.get("entityRanges").and_then(|e| e.as_array()) {
                for range in ranges {
                    if let Some(key) = range.get("key").and_then(|k| k.as_u64()) {
                        if let Some(ent) = entity_map.get(key.to_string()) {
                            let etype = ent.get("type").and_then(|t| t.as_str()).unwrap_or("");
                            let data = ent.get("data");
                            render_entity(&mut out, etype, data);
                        }
                    }
                }
            }
        } else if !text.trim().is_empty() {
            out.push_str(&format!("<p>{}</p>", escape_html(text)));
        }
    }
    if out.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(out))
}

fn render_entity(out: &mut String, etype: &str, data: Option<&serde_json::Value>) {
    let get = |k: &str| data.and_then(|d| d.get(k)).and_then(|x| x.as_str());
    match etype {
        "IMAGE" => {
            if let Some(path) = get("path") {
                // gcores 图片域是 image.gcores.com；path 可能是纯文件名/相对路径/完整 URL
                let src = if path.starts_with("http://") || path.starts_with("https://") {
                    path.to_string()
                } else if path.starts_with("//") {
                    format!("https:{path}")
                } else if path.starts_with("/") {
                    format!("https://image.gcores.com{path}")
                } else {
                    format!("https://image.gcores.com/{path}")
                };
                out.push_str(&format!("<p><img src=\"{src}\"></p>"));
            }
        }
        // 视频/嵌入（B站/腾讯/网易云 iframe 等）：实为 data.content（HTML），其次 html/url
        "VIDEO" | "EMBED" => {
            let html = get("content").or_else(|| get("html"));
            if let Some(html) = html {
                out.push_str(&format!("<p>{html}</p>"));
            } else if let Some(url) = get("url") {
                out.push_str(&format!("<p><a href=\"{url}\">{url}</a></p>"));
            }
        }
        // 卡片/链接（如商店链接）
        "WIDGET" | "LINK" => {
            if let Some(url) = get("url") {
                out.push_str(&format!("<p><a href=\"{url}\">{url}</a></p>"));
            }
        }
        _ => {}
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
