//! Flashcat（flashcat.cloud）statuspage 状态页通用提取 + 美化提炼。
//!
//! 多家公司（如 DeepSeek）用 Flashcat 托管服务状态页（status.xxx.com/incidents/...），
//! 页面是服务端渲染：页头 h1 + 彩色状态 badge + 时间段 + 正文，下方是"更新时间线"
//!（Investigating → Identified → Monitoring → Resolved 每个阶段的 状态 + 时间 + 正文）。
//! 通用流水线（readability/容器选择器）因容器 class 通用常选不中，这里直接按结构
//! 提取并**保留页面原有的视觉结构**（状态色块、时间线），输出美观 HTML。

/// 页面是否 Flashcat statuspage（按静态资源/品牌特征探测）。
pub(crate) fn is_flashcat_page(html: &str) -> bool {
    html.contains("static.flashcat.cloud/statuspage")
        || html.contains("flashcat.cloud/statuspage")
}

/// 取某个开标签（如 `<h1 ...>`）内的文本。
fn between_tag(html: &str, open: &str, close: &str) -> Option<String> {
    let start = html.find(open)?;
    let gt = html[start..].find('>')? + start;
    let after = gt + 1;
    let end = html[after..].find(close)? + after;
    Some(html[after..end].trim().to_string())
}

/// 在锚点之后取第一个 `<span>…</span>` 的文本。
fn span_after(html: &str, anchor: &str) -> Option<String> {
    let a = html.find(anchor)? + anchor.len();
    let s = html[a..].find("<span>")? + a + "<span>".len();
    let e = html[s..].find("</span>")? + s;
    Some(html[s..e].trim().to_string())
}

/// 在锚点之后截取到闭合串之间的内容。
fn between_after<'a>(html: &'a str, anchor: &str, close: &str) -> Option<&'a str> {
    let a = html.find(anchor)? + anchor.len();
    let e = html[a..].find(close)? + a;
    Some(&html[a..e])
}

/// 状态 → 页面同款的 badge 配色（浅底 + 深字）。
fn status_badge(status: &str) -> String {
    let (bg, fg) = if status.contains("esolved") || status.contains("ompleted") {
        ("#dcfce7", "#166534") // 绿
    } else if status.contains("onitoring") {
        ("#dbeafe", "#1d4ed8") // 蓝
    } else if status.contains("nvestigating") {
        ("#fef3c7", "#92400e") // 琥珀
    } else if status.contains("dentified") {
        ("#ede9fe", "#5b21b6") // 紫
    } else {
        ("#f1f5f9", "#334155") // 灰
    };
    format!(
        "<span style=\"display:inline-block;background:{bg};color:{fg};border-radius:4px;padding:1px 8px;font-size:.9em;font-weight:600\">{}</span>",
        escape(status)
    )
}

/// 状态 → 时间线圆点颜色。
fn status_dot(status: &str) -> &'static str {
    if status.contains("esolved") || status.contains("ompleted") {
        "#22c55e"
    } else if status.contains("onitoring") {
        "#3b82f6"
    } else if status.contains("nvestigating") {
        "#f59e0b"
    } else if status.contains("dentified") {
        "#8b5cf6"
    } else {
        "#94a3b8"
    }
}

/// 提取"更新时间线"条目列表：(状态, 时间, 正文HTML)。
fn extract_timeline(html: &str) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    let block_anchor = "relative pl-8 pb-10";
    let status_anchor = "<span class=\"text-sm font-medium text-foreground\">";
    let time_anchor = "text-muted-foreground\"><span>";
    let body_anchor = "text-sm text-foreground\"><div>";
    while let Some(rel) = html[pos..].find(block_anchor) {
        let b = pos + rel;
        let Some(st) = html[b..].find(status_anchor) else { break };
        let st_b = b + st + status_anchor.len();
        let Some(st_e) = html[st_b..].find("</span>") else { break };
        let status = html[st_b..st_b + st_e].trim().to_string();
        let Some(tm) = html[st_b + st_e..].find(time_anchor) else { break };
        let tm_b = st_b + st_e + tm + time_anchor.len();
        let Some(tm_e) = html[tm_b..].find("</span>") else { break };
        let time = html[tm_b..tm_b + tm_e].trim().to_string();
        let Some(by) = html[tm_b + tm_e..].find(body_anchor) else { break };
        let by_b = tm_b + tm_e + by + body_anchor.len();
        let Some(by_e) = html[by_b..].find("</div></div>") else { break };
        let body_html = html[by_b..by_b + by_e].trim().to_string();
        if !status.is_empty() && !time.is_empty() && !body_html.is_empty() {
            out.push((status, time, body_html));
        }
        pos = by_b + by_e;
    }
    out
}

/// 提取 Flashcat incident 页面为美化后的阅读 HTML（标题 + 状态色块 + 时间段 + 正文 + 更新时间线）。
pub(crate) fn extract_flashcat_page(html: &str) -> Option<String> {
    let title = between_tag(html, "<h1", "</h1>")?;
    // 主状态 badge（页头彩色 badge，如 Resolved / Investigating…）
    let status = span_after(html, "rounded-md").unwrap_or_default();
    // 时间段（如 "Sat, Aug 22, 2026, 01:57 AM ~ ... (19min)"）
    let period = span_after(html, "text-muted-foreground flex-wrap").unwrap_or_default();
    // 主正文容器：`<div class="... pt-2"><div><p>…</p>…</div></div>`（锚点含 `>`，避免正文开头截入孤立 >）
    let body_anchor = "text-sm text-foreground pt-2\">";
    let body = between_after(html, body_anchor, "</div></div>").unwrap_or_default();
    let mut body_html = body.trim().to_string();
    if body_html.starts_with("<div>") {
        body_html = body_html[5..].trim().to_string();
    }
    if body_html.ends_with("</div>") {
        body_html = body_html[..body_html.len() - 6].trim().to_string();
    }
    if body_html.is_empty() {
        return None;
    }

    // 主区域：标题 + 状态 badge + 时间段 + 正文，包一层浅色圆角卡片（rgba 自适应深浅主题）
    let mut out = String::new();
    out.push_str("<div style=\"background:rgba(127,127,127,.06);border:1px solid rgba(127,127,127,.18);border-radius:12px;padding:14px 18px\">\n");
    out.push_str(&format!("  <h2 style=\"margin:0 0 8px\">{}</h2>\n", escape(&title)));
    let mut meta = String::new();
    if !status.is_empty() {
        meta.push_str(&status_badge(&status));
    }
    if !period.is_empty() {
        if !meta.is_empty() {
            meta.push_str(" &nbsp; ");
        }
        meta.push_str(&format!(
            "<span style=\"color:rgba(127,127,127,.8);font-size:.92em\">{}</span>",
            escape(&period)
        ));
    }
    if !meta.is_empty() {
        out.push_str(&format!("  <p style=\"margin:0 0 10px\">{meta}</p>\n"));
    }
    out.push_str(&body_html);
    out.push('\n');
    out.push_str("</div>\n");

    // 更新时间线（卡片式：左侧竖线 + 彩色圆点 + 浅色条目）
    let timeline = extract_timeline(html);
    if !timeline.is_empty() {
        out.push_str("<h3>更新时间线</h3>\n");
        out.push_str("<div style=\"border-left:3px solid rgba(127,127,127,.25);margin:8px 0 0 6px;padding-left:20px\">\n");
        for (st, tm, tb) in timeline {
            out.push_str("  <div style=\"position:relative;background:rgba(127,127,127,.06);border:1px solid rgba(127,127,127,.18);border-radius:10px;padding:10px 14px;margin:0 0 12px\">\n");
            out.push_str(&format!(
                "    <span style=\"position:absolute;left:-28px;top:13px;color:{};font-size:13px\">●</span>\n",
                status_dot(&st)
            ));
            out.push_str(&format!(
                "    <strong>{}</strong> <span style=\"color:rgba(127,127,127,.7);font-size:.85em\">· {}</span>\n",
                escape(&st),
                escape(&tm)
            ));
            out.push_str(&format!("    <div style=\"margin-top:4px\">{tb}</div>\n"));
            out.push_str("  </div>\n");
        }
        out.push_str("</div>\n");
    }
    Some(out)
}

/// HTML 转义文本（标题/状态/时间等来自页面，避免标签注入）。
fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}
