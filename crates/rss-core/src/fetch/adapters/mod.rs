//! 抓取层：站点专项适配器注册表。
//!
//! 通用抓取流水线（`super::generic`）对绝大多数站点有效；需要特殊处理的站点
//! 在此注册 `SiteAdapter`。新增站点 = 新建一个 adapter 文件 + `find_adapter` 加一行。

pub mod gcores;

use reqwest::blocking::Client;

use crate::error::Result;

/// 站点专项适配器：覆盖/补充通用抓取逻辑（正文提取、API 抓取、图片处理等）。
pub trait SiteAdapter {
    /// 匹配的站点（host 包含判断），如 `"gcores.com"`。
    #[allow(dead_code)]
    fn host(&self) -> &str;

    /// 适配器名称（用于日志），如 `"gcores"`。
    fn name(&self) -> &str;

    /// 抓取并返回处理后的正文 HTML（`None` 表示本适配器不适用/失败，交由通用层兜底）。
    fn fetch_full(&self, client: &Client, url: &str) -> Result<Option<String>>;
}

/// 按 URL 匹配适配器；无匹配返回 `None`（走通用流水线）。
pub fn find_adapter(url: &str) -> Option<Box<dyn SiteAdapter>> {
    let host = url::Url::parse(url).ok()?.host_str()?.to_lowercase();
    if host.contains("gcores.com") {
        return Some(Box::new(gcores::GcoresAdapter));
    }
    None
}
