//! TUI 中英文 i18n。语言从共享数据库 settings 读取（桌面端可改），默认跟随系统 LANG。

use std::sync::OnceLock;

static LANG: OnceLock<Lang> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Zh,
    En,
}

impl Lang {
    pub fn from_env() -> Self {
        let lang = std::env::var("LANG").unwrap_or_default().to_lowercase();
        if lang.starts_with("zh") {
            Lang::Zh
        } else {
            Lang::En
        }
    }
}

/// 初始化语言。data_dir 用于读取共享 settings（与桌面端一致）。
pub fn init(data_dir: Option<&std::path::Path>) {
    let mut lang = Lang::from_env();
    if let Some(dir) = data_dir {
        if let Ok(reader) = rss_core::RssReader::with_data_dir(dir.to_path_buf()) {
            if let Ok(Some(v)) = reader.get_setting("lang") {
                if v == "zh" {
                    lang = Lang::Zh;
                } else if v == "en" {
                    lang = Lang::En;
                }
            }
        }
    }
    let _ = LANG.set(lang);
}

pub fn lang() -> Lang {
    *LANG.get().unwrap_or(&Lang::En)
}

fn dict(lang: Lang) -> &'static [(&'static str, &'static str)] {
    match lang {
        Lang::Zh => ZH,
        Lang::En => EN,
    }
}

pub fn t(key: &str) -> String {
    lookup(lang(), key).to_string()
}

fn lookup(lang: Lang, key: &str) -> &str {
    dict(lang)
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| *v)
        .unwrap_or(key)
}

pub fn fmt(key: &str, params: &[(&str, &str)]) -> String {
    fmt_with(lang(), key, params)
}

fn fmt_with(lang: Lang, key: &str, params: &[(&str, &str)]) -> String {
    let mut s = lookup(lang, key).to_string();
    for (k, v) in params {
        s = s.replace(&format!("{{{k}}}"), v);
    }
    s
}

const ZH: &[(&str, &str)] = &[
    ("all", "全部文章"),
    ("unread", "未读"),
    ("starred", "星标"),
    ("folders", "分组"),
    ("feeds", "订阅"),
    ("reader", "阅读"),
    ("reader_hint", "按 ? 查看快捷键"),
    ("no_articles", "（空）"),
    ("select_hint", "选择一篇文章后在此阅读"),
    ("refreshing", "刷新中…"),
    ("refreshing_feed", "刷新订阅中…"),
    ("refreshing_all", "刷新全部订阅中…"),
    ("refreshed", "已刷新：新增 {n} 篇"),
    ("errors", "，{n} 个错误"),
    ("feed_removed", "订阅已删除"),
    ("folder_removed", "分组已删除"),
    ("feed_added", "已添加订阅 '{title}'（新增 {n} 篇）"),
    ("feed_add_failed", "添加订阅失败：{e}"),
    ("folder_created", "已创建分组 '{name}'"),
    ("folder_failed", "创建分组失败：{e}"),
    ("renamed", "已重命名为 '{name}'"),
    ("rename_failed", "重命名失败：{e}"),
    ("move_failed", "移动失败：{e}"),
    ("moved", "已移动到 '{label}'"),
    ("rename_cancelled", "已取消重命名"),
    ("starred_ok", "已星标"),
    ("unstarred_ok", "已取消星标"),
    ("full_fetched", "全文已获取"),
    ("no_full", "无可用全文"),
    ("opening", "正在打开：{url}"),
    ("marked_all_read", "已全部标为已读"),
    ("imported", "导入：新增 {a} 个订阅，{e} 个已存在，{err} 个错误"),
    ("import_failed", "导入失败：{e}"),
    ("exported", "已导出到 {path}"),
    ("export_failed", "导出失败：{e}"),
    ("import_input", "要导入的 OPML 文件路径"),
    ("export_input", "导出 OPML 路径（默认：export.opml）"),
    ("export_cancelled", "已取消导出"),
    ("by", "作者"),
    ("press_help", "按 ? 查看快捷键"),
    ("input_add_feed", "添加订阅 URL（Enter 确定 Esc 取消）"),
    ("input_new_folder", "新建分组名称"),
    ("input_rename_folder", "重命名分组为（Esc 取消）"),
    ("input_move_feed", "移动到分组（留空=未分组，Esc 取消）"),
    ("input_search", "搜索（Esc 清除）"),
    ("input_import", "要导入的 OPML 文件路径"),
    ("input_export", "导出 OPML 路径（默认 export.opml）"),
];

const EN: &[(&str, &str)] = &[
    ("all", "All Articles"),
    ("unread", "Unread"),
    ("starred", "Starred"),
    ("folders", "Folders"),
    ("feeds", "Feeds"),
    ("reader", "Reader"),
    ("reader_hint", "press ? for key bindings"),
    ("no_articles", "(empty)"),
    ("select_hint", "Select an article to read it here"),
    ("refreshing", "refreshing…"),
    ("refreshing_feed", "refreshing feed…"),
    ("refreshing_all", "refreshing all feeds…"),
    ("refreshed", "refreshed: {n} new article(s)"),
    ("errors", ", {n} errors"),
    ("feed_removed", "feed removed"),
    ("folder_removed", "folder removed"),
    ("feed_added", "added '{title}' (+{n} articles)"),
    ("feed_add_failed", "add feed failed: {e}"),
    ("folder_created", "folder '{name}' created"),
    ("folder_failed", "add folder failed: {e}"),
    ("renamed", "renamed to '{name}'"),
    ("rename_failed", "rename failed: {e}"),
    ("move_failed", "move failed: {e}"),
    ("moved", "moved to '{label}'"),
    ("rename_cancelled", "rename cancelled"),
    ("starred_ok", "starred"),
    ("unstarred_ok", "unstarred"),
    ("full_fetched", "full content fetched"),
    ("no_full", "no full content available"),
    ("opening", "opening: {url}"),
    ("marked_all_read", "marked all read"),
    ("imported", "import: +{a} feeds, {e} existed, {err} errors"),
    ("import_failed", "import failed: {e}"),
    ("exported", "exported to {path}"),
    ("export_failed", "export failed: {e}"),
    ("import_input", "Path to OPML file to import"),
    ("export_input", "Export OPML path (default: export.opml)"),
    ("export_cancelled", "export cancelled"),
    ("by", "by"),
    ("press_help", "press ? for key bindings"),
    ("input_add_feed", "Add feed URL (Enter=ok Esc=cancel)"),
    ("input_new_folder", "New folder name"),
    ("input_rename_folder", "Rename folder to (Esc=cancel)"),
    ("input_move_feed", "Move feed to folder (blank=ungrouped, Esc=cancel)"),
    ("input_search", "Search (Esc clears)"),
    ("input_import", "Path to OPML file to import"),
    ("input_export", "Export OPML path (default: export.opml)"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dict_lookup_works() {
        assert_eq!(lookup(Lang::Zh, "all"), "全部文章");
        assert_eq!(lookup(Lang::En, "all"), "All Articles");
        assert_eq!(lookup(Lang::Zh, "nope"), "nope");
        assert_eq!(
            fmt_with(Lang::Zh, "feed_added", &[("title", "HN"), ("n", "3")]),
            "已添加订阅 'HN'（新增 3 篇）"
        );
        assert_eq!(
            fmt_with(Lang::En, "refreshed", &[("n", "5")]),
            "refreshed: 5 new article(s)"
        );
    }

    #[test]
    fn env_detection() {
        let lang = std::env::var("LANG").unwrap_or_default().to_lowercase();
        let expect = if lang.starts_with("zh") { Lang::Zh } else { Lang::En };
        assert_eq!(Lang::from_env(), expect);
    }
}
