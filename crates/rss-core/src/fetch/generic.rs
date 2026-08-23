//! 通用抓取处理：正文容器提取、HTML 清洗、图片归一化、Cloudflare 挑战检测。
//!
//! 这里的函数与网络 IO 解耦（输入 HTML 字符串 → 输出处理后的 HTML），
//! 由 `fetch_full_content` / `fetch_page_resource` 调用；站点专项逻辑放 `super::adapters`。


/// 去掉 HTML 标签后的文本长度（用于比较完整度）。
pub(crate) fn text_len(html: &str) -> usize {
    let mut len = 0usize;
    let bytes = html.as_bytes();
    let mut in_tag = false;
    for &b in bytes {
        if b == b'<' {
            in_tag = true;
        } else if b == b'>' {
            in_tag = false;
        } else if !in_tag && !b.is_ascii_whitespace() {
            len += 1;
        }
    }
    len
}

pub(crate) fn utf8_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b >> 5 == 0b110 {
        2
    } else if b >> 4 == 0b1110 {
        3
    } else if b >> 3 == 0b11110 {
        4
    } else {
        1
    }
}

/// 判断是否为 Cloudflare 挑战页（"Just a moment..."），此时抓正文/内嵌原文都会失败。
pub(crate) fn detect_cloudflare(body: &str) -> bool {
    let low = body.to_ascii_lowercase();
    low.contains("just a moment")
        || low.contains("challenges.cloudflare.com")
        || (low.contains("cf-challenge") && low.contains("cf-browser-verification"))
}

/// 备用策略：抓取常见正文容器（`<article>`、`[role=main]`、`.post-content` 等），
/// 取**文本量最大**的一个并清洗噪声。选择器持续扩充以覆盖更多站点。
pub(crate) fn extract_content_container(html: &str) -> Option<String> {
    use scraper::{Html, Selector};

    let document = Html::parse_document(html);
    let selectors = [
        "article",
        "[role='main']",
        "#content",
        "#main-content",
        ".post-content",
        ".article-content",
        ".entry-content",
        ".article-body",
        ".post-body",
        ".article_content",   // CSDN 外层
        ".htmledit_views",    // CSDN 正文（skins）
        ".artical-body",      // FreeBuf（拼写）
        ".tinymce-editor",    // FreeBuf / 常见富文本
        ".content-detail",    // FreeBuf
        ".content-detail-wrap",
        "[class*='article-body']",
        "[class*='article-content']",
        ".article-main",
        ".rich_media_content", // 微信公众号
        ".js_content",
        ".doc-content",        // 游研社
    ];

    let mut best: Option<(usize, String)> = None;
    for sel in selectors {
        let Ok(selector) = Selector::parse(sel) else { continue };
        for node in document.select(&selector) {
            let raw = node.inner_html();
            let cleaned = clean_container(&raw);
            let len = text_len(&cleaned);
            if len >= 200 && best.as_ref().map(|(l, _)| len > *l).unwrap_or(true) {
                best = Some((len, cleaned));
            }
        }
    }
    best.map(|(_, h)| h)
}

/// 跳过从 `i`（指向 `<`）开始的块：深度匹配同名开/闭标签（处理嵌套），返回块后位置。
/// `open_name` 形如 `"<div"`（不含 class 等）。
fn skip_block(lower: &str, mut i: usize, open_name: &str) -> usize {
    let close = format!("</{}", &open_name[1..]);
    // 跳过开标签自身（到 '>'）
    if let Some(gt) = lower[i..].find('>') {
        i += gt + 1;
    } else {
        return lower.len();
    }
    let mut depth = 1usize;
    let name_len = open_name.len();
    while i < lower.len() {
        if lower[i..].starts_with(&close) {
            depth -= 1;
            if depth == 0 {
                // 跳到闭合标签的 '>' 之后
                if let Some(gt) = lower[i..].find('>') {
                    return i + gt + 1;
                }
                return lower.len();
            }
            // 跳过 </tag>
            if let Some(gt) = lower[i..].find('>') {
                i += gt + 1;
                continue;
            }
            return lower.len();
        }
        // 嵌套同标签开：<div ... 或 <div>
        if lower[i..].starts_with(open_name) {
            let after = lower[i + name_len..].chars().next().unwrap_or('\0');
            if after.is_whitespace() || after == '>' || after == '/' {
                depth += 1;
            }
        }
        let b = lower.as_bytes()[i];
        i += utf8_len(b);
    }
    lower.len()
}

/// 清洗容器 HTML：去掉 script/style/nav/footer/header/aside/form 及评论/讨论/推荐/充电/页脚区块，
/// 移除关键词推广标题块（富文本里无独立 class 的 h2/h3），并移除内联事件属性。
pub(crate) fn clean_container(html: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let mut out = String::with_capacity(html.len());
    let bytes = html.as_bytes();
    let mut i = 0;
    // 高危块标签（svg 是内容图（图表/logo），不删除）
    let block_tags = [
        "<script", "<style", "<nav", "<footer", "<header", "<aside", "<form",
    ];
    // 非内容的资源/清理标签：link（CSS/icon，会残留源站样式）、meta、base、iframe(不带内容时)
    let resource_tags = ["<link", "<meta", "<base"];
    // 评论区/讨论/推荐等容器（div/section/ul/ol + 类名特征），深度匹配删除
    let noise_class_markers = [
        "comment", "discussion", "recommend", "reply", "related", "charge", "copyright",
        "footer",
    ];
    // 富文本里无独立 class 的推广/相关区块：h2/h3 标题文本命中即删除该块（到下一个标题或末尾）
    let promo_head_markers = [
        "近期动态", "你可能错过", "相关阅读", "相关文章", "推荐阅读", "本周精选",
        "相关推荐", "热门推荐", "更多精彩",
    ];
    while i < bytes.len() {
        let mut skipped = false;
        // 1) 高危块标签
        for tag in block_tags {
            if lower[i..].starts_with(tag) {
                i = skip_block(&lower, i, tag);
                skipped = true;
                break;
            }
        }
        if skipped {
            continue;
        }
        // 1.5) 资源标签（link/meta/base）：自闭合，整段丢弃（残留源站 CSS/icon，还会串样式）
        for tag in resource_tags {
            if lower[i..].starts_with(tag) {
                if let Some(gt) = lower[i..].find('>') {
                    i += gt + 1;
                    skipped = true;
                }
                break;
            }
        }
        if skipped {
            continue;
        }
        // 2) 噪声容器（div/section/ul/ol 且 class/id 含评论等特征）
        if lower[i..].starts_with("<div")
            || lower[i..].starts_with("<section")
            || lower[i..].starts_with("<ul")
            || lower[i..].starts_with("<ol")
        {
            // 取该开标签到 '>'，检查 class
            if let Some(gt) = lower[i..].find('>') {
                let open_tag = &lower[i..i + gt];
                let is_noise = noise_class_markers.iter().any(|m| {
                    // 在 class/id 属性值里匹配 marker（避免误伤 url 含 comment）
                    (open_tag.contains("class") || open_tag.contains("id")) && open_tag.contains(m)
                });
                if is_noise {
                    let open_name = &lower[i..i + open_tag.find(' ').unwrap_or(gt)];
                    i = skip_block(&lower, i, open_name);
                    skipped = true;
                }
            }
            if skipped {
                continue;
            }
        }
        // 3) 关键词标题块（h2/h3 标题文本含推广/相关词 → 删到下一个标题或末尾）
        if lower[i..].starts_with("<h2") || lower[i..].starts_with("<h3") {
            let head = if lower[i..].starts_with("<h2") { "h2" } else { "h3" };
            if let Some(gt) = lower[i..].find('>') {
                let inner = i + gt + 1;
                let close = format!("</{}", head);
                if let Some(rel) = lower[inner..].find(&close) {
                    let head_text = &lower[inner..inner + rel];
                    if promo_head_markers.iter().any(|k| head_text.contains(k)) {
                        let after = inner + rel + close.len();
                        let rest = &lower[after..];
                        let heads = ["<h2", "<h3", "<h4"];
                        let next = heads
                            .iter()
                            .filter_map(|p| rest.find(p).map(|r| (p, r)))
                            .min_by_key(|(_, r)| *r);
                        i = match next {
                            Some((_, r)) => after + r,
                            None => lower.len(),
                        };
                        skipped = true;
                    }
                }
            }
            if skipped {
                continue;
            }
        }
        let ch_len = utf8_len(bytes[i]);
        out.push_str(&html[i..i + ch_len]);
        i += ch_len;
    }
    // 二次扫描移除内联事件属性（onxxx=...）
    let mut result = String::with_capacity(out.len());
    let low = out.to_ascii_lowercase();
    let inline_events = [
        "onclick", "onload", "onerror", "onmouse", "onfocus", "onblur", "onsubmit",
    ];
    let mut j = 0;
    let ob = out.as_bytes();
    while j < ob.len() {
        if ob[j] == b'o' && ob[j].is_ascii() {
            let mut ev = false;
            for e in inline_events {
                if low[j..].starts_with(e) {
                    let after = low[j + e.len()..].chars().next().unwrap_or('\0');
                    if after == '=' || after.is_whitespace() {
                        ev = true;
                        break;
                    }
                }
            }
            if ev {
                let mut k = j + 1;
                while k < ob.len() && !matches!(ob[k], b' ' | b'\t' | b'\r' | b'\n' | b'>') {
                    k += 1;
                }
                j = k;
                continue;
            }
        }
        result.push_str(&out[j..j + utf8_len(ob[j])]);
        j += utf8_len(ob[j]);
    }
    result
}

/// 图片归一化：懒加载属性 data-src/data-original/data-lazy-src → src，
/// 相对 URL 基于 base 补全为绝对地址，保留其他属性，追加 referrerpolicy=no-referrer。
pub(crate) fn normalize_images(html: &str, base: &str) -> String {
    let bytes = html.as_bytes();
    let mut out = String::with_capacity(html.len());
    let mut i = 0;
    let lower = html.to_ascii_lowercase();
    while i < bytes.len() {
        if lower[i..].starts_with("<img") {
            if let Some(rel_end) = lower[i..].find('>') {
                let end = i + rel_end + 1;
                let tag = &html[i..end];
                out.push_str(&normalize_img_tag(tag, base));
                i = end;
                continue;
            }
        }
        let ch_len = utf8_len(bytes[i]);
        out.push_str(&html[i..i + ch_len]);
        i += ch_len;
    }
    out
}

/// 归一化单个 <img> 标签。
fn normalize_img_tag(tag: &str, base: &str) -> String {
    // 候选图片地址属性，优先懒加载属性
    const SRC_KEYS: [&str; 8] = [
        "data-src",
        "data-original",
        "data-lazy-src",
        "data-url",
        "data-echo",
        "data-lazy",
        "src",
        "srcset",
    ];
    let mut src: Option<String> = None;
    for key in SRC_KEYS {
        if let Some(v) = extract_attr(tag, key) {
            let v = v.trim();
            if !v.is_empty() && !v.starts_with("data:") {
                src = Some(v.to_string());
                break;
            }
        }
    }
    let Some(raw) = src else {
        return tag.to_string();
    };
    // srcset 取第一个候选
    let candidate = raw.split(',').next().unwrap_or(&raw).trim();
    let abs = resolve_url(candidate, base);

    // 保留原有属性，但移除 src 系属性，再补 src=abs + referrerpolicy
    let mut kept = String::new();
    // 剥离 <img 前缀
    let after_open = tag.find('>').unwrap_or(tag.len());
    let inner = &tag[..after_open];
    let mut pos = 4; // 跳过 "<img"
    while pos < inner.len() {
        while pos < inner.len() && (inner.as_bytes()[pos] as char).is_whitespace() {
            pos += 1;
        }
        if pos >= inner.len() {
            break;
        }
        // 属性名
        let name_start = pos;
        while pos < inner.len()
            && !matches!(inner.as_bytes()[pos], b'=' | b' ' | b'\t' | b'\r' | b'\n' | b'>')
        {
            pos += 1;
        }
        let name = &inner[name_start..pos];
        // 跳过空白
        while pos < inner.len() && matches!(inner.as_bytes()[pos], b' ' | b'\t' | b'\r' | b'\n') {
            pos += 1;
        }
        if pos < inner.len() && inner.as_bytes()[pos] == b'=' {
            pos += 1;
            while pos < inner.len() && matches!(inner.as_bytes()[pos], b' ' | b'\t' | b'\r' | b'\n') {
                pos += 1;
            }
            // 属性值
            let value_start = pos;
            if pos < inner.len() && (inner.as_bytes()[pos] == b'"' || inner.as_bytes()[pos] == b'\'') {
                let quote = inner.as_bytes()[pos];
                pos += 1;
                while pos < inner.len() && inner.as_bytes()[pos] != quote {
                    pos += 1;
                }
                pos = (pos + 1).min(inner.len());
            } else {
                while pos < inner.len()
                    && !matches!(inner.as_bytes()[pos], b' ' | b'\t' | b'\r' | b'\n' | b'>')
                {
                    pos += 1;
                }
            }
            let _val = &inner[value_start..pos];
        }
        // 只保留非图片地址属性
        let name_lower = name.to_ascii_lowercase();
        let skip = name_lower.starts_with("src")
            || name_lower.starts_with("data-src")
            || name_lower.starts_with("data-original")
            || name_lower.starts_with("data-lazy")
            || name_lower.starts_with("data-url")
            || name_lower.starts_with("data-echo")
            || name_lower.starts_with("loading");
        if !name_lower.is_empty() && !skip {
            kept.push_str(&inner[name_start..pos]);
        }
    }
    // 处理自闭合
    let self_closing = inner.ends_with('/');
    let close = if self_closing { " />" } else { ">" };
    format!("<img{kept} src=\"{abs}\"{close}")
}

/// 从标签字符串中提取指定属性值（返回去掉引号的值）。
fn extract_attr(tag: &str, key: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let mut search_from = 0;
    while let Some(idx) = lower[search_from..].find(key) {
        let start = search_from + idx;
        // 确认是完整属性名（前面是空白或标签开头，后面是 = 或空白）
        let before_ok = start == 0
            || matches!(tag.as_bytes()[start - 1], b' ' | b'\t' | b'\r' | b'\n' | b'<' | b'/');
        let after = lower[start + key.len()..]
            .chars()
            .next()
            .unwrap_or('\0');
        if before_ok && after == '=' {
            let mut p = start + key.len() + 1;
            let bytes = tag.as_bytes();
            while p < bytes.len() && matches!(bytes[p], b' ' | b'\t' | b'\r' | b'\n') {
                p += 1;
            }
            if p < bytes.len() && (bytes[p] == b'"' || bytes[p] == b'\'') {
                let quote = bytes[p];
                p += 1;
                let vstart = p;
                while p < bytes.len() && bytes[p] != quote {
                    p += 1;
                }
                return Some(tag[vstart..p].to_string());
            }
            let vstart = p;
            while p < bytes.len() && !matches!(bytes[p], b' ' | b'\t' | b'\r' | b'\n' | b'>') {
                p += 1;
            }
            return Some(tag[vstart..p].to_string());
        }
        search_from = start + key.len();
    }
    None
}

/// 把相对 URL 解析为基于 base 的绝对 URL；已是绝对或无法解析则原样返回。
/// 同时把 HTML 实体 `&amp;` 解码为 `&`（微信图床等站点 URL 常带实体符号）。
pub(crate) fn resolve_url(url: &str, base: &str) -> String {
    let u = url.trim().replace("&amp;", "&");
    if u.starts_with("http://") || u.starts_with("https://") || u.starts_with("//") {
        if u.starts_with("//") {
            // 协议相对：http: 前缀
            return format!("https:{u}");
        }
        return u;
    }
    match url::Url::parse(base).and_then(|b| b.join(&u)) {
        Ok(abs) => abs.to_string(),
        Err(_) => u,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain_text(html: &str) -> String {
        let s = html.replace('<', " <").replace('>', "> ");
        let out = s
            .split('<')
            .map(|x| x.split('>').nth(1).unwrap_or(""))
            .collect::<Vec<_>>()
            .join(" ");
        out.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn normalize_images_data_src_and_absolute() {
        // sspai 风格：src 带 query 缩略图 + data-original 原图
        let html = r#"<p><img src="https://cdn.sspai.com/a.png?imageView2/2/w/1120" data-original="https://cdn.sspai.com/a.png"/></p>"#;
        let out = normalize_images(html, "https://sspai.com/post/1");
        assert!(out.contains("src=\"https://cdn.sspai.com/a.png\""), "应取 data-original: {out}");
        assert!(!out.contains("data-original"), "应移除 data-original: {out}");
        assert!(!out.contains("?imageView2"), "应移除原 src query: {out}");
    }

    #[test]
    fn normalize_images_relative_url() {
        let html = r#"<img src="/images/foo.jpg">"#;
        let out = normalize_images(html, "https://example.com/post/1");
        assert!(out.contains("src=\"https://example.com/images/foo.jpg\""), "相对补全: {out}");
    }

    #[test]
    fn normalize_images_lazy_data_src() {
        let html = r#"<img data-src="https://cdn.x.com/x.png" alt="x">"#;
        let out = normalize_images(html, "https://x.com/post");
        assert!(out.contains("src=\"https://cdn.x.com/x.png\""), "data-src 转 src: {out}");
        assert!(out.contains("alt=\"x\""), "保留其他属性: {out}");
    }

    #[test]
    fn resolve_url_decodes_amp() {
        // 微信图床 URL 常带 &amp; 实体
        let u = "https://mmbiz.qpic.cn/a.png?wx_fmt=png&amp;tp=webp";
        assert_eq!(resolve_url(u, "https://x.com/"), "https://mmbiz.qpic.cn/a.png?wx_fmt=png&tp=webp");
    }

    #[test]
    fn container_picks_longest_and_cleans_noise() {
        // 模拟"多卡片晨报"：article 里含多条新闻卡片 + nav/footer/script 噪声
        let html = r#"<html><head><script>var x=1;</script></head><body>
            <nav>导航链接</nav>
            <article class="morning__paper__article">
                <h1>今日早报</h1>
                <div class="article-body">
                    <h2>新闻一</h2><p>某某公司发布新品，股价应声上涨，市场反应热烈，分析师预计后续还有更多动作值得关注。</p><img src="https://cdn.sspai.com/a.jpg">
                    <h2>新闻二</h2><p>某地举办科技展会，众多创新产品亮相，现场参观者络绎不绝，主办方表示明年将继续扩大规模并引入更多国际展商参与。</p><img src="https://cdn.sspai.com/b.jpg">
                    <h2>新闻三</h2><p>第三则新闻：某开源项目发布新版本，修复多项安全漏洞并带来性能提升，社区开发者普遍反馈良好。</p><img src="https://cdn.sspai.com/c.jpg">
                </div>
            </article>
            <footer>页脚</footer>
        </body></html>"#;
        let got = extract_content_container(html).expect("container");
        assert!(!got.contains("<script"), "应清 script");
        assert!(!got.contains("<nav"), "应清 nav");
        assert!(!got.contains("<footer"), "应清 footer");
        assert!(got.contains("新闻一") && got.contains("新闻二") && got.contains("新闻三"), "应保留全部新闻卡片");
        assert!(text_len(&got) > 200, "容器文本应足够长");
    }

    #[test]
    fn text_len_ignores_tags() {
        assert!(text_len("<p>你好 world</p><img src='x'/>") > 5);
    }

    #[test]
    fn csdn_extracts_htmledit_views_and_drops_link() {
        // 复现 CSDN：外层 .article_content 内嵌 .htmledit_views，顶部残留 <link rel=stylesheet>
        let html = r#"<html><body><div id="article_content" class="article_content clearfix">
            <link rel="stylesheet" href="https://csdnimg.cn/xx/editor.css">
            <div id="content_views" class="htmledit_views atom-one-dark">
                <h2>1. 什么是位域？</h2>
                <p>位域是 C++ 中一种特殊的结构体成员声明方式，允许程序员以位为单位来指定成员变量所占用的内存空间大小。</p>
                <pre><code class="language-cpp">struct StatusRegister { unsigned flag1:1; };</code></pre>
                <p>通过这种方式，我们可以将多个布尔标志或小范围整数打包到一个结构体内。</p>
                <p>这段说明文字足够长以通过提取阈值。继续补充一些内容，确保文本长度能稳定超过二百字符阈值，从而被容器提取逻辑采纳。</p>
            </div></div></body></html>"#;
        let got = extract_content_container(html).expect("container");
        assert!(!got.contains("<link"), "应清除残留的 <link>（CSDN 源站 CSS）");
        assert!(got.contains("什么是位域"), "应命中正文");
        assert!(got.contains("结构体成员"), "正文应完整");
    }

    #[test]
    fn clean_removes_inline_events() {
        let html = r#"<p onclick="alert(1)" onerror="x">正文</p>"#;
        let cleaned = clean_container(html);
        assert!(!cleaned.to_lowercase().contains("onclick"));
        assert!(!cleaned.to_lowercase().contains("onerror"));
        assert!(cleaned.contains("正文"));
    }

    #[test]
    fn clean_removes_comment_blocks() {
        // sspai 样式：article 内含 comment__list + common__comment__brief（评论/首评）
        let html = r#"<article>
            <h1>今日早报</h1>
            <p>正文内容一：这是一条新闻。</p>
            <div class="comment__list">
                <div class="common__comment__brief">
                    <span>发表首评</span>
                    <p>这是评论区内容不该被抓取</p>
                </div>
                <div class="common__comment__brief"><p>更多评论</p></div>
            </div>
            <p>正文内容二：另一条新闻在这里。</p>
            <div id="discussion"><p>讨论区也不该有</p></div>
        </article>"#;
        let cleaned = clean_container(html);
        assert!(!cleaned.contains("发表首评"), "应清评论区: {cleaned}");
        assert!(!cleaned.contains("comment__list"), "应清 comment__list: {cleaned}");
        assert!(!cleaned.contains("更多评论"), "应清评论内容");
        assert!(!cleaned.contains("discussion"), "应清 discussion");
        assert!(cleaned.contains("正文内容一"), "正文保留");
        assert!(cleaned.contains("正文内容二"), "正文保留");
    }

    #[test]
    fn clean_keeps_nested_article_body() {
        let html = r#"<article><div class="article-body"><h2>新闻</h2><p>长正文</p></div></article>"#;
        let cleaned = clean_container(html);
        assert!(cleaned.contains("article-body"), "不误删正文容器");
        assert!(cleaned.contains("新闻"));
    }

    #[test]
    fn clean_removes_promo_headings_and_charge() {
        // sspai 晨报：富文本里无 class 的推广 h2 块 + 充电/页脚区块
        let html = r#"<article>
            <h1>派早报</h1>
            <h2>阿里巴巴出售灵犀互娱</h2><p>正经新闻正文。</p>
            <h2>少数派的近期动态</h2><ul><li>新一季会员启航，点击了解</li></ul>
            <h2>你可能错过的文章</h2><ul><li>模块笔入门导购</li></ul>
            <div class="article__charge__card"><p>11位派友已充电</p></div>
            <div class="article__footer__editor"><span>本文责编</span></div>
            <div class="article__footer__copyright"><span>© 著作权</span></div>
        </article>"#;
        let cleaned = clean_container(html);
        assert!(cleaned.contains("阿里巴巴出售灵犀互娱"), "正文标题保留");
        assert!(cleaned.contains("正经新闻正文"), "正文保留");
        assert!(!cleaned.contains("近期动态"), "应清推广块: {cleaned}");
        assert!(!cleaned.contains("你可能错过"), "应清相关块: {cleaned}");
        assert!(!cleaned.contains("已充电"), "应清充电卡: {cleaned}");
        assert!(!cleaned.contains("本文责编"), "应清编辑页脚");
        assert!(!cleaned.contains("著作权"), "应清版权");
    }

    #[test]
    fn clean_removes_article_footers_without_promo() {
        // 无推广标题时，页脚/充电 div（class 特征）也要独立删除
        let html = r#"<article>
            <h2>正文新闻</h2><p>正常内容。</p>
            <div class="article__footer__editor"><span>本文责编</span></div>
            <div class="article__footer__copyright"><span>© 著作权</span></div>
            <div class="article__charge__card"><p>已充电</p></div>
        </article>"#;
        let cleaned = clean_container(html);
        assert!(cleaned.contains("正文新闻"), "正文保留");
        assert!(!cleaned.contains("本文责编"), "应清编辑页脚: {cleaned}");
        assert!(!cleaned.contains("著作权"), "应清版权");
        assert!(!cleaned.contains("已充电"), "应清充电卡");
    }

    #[test]
    fn clean_keeps_normal_headings() {
        let html = r#"<article><h2>本周最热新闻</h2><p>正文A</p><h3>深入报道</h3><p>正文B</p></article>"#;
        let cleaned = clean_container(html);
        assert!(cleaned.contains("本周最热新闻"), "正常 h2 保留（标题含'本周'不误伤）");
        assert!(cleaned.contains("深入报道"), "正常 h3 保留");
        assert!(cleaned.contains("正文A") && cleaned.contains("正文B"));
    }

    #[test]
    fn clean_keeps_inline_svg_charts() {
        // artificialanalysis 等站正文图表是内联 SVG，不应当噪声删除
        let html = r#"<article>
            <h1>模型对比</h1>
            <svg viewBox="0 0 100 100"><text>GLM-5.3</text></svg>
            <p>正文内容。</p>
            <svg viewBox="0 0 50 50"><rect width="50" height="50"/></svg>
        </article>"#;
        let cleaned = clean_container(html);
        assert_eq!(cleaned.matches("<svg").count(), 2, "SVG 图表应保留: {cleaned}");
        assert!(cleaned.contains("GLM-5.3"), "SVG 内文本保留");
        assert!(cleaned.contains("正文内容"));
    }

    #[test]
    fn detects_cloudflare_challenge() {
        assert!(detect_cloudflare("Just a moment... <title>Just a moment</title>"));
        assert!(detect_cloudflare("<html><script src=\"https://challenges.cloudflare.com/cdn-cgi/...\"></script>"));
        assert!(!detect_cloudflare("<html><title>正常页面</title><p>正文</p></html>"));
    }
}

/// 协议相对 URL（`//host/...`）补全为 `https://`。正文里的第三方 iframe（如
/// gcores 的 B 站视频 `//player.bilibili.com/...`）与资源 src 常是协议相对，
/// 不补全浏览器能加载但我们的媒体识别/后续处理拿不到绝对地址。
pub(crate) fn fix_protocol_relative(html: &str) -> String {
    html.replace("src=\"//", "src=\"https://").replace("src='//", "src='https://")
}
