import { invoke } from "@tauri-apps/api/core";

declare const __APP_BUILD_ID__: string;

export const BUILD_ID: string =
  typeof __APP_BUILD_ID__ !== "undefined" ? __APP_BUILD_ID__ : "dev";

const MAX_LINE = 2000;

function fmt(d: unknown): string {
  if (typeof d === "string") return d;
  try {
    return JSON.stringify(d);
  } catch {
    return String(d);
  }
}

function send(line: string) {
  invoke("log_to_file", { line }).catch(() => {
    /* 日志写入失败不阻塞业务 */
  });
}

/** 前端日志：console + 透传给后端写入统一日志文件（logs/app.log）。 */
export function log(msg: string, ...data: unknown[]): void {
  const ts = new Date().toISOString().slice(11, 23);
  let line = `[frontend] ${msg}`;
  if (data.length > 0) line += " | " + data.map(fmt).join(" | ");
  if (line.length > MAX_LINE) line = line.slice(0, MAX_LINE) + "…(truncated)";
  try {
    console.log(line);
  } catch {
    /* 忽略 */
  }
  send(line);
}

/** 捕获渲染进程未处理错误，写进日志便于排错。 */
export function installErrorHandlers(): void {
  window.addEventListener("error", (e) => {
    log("window.onerror", {
      message: e.message,
      filename: e.filename,
      lineno: e.lineno,
      colno: e.colno,
      error: String(e.error),
    });
  });
  window.addEventListener("unhandledrejection", (e) => {
    log("unhandledrejection", { reason: String(e.reason) });
  });
}

/** 操作级日志：方便按行为定位问题 */
export const opLog = {
  /** 用户点击某篇文章 */
  openArticle: (id: number, title: string, contentLen: number, contentFetched: boolean, summaryLen: number, action: "fetch" | "trust" | "skip" | "pending") =>
    log("openArticle", { id, title, contentLen, contentFetched, summaryLen, action }),
  /** 抓取全文结果 */
  fetchFull: (id: number, ok: boolean, contentLen: number, err?: string) =>
    log("fetchFull", { id, ok, contentLen, err }),
  /** 抓取进度 */
  imagesPending: (n: number) => log("images.pending", { n }),
  imagesDone: (done: number, total: number, failed: number, timeout: number) =>
    log("images.done", { done, total, failed, timeout }),
  /** 阅读器状态流转 */
  readerState: (state: { loadingFull?: boolean; imgLoading?: boolean; fetchFailed?: boolean; openOriginal?: boolean }) =>
    log("reader.state", state),
  /** 列表加载 */
  articlesLoaded: (count: number, hasMore: boolean) => log("articles.loaded", { count, hasMore }),
  /** 用户操作 */
  user: (action: string, detail?: unknown) => log("user", { action, detail }),
};
