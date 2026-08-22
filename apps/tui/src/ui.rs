use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap,
};
use ratatui::Frame;

use crate::app::{App, Focus, InputMode, SidebarItem, short_date};
use crate::lang::t;

const ACCENT: Color = Color::Cyan;
const DIM: Color = Color::DarkGray;
const STAR: &str = "★ ";

fn block(title: &str, focused: bool) -> Block<'static> {
    let style = if focused {
        Style::default().fg(ACCENT)
    } else {
        Style::default().fg(DIM)
    };
    Block::default()
        .borders(Borders::ALL)
        .border_style(style)
        .title(format!(" {title} "))
}

fn focused(app: &App) -> bool {
    app.input_mode != InputMode::None || app.help
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let layout = Layout::vertical([Constraint::Min(4), Constraint::Length(1)]).split(area);
    let main = layout[0];
    let status_line = layout[1];

    let cols = Layout::horizontal([
        Constraint::Percentage(22),
        Constraint::Percentage(30),
        Constraint::Percentage(48),
    ])
    .split(main);

    draw_sidebar(frame, cols[0], app);
    draw_list(frame, cols[1], app);
    draw_reader(frame, cols[2], app);
    draw_status(frame, status_line, app);

    if app.input_mode != InputMode::None {
        draw_input(frame, app);
    }
    if app.help {
        draw_help(frame);
    }
}

// ---------- sidebar ----------

fn draw_sidebar(frame: &mut Frame, area: Rect, app: &mut App) {
    let focused = !focused(app) && app.focus == Focus::Sidebar;
    let title = if app.search.is_some() {
        format!("{} / {}", t("feeds"), t("unread"))
    } else {
        app.selection.label(&app.folders, &app.feeds)
    };

    let mut items: Vec<ListItem> = Vec::new();
    for item in &app.sidebar {
        items.push(match item {
            SidebarItem::Section(name) => ListItem::new(
                Line::from(Span::styled(
                    format!(" {name}"),
                    Style::default().fg(DIM).add_modifier(Modifier::BOLD),
                )),
            ),
            _ => {
                let label = match item {
                    SidebarItem::All(_) => t("all"),
                    SidebarItem::Unread(_) => t("unread"),
                    SidebarItem::Starred => t("starred"),
                    SidebarItem::Folder { name, .. } => format!("[{}]", name),
                    SidebarItem::Feed { name, .. } => format!("  {}", name),
                    SidebarItem::Section(_) => unreachable!(),
                };
                let unread = item.unread();
                let mut spans = Vec::new();
                if unread > 0 {
                    spans.push(Span::styled(
                        format!("{label:<28} {}", unread),
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                    ));
                } else {
                    spans.push(Span::styled(
                        format!("{label:<28}"),
                        Style::default().fg(Color::Gray),
                    ));
                }
                ListItem::new(Line::from(spans))
            }
        });
    }

    let mut state = ListState::default();
    state.select(Some(app.sidebar_index.min(app.sidebar.len().saturating_sub(1))));

    let list = List::new(items)
        .block(block(&title, focused))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::REVERSED),
        )
        .highlight_symbol("▸ ");
    frame.render_stateful_widget(list, area, &mut state);
}

// ---------- article list ----------

fn draw_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let focused = !focused(app) && app.focus == Focus::List;
    let unread_in_view = app.articles.iter().filter(|a| !a.is_read).count();
    let title = format!("{} ({}/{})", app.selection.label(&app.folders, &app.feeds), unread_in_view, app.articles.len());

    let items: Vec<ListItem> = app
        .articles
        .iter()
        .map(|a| {
            let title_style = if a.is_read {
                Style::default().fg(Color::Gray)
            } else {
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
            };
            let mut line1 = Vec::new();
            if a.is_starred {
                line1.push(Span::styled(STAR, Style::default().fg(Color::Yellow)));
            }
            line1.push(Span::styled(
                a.title.clone().unwrap_or_else(|| "(untitled)".to_string()),
                title_style,
            ));
            let feed_name = app
                .feeds
                .iter()
                .find(|f| f.id == a.feed_id)
                .map(|f| f.title.clone())
                .unwrap_or_default();
            let meta = match a.published_at {
                Some(dt) => format!("{} · {}", short_date(&dt), feed_name),
                None => feed_name,
            };
            let line2 = Line::from(Span::styled(meta, Style::default().fg(DIM)));
            ListItem::new(vec![Line::from(line1), line2])
        })
        .collect();

    let mut state = ListState::default();
    if !app.articles.is_empty() {
        state.select(Some(app.list_index.min(app.articles.len() - 1)));
    }

    let list = List::new(items)
        .block(block(&title, focused))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::REVERSED),
        )
        .highlight_symbol("▸ ");
    frame.render_stateful_widget(list, area, &mut state);
}

// ---------- reader ----------

fn draw_reader(frame: &mut Frame, area: Rect, app: &mut App) {
    let focused = !focused(app) && app.focus == Focus::Reader;

    // 组装排版后的正文：标题 + 元信息行 + 分隔 + 正文段落
    let content: String = if app.articles.get(app.list_index).is_some() {
        if app.reader_text.is_empty() {
            t("no_articles")
        } else {
            app.reader_text.clone()
        }
    } else {
        format!("{}\n{}", t("select_hint"), t("press_help"))
    };
    let mut text: Vec<Line> = Vec::new();

    if let Some(a) = app.articles.get(app.list_index) {
        let title = a.title.clone().unwrap_or_else(|| "(untitled)".to_string());
        text.push(Line::from(Span::styled(
            format!("{}{}", if a.is_starred { STAR } else { "" }, title),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        )));

        // 元信息：作者 · 日期 · 订阅源
        let feed_name = app
            .feeds
            .iter()
            .find(|f| f.id == a.feed_id)
            .map(|f| f.title.clone())
            .unwrap_or_default();
        let mut meta_parts: Vec<String> = Vec::new();
        if let Some(author) = a.author.as_deref() {
            meta_parts.push(format!("{} {}", t("by"), author));
        }
        if let Some(dt) = a.published_at {
            meta_parts.push(short_date(&dt));
        }
        if !feed_name.is_empty() {
            meta_parts.push(feed_name);
        }
        if !meta_parts.is_empty() {
            text.push(Line::from(Span::styled(
                meta_parts.join("  ·  "),
                Style::default().fg(DIM),
            )));
        }
        text.push(Line::from(Span::styled(
            "─".repeat(80),
            Style::default().fg(DIM),
        )));
        text.push(Line::default());

        for line in content.lines() {
            let is_heading = line.starts_with('#');
            let styled = if is_heading {
                Span::styled(
                    line.trim_start_matches('#').trim(),
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )
            } else {
                Span::raw(line)
            };
            text.push(Line::from(styled));
        }
    }

    let p = Paragraph::new(text)
        .block(block(&t("reader"), focused))
        .wrap(Wrap { trim: false })
        .scroll((app.reader_scroll, 0));
    frame.render_widget(p, area);
}

// ---------- status bar ----------

fn draw_status(frame: &mut Frame, area: Rect, app: &App) {
    let mut parts: Vec<Span> = Vec::new();
    let focus_name = match app.focus {
        Focus::Sidebar => "[sidebar]",
        Focus::List => "[list]",
        Focus::Reader => "[reader]",
    };
    parts.push(Span::styled(
        format!(" {} ", focus_name),
        Style::default().fg(ACCENT),
    ));
    parts.push(Span::styled(
        format!("{}:{} ", t("unread"), app.unread.total),
        Style::default().fg(Color::Yellow),
    ));
    if app.refreshing {
        parts.push(Span::styled("⟳ refreshing ", Style::default().fg(ACCENT)));
    }
    if !app.status.is_empty() {
        parts.push(Span::styled(&app.status, Style::default().fg(DIM)));
    }
    let line = Line::from(parts);
    frame.render_widget(
        Paragraph::new(line).block(Block::default().borders(Borders::TOP)),
        area,
    );
}

// ---------- input dialog ----------

fn input_popup(frame: &mut Frame, area: Rect, title: &str, text: &str) {
    let w = area.width.min(70);
    let h = 5;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + area.height.saturating_sub(h + 2);
    let popup = Rect::new(x, y, w, h);

    frame.render_widget(Clear, popup);
    let input_widget = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ACCENT))
                .title(format!(" {title} ")),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(input_widget, popup);

    // cursor
    if let Some((col, row)) = cursor_pos(text, popup) {
        frame.set_cursor_position((popup.x + col + 1, popup.y + row + 1));
    }
}

fn cursor_pos(text: &str, area: Rect) -> Option<(u16, u16)> {
    let width = area.width.saturating_sub(2).max(1) as usize;
    let lines: Vec<&str> = text
        .as_bytes()
        .chunks(width)
        .map(|c| std::str::from_utf8(c).unwrap_or(""))
        .collect();
    let row = lines.len().saturating_sub(1) as u16;
    let col = lines.last().map(|l| l.len() as u16).unwrap_or(0);
    Some((col, row))
}

fn draw_input(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let title = app.input_mode.title();
    input_popup(frame, area, &title, &app.input);
}

// ---------- help ----------

fn draw_help(frame: &mut Frame) {
    let area = frame.area();
    let w = 62;
    let h = 26;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let popup = Rect::new(x, y, w, h);

    frame.render_widget(Clear, popup);

    let lines = vec![
        "  ┌─ Navigation ───────────────────────────────────┐",
        "  │ Tab        cycle focus (sidebar/list/reader)   │",
        "  │ j / k      move down / up                      │",
        "  │ Enter      open article / activate selection   │",
        "  │ g / G      top / bottom of list                │",
        "  ├─ Actions ──────────────────────────────────────┤",
        "  │ a          add feed (folder prompt first)      │",
        "  │ n          new folder                          │",
        "  │ r          refresh (selected feed or all)      │",
        "  │ d          delete selected feed / folder       │",
        "  │ s          star / unstar article               │",
        "  │ space      toggle read status                  │",
        "  │ m          mark all read in current view       │",
        "  │ f          fetch full article content          │",
        "  │ o          open article in browser             │",
        "  │ /          search articles                     │",
        "  │ e / E      export / import OPML                │",
        "  │ ?          toggle this help                    │",
        "  │ q          quit                                │",
        "  └────────────────────────────────────────────────┘",
    ];
    let text = ratatui::text::Text::raw(lines.join("\n"));
    let p = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ACCENT))
                .title(" Key bindings "),
        )
        .style(Style::default().fg(Color::White));
    frame.render_widget(p, popup);
}
