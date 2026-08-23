//! headless 浏览器渲染兜底（可选附加功能）。
//!
//! JS 渲染站点（Next.js / React SPA / gcores DraftJS 等）静态 HTML 没有正文，
//! 通用流水线（readability/容器）提取不到。这里复用**用户系统里已有的浏览器**
//! 以 headless 模式渲染页面，拿到渲染后的完整 DOM，再交给上游提取正文。
//!
//! 软依赖：探测不到可用浏览器（或 `RENDER_CMD` 未设置）时返回 `None`，调用方
//! 自动回退现有逻辑，不强求。支持的浏览器：
//! - Chromium / Google Chrome / Microsoft Edge：`--headless=new --dump-dom`
//! - Firefox：`--headless --dump-dom`
//! - Safari（macOS）无 headless CLI，不支持。

use std::process::Command;

/// 探测可用的 headless 渲染器。`RENDER_CMD` 环境变量优先，否则按平台探测。
pub(crate) fn detect_renderer() -> Option<String> {
    if let Ok(cmd) = std::env::var("RENDER_CMD") {
        let cmd = cmd.trim();
        if !cmd.is_empty() && find_executable(cmd) {
            return Some(cmd.to_string());
        }
    }
    #[cfg(target_os = "linux")]
    let candidates: &[&str] = &["chromium", "google-chrome", "chromium-browser", "firefox"];
    #[cfg(target_os = "windows")]
    let candidates: &[&str] = &["msedge", "chrome", "firefox"];
    #[cfg(target_os = "macos")]
    let candidates: &[&str] = &[
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        "firefox",
    ];
    #[allow(unreachable_code)]
    for c in candidates {
        if find_executable(c) {
            return Some(c.to_string());
        }
    }
    None
}

/// 检查可执行文件是否存在（含路径/或 PATH 内）。
fn find_executable(bin: &str) -> bool {
    if bin.contains('/') {
        return std::path::Path::new(bin).exists();
    }
    std::env::var_os("PATH").is_some_and(|p| {
        std::env::split_paths(&p).any(|dir| dir.join(bin).exists())
    })
}

/// 用 headless 浏览器渲染页面，返回渲染后的完整 HTML（DOM）。
pub(crate) fn render_dom(url: &str) -> Option<String> {
    let cmd = detect_renderer()?;
    let mut command = Command::new(&cmd);
    if cmd.contains("firefox") {
        // Firefox：--headless --dump-dom 直接输出 DOM
        command.args(["--headless", "--dump-dom", url]);
    } else {
        // Chromium 系：--headless=new + virtual-time-budget 等页面脚本跑完
        command.args([
            "--headless=new",
            "--disable-gpu",
            "--no-sandbox",
            "--disable-dev-shm-usage",
            "--virtual-time-budget=5000",
            "--dump-dom",
            url,
        ]);
    }
    let out = command.output().ok()?;
    if !out.status.success() {
        return None;
    }
    let html = String::from_utf8_lossy(&out.stdout).to_string();
    if html.trim().is_empty() {
        return None;
    }
    Some(html)
}
