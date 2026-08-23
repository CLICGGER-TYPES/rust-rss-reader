//! DeepSeek 服务状态页（status.deepseek.com）专项适配器。
//!
//! DeepSeek 用 Flashcat 托管状态页，incident 详情页正文是服务端渲染但容器 class 通用，
//! 通用流水线选不中。走专用 adapter：host 精确匹配 `status.deepseek.com`，
//! 抓取页面后用 Flashcat 通用提取逻辑拿正文。

use reqwest::blocking::Client;

use crate::error::{Error, Result};

use super::{flashcat_status, SiteAdapter};

pub struct DeepseekStatusAdapter;

impl SiteAdapter for DeepseekStatusAdapter {
    fn host(&self) -> &str {
        "status.deepseek.com"
    }

    fn name(&self) -> &str {
        "deepseek_status"
    }

    fn fetch_full(&self, client: &Client, url: &str) -> Result<Option<String>> {
        let resp = client
            .get(url)
            .header(reqwest::header::USER_AGENT, crate::feed::BROWSER_UA)
            .send()
            .map_err(Error::Http)?;
        if !resp.status().is_success() {
            return Ok(None);
        }
        let html = resp.text().unwrap_or_default();
        if !flashcat_status::is_flashcat_page(&html) {
            return Ok(None);
        }
        Ok(flashcat_status::extract_flashcat_page(&html))
    }
}
