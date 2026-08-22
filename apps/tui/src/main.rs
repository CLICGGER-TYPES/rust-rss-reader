mod app;
mod lang;
mod ui;

use std::io;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use app::{App, Focus, InputMode, Selection, SidebarItem, open_in_browser};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::DefaultTerminal;
use rss_core::{config, RssReader};

fn main() -> Result<()> {
    // --data-dir <path> 优先，其次配置文件，其次默认目录
    let mut cli_dir: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--data-dir" || a == "-d" {
            if let Some(v) = args.next() {
                cli_dir = Some(PathBuf::from(v));
            }
        }
    }
    let data_dir = cli_dir
        .or_else(|| std::env::var("RSS_READER_DATA_DIR").ok().map(PathBuf::from))
        .or_else(config::load_data_dir)
        .unwrap_or_else(|| {
            dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("rss-reader")
        });

    lang::init(Some(&data_dir));
    let reader = RssReader::with_data_dir(data_dir)?;
    // 启动时按共享设置清理旧文章
    prune_on_start(&reader);
    let mut app = App::new(reader);
    let mut terminal = ratatui::init();
    let run_result = run(&mut terminal, &mut app);
    ratatui::restore();
    run_result?;
    Ok(())
}

fn prune_on_start(reader: &RssReader) {
    if let Ok(Some(days)) = reader.get_setting("prune_days") {
        if let Ok(d) = days.parse::<i64>() {
            if d > 0 {
                let include_unread = reader
                    .get_setting("prune_include_unread")
                    .ok()
                    .flatten()
                    .as_deref()
                    == Some("1");
                let _ = reader.prune_articles(d, include_unread);
            }
        }
    }
}

fn run(terminal: &mut DefaultTerminal, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|frame| ui::draw(frame, app))?;
        app.tick();

        if !event::poll(Duration::from_millis(200))? {
            continue;
        }
        let evt = event::read()?;
        if let Event::Key(key) = evt {
            if key.kind == KeyEventKind::Press && handle_key(app, key)? {
                return Ok(());
            }
        }
    }
}

fn handle_key(app: &mut App, key: KeyEvent) -> io::Result<bool> {
    if app.help {
        match key.code {
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => app.help = false,
            _ => {}
        }
        return Ok(false);
    }

    if app.input_mode != InputMode::None {
        handle_input(app, key);
        return Ok(false);
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => return Ok(true),
        KeyCode::Char('?') => {
            app.help = true;
            return Ok(false);
        }
        KeyCode::Tab => {
            app.focus = match app.focus {
                Focus::Sidebar => Focus::List,
                Focus::List => Focus::Reader,
                Focus::Reader => Focus::Sidebar,
            };
            return Ok(false);
        }
        _ => {}
    }

    match app.focus {
        Focus::Sidebar => handle_sidebar(app, key),
        Focus::List => handle_list(app, key),
        Focus::Reader => handle_reader(app, key),
    }
    Ok(false)
}

fn handle_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::None;
            app.input.clear();
        }
        KeyCode::Enter => {
            let input = app.input.trim().to_string();
            let mode = std::mem::replace(&mut app.input_mode, InputMode::None);
            match mode {
                InputMode::AddFeed { folder_id } => {
                    if !input.is_empty() {
                        app.add_feed(&input, folder_id);
                    }
                }
                InputMode::AddFolder => {
                    if !input.is_empty() {
                        app.add_folder(&input);
                    }
                }
                InputMode::RenameFolder { id } => {
                    if !input.is_empty() {
                        app.rename_folder_ui(id, &input);
                    }
                }
                InputMode::MoveFeed { feed_id } => {
                    app.move_feed_ui(feed_id, &input);
                }
                InputMode::Search => {
                    app.search = if input.is_empty() {
                        None
                    } else {
                        Some(input)
                    };
                    app.reload_articles();
                }
                InputMode::ImportOpml => {
                    if !input.is_empty() {
                        app.import_opml(&input);
                    }
                }
                InputMode::ExportOpml => {
                    let path = if input.is_empty() {
                        App::data_dir().join("export.opml").display().to_string()
                    } else {
                        input
                    };
                    app.export_opml(&path);
                }
                InputMode::None => {}
            }
            app.input.clear();
        }
        KeyCode::Backspace => {
            app.input.pop();
        }
        KeyCode::Char(c) => {
            app.input.push(c);
        }
        _ => {}
    }
}

fn handle_sidebar(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Down | KeyCode::Char('j') => move_sidebar(app, 1),
        KeyCode::Up | KeyCode::Char('k') => move_sidebar(app, -1),
        KeyCode::Enter => {
            if let Some(item) = app.sidebar.get(app.sidebar_index) {
                if let Some(sel) = item.selection() {
                    app.selection = sel;
                    app.list_index = 0;
                    app.current_article_id = None;
                    app.reload_articles();
                    app.reload_reader();
                    app.focus = Focus::List;
                }
            }
        }
        KeyCode::Char('a') => {
            let folder_id = match &app.selection {
                Selection::Folder(id) => Some(*id),
                _ => None,
            };
            app.input_mode = InputMode::AddFeed { folder_id };
            app.input.clear();
        }
        KeyCode::Char('n') => {
            app.input_mode = InputMode::AddFolder;
            app.input.clear();
        }
        KeyCode::Char('d') => app.delete_current(),
        KeyCode::Char('r') => {
            let feed_id = match &app.selection {
                Selection::Feed(id) => Some(*id),
                _ => None,
            };
            app.spawn_refresh(feed_id);
        }
        KeyCode::Char('m') => app.mark_all_read_in_selection(),
        KeyCode::Char('M') => {
            if let Some(SidebarItem::Feed { id, .. }) = app.sidebar.get(app.sidebar_index).cloned() {
                app.input_mode = InputMode::MoveFeed { feed_id: id };
                app.input.clear();
            }
        }
        KeyCode::Char('R') => {
            if let Some(SidebarItem::Folder { id, .. }) = app.sidebar.get(app.sidebar_index).cloned() {
                app.input_mode = InputMode::RenameFolder { id };
                app.input.clear();
            }
        }
        KeyCode::Char('e') => {
            app.input_mode = InputMode::ExportOpml;
            app.input.clear();
        }
        KeyCode::Char('E') | KeyCode::Char('i') => {
            app.input_mode = InputMode::ImportOpml;
            app.input.clear();
        }
        _ => {}
    }
}

fn handle_list(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Down | KeyCode::Char('j') => move_list(app, 1),
        KeyCode::Up | KeyCode::Char('k') => move_list(app, -1),
        KeyCode::Char('g') => {
            app.list_index = 0;
            app.reload_reader();
        }
        KeyCode::Char('G') => {
            app.list_index = app.articles.len().saturating_sub(1);
            app.reload_reader();
        }
        KeyCode::Enter => app.open_article(),
        KeyCode::Char('s') => app.toggle_star_current(),
        KeyCode::Char(' ') => app.mark_current_read(!app.articles.get(app.list_index).map(|a| a.is_read).unwrap_or(false)),
        KeyCode::Char('x') => {
            app.mark_current_read(true);
            move_list(app, 1);
        }
        KeyCode::Char('f') => app.fetch_full_current(),
        KeyCode::Char('o') => {
            if let Some(a) = app.articles.get(app.list_index) {
                if let Some(url) = &a.url {
                    open_in_browser(url);
                    app.status = crate::lang::fmt("opening", &[("url", url)]);
                }
            }
        }
        KeyCode::Char('/') => {
            app.input_mode = InputMode::Search;
            app.input.clear();
        }
        KeyCode::Char('r') => app.spawn_refresh(None),
        KeyCode::Char('m') => app.mark_all_read_in_selection(),
        KeyCode::Esc => {
            app.focus = Focus::Sidebar;
        }
        _ => {}
    }
}

fn handle_reader(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Down | KeyCode::Char('j') => app.reader_scroll = app.reader_scroll.saturating_add(1),
        KeyCode::Up | KeyCode::Char('k') => app.reader_scroll = app.reader_scroll.saturating_sub(1),
        KeyCode::Char(' ') | KeyCode::PageDown => {
            app.reader_scroll = app.reader_scroll.saturating_add(20);
        }
        KeyCode::PageUp => app.reader_scroll = app.reader_scroll.saturating_sub(20),
        KeyCode::Char('g') => app.reader_scroll = 0,
        KeyCode::Char('G') => app.reader_scroll = u16::MAX,
        KeyCode::Char('s') => app.toggle_star_current(),
        KeyCode::Char('f') => app.fetch_full_current(),
        KeyCode::Char('o') => {
            if let Some(a) = app.articles.get(app.list_index) {
                if let Some(url) = &a.url {
                    open_in_browser(url);
                    app.status = crate::lang::fmt("opening", &[("url", url)]);
                }
            }
        }
        KeyCode::Esc | KeyCode::Backspace => {
            app.focus = Focus::List;
        }
        _ => {}
    }
}

fn move_sidebar(app: &mut App, delta: isize) {
    let len = app.sidebar.len();
    if len == 0 {
        return;
    }
    let mut idx = app.sidebar_index as isize + delta;
    while idx >= 0 && (idx as usize) < len && app.sidebar[idx as usize].is_section() {
        idx += delta;
    }
    if idx < 0 {
        idx = 0;
    }
    if idx as usize >= len {
        idx = (len - 1) as isize;
    }
    app.sidebar_index = idx as usize;
    if let Some(item) = app.sidebar.get(app.sidebar_index) {
        if let Some(sel) = item.selection() {
            if sel != app.selection {
                app.selection = sel;
                app.list_index = 0;
                app.current_article_id = None;
                app.reload_articles();
                app.reload_reader();
            }
        }
    }
}

fn move_list(app: &mut App, delta: isize) {
    if app.articles.is_empty() {
        return;
    }
    let len = app.articles.len() as isize;
    let next = (app.list_index as isize + delta).clamp(0, len - 1) as usize;
    if next != app.list_index {
        app.list_index = next;
        app.reload_reader();
    }
}
