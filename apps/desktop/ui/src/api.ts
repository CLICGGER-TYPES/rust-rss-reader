import { invoke } from "@tauri-apps/api/core";
import { log } from "./logger";

export interface Folder {
  id: number;
  name: string;
  position: number;
}

export interface Feed {
  id: number;
  folder_id: number | null;
  title: string;
  url: string;
  site_url: string | null;
  description: string | null;
  favicon_url: string | null;
  last_updated: string | null;
  error: string | null;
  refresh_interval: number | null;
  use_proxy: boolean;
  default_original: boolean;
}

export interface Article {
  id: number;
  feed_id: number;
  title: string | null;
  url: string | null;
  author: string | null;
  summary: string | null;
  content: string | null;
  content_fetched: boolean;
  published_at: string | null;
  fetched_at: string;
  is_read: boolean;
  is_starred: boolean;
  guid: string | null;
}

export interface FeedUnread {
  feed_id: number;
  folder_id: number | null;
  title: string;
  unread: number;
}

export interface UnreadStats {
  total: number;
  per_feed: FeedUnread[];
}

export interface AddFeedResult {
  feed: Feed;
  articles_new: number;
  existed: boolean;
}

export interface RefreshResult {
  feeds_checked: number;
  articles_new: number;
  errors: string[];
}

export interface OpmlImportResult {
  feeds_added: number;
  feeds_existing: number;
  errors: string[];
}

export interface PageResource {
  kind: "html" | "file";
  content_type: string;
  content: string;
  allow_embed: boolean;
}

export interface FetchedImage {
  content_type: string;
  data_b64: string;
}

export interface ArticleFilter {
  feedId?: number;
  folderId?: number;
  unreadOnly?: boolean;
  starredOnly?: boolean;
  search?: string;
  limit?: number;
  offset?: number;
  sort?: string;
}

/**
 * invoke 封装：每个命令调用前把实际参数记入日志。
 * 注意：Tauri 2 默认将 Rust 命令参数名(snake_case)映射为 camelCase 作为前端契约，
 * 所以这里传参键必须是 camelCase（fetchFull/feedId/...），否则后端报 missing required key。
 */
function ic<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  log(`invoke ${cmd}`, args);
  return invoke<T>(cmd, args);
}

export const api = {
  listFolders: () => ic<Folder[]>("list_folders"),
  addFolder: (name: string) => ic<number>("add_folder", { name }),
  renameFolder: (id: number, name: string) => ic<void>("rename_folder", { id, name }),
  removeFolder: (id: number) => ic<void>("remove_folder", { id }),

  listFeeds: () => ic<Feed[]>("list_feeds"),
  addFeed: (url: string, folderId: number | null, fetchFull = false) =>
    ic<AddFeedResult>("add_feed", { url, folderId, fetchFull }),
  removeFeed: (id: number) => ic<void>("remove_feed", { id }),
  setFeedFolder: (feedId: number, folderId: number | null) =>
    ic<void>("set_feed_folder", { feedId, folderId }),
  renameFeed: (feedId: number, title: string) => ic<void>("rename_feed", { feedId, title }),
  setFeedRefreshInterval: (feedId: number, minutes: number | null) =>
    ic<void>("set_feed_refresh_interval", { feedId, minutes }),
  setFeedUseProxy: (feedId: number, useProxy: boolean) =>
    ic<void>("set_feed_use_proxy", { feedId, useProxy }),
  setFeedDefaultOriginal: (feedId: number, on: boolean) =>
    ic<void>("set_feed_default_original", { feedId, on }),
  updateFeedUrl: (feedId: number, url: string) =>
    ic<void>("update_feed_url", { feedId, url }),
  pruneArticles: (days: number, includeUnread: boolean) =>
    ic<number>("prune_articles", { days, includeUnread }),
  testConnection: () => ic<string>("test_connection"),
  refresh: (fetchFull = false) => ic<RefreshResult>("refresh", { fetchFull }),
  refreshFeed: (id: number, fetchFull = false) => ic<number>("refresh_feed", { id, fetchFull }),

  listArticles: (filter: ArticleFilter = {}) =>
    ic<Article[]>("list_articles", {
      feedId: filter.feedId ?? null,
      folderId: filter.folderId ?? null,
      unreadOnly: filter.unreadOnly ?? false,
      starredOnly: filter.starredOnly ?? false,
      search: filter.search ?? null,
      limit: filter.limit ?? 200,
      offset: filter.offset ?? 0,
      sort: filter.sort ?? "desc",
    }),
  getArticle: (id: number) => ic<Article | null>("get_article", { id }),
  markRead: (ids: number[], read: boolean) => ic<void>("mark_read", { ids, read }),
  markFeedRead: (feedId: number) => ic<void>("mark_feed_read", { feedId }),
  markFolderRead: (folderId: number) => ic<void>("mark_folder_read", { folderId }),
  markAllRead: () => ic<void>("mark_all_read"),
  toggleStar: (id: number) => ic<boolean>("toggle_star", { id }),
  // 8s 超时：抓取进度在 UI 上有独立的 loadingFull 转圈，此处仅防止抓取挂起导致 refreshing 永久卡死
  fetchFullContent: (id: number) =>
    Promise.race([
      ic<boolean>("fetch_full_content", { id }),
      new Promise<never>((_, rej) => setTimeout(() => rej(new Error("fetch full timeout")), 8000)),
    ]),

  unreadStats: () => ic<UnreadStats>("unread_stats"),

  importOpml: (content: string) => ic<OpmlImportResult>("import_opml", { content }),
  exportOpml: () => ic<string>("export_opml"),
  importOpmlFrom: (path: string) => ic<OpmlImportResult>("import_opml_from", { path }),
  exportOpmlTo: (path: string) => ic<void>("export_opml_to", { path }),

  getSetting: (key: string) => ic<string | null>("get_setting", { key }),
  setSetting: (key: string, value: string) => ic<void>("set_setting", { key, value }),

  getProxy: () => ic<string | null>("get_proxy"),
  setProxy: (proxy: string | null) => ic<void>("set_proxy", { proxy }),
  clearContentCache: () => ic<number>("clear_content_cache"),
  getDataDir: () => ic<string>("get_data_dir"),
  setDataDir: (newDir: string, migrate: boolean) =>
    ic<string>("set_data_dir", { newDir, migrate }),
  getAppInfo: () =>
    ic<{ name: string; version: string; tauri_version: string; license: string; homepage: string }>(
      "get_app_info"
    ),
  // 打开原文的 CF 预检：8s 超时兜底，避免重 JS 站抓整页 HTML 卡住打开操作
  fetchOriginalHtml: (url: string) =>
    Promise.race([
      ic<PageResource>("fetch_original_html", { url }),
      new Promise<never>((_, rej) => setTimeout(() => rej(new Error("original html timeout")), 8000)),
    ]),
  fetchPageRendered: (url: string) => ic<PageResource>("fetch_page_rendered", { url }),
  probeUrl: (url: string) => ic<PageResource>("probe_url", { url }),
  writeTextFile: (path: string, content: string) => ic<void>("write_text_file", { path, content }),
  fetchImage: (url: string, referer?: string, maxWidth?: number) =>
    ic<FetchedImage>("fetch_image", { url, referer: referer ?? null, maxWidth: maxWidth ?? null }),
  openMediaWindow: (url: string, width?: number, height?: number) =>
    ic<void>("open_media_window", { url, width: width ?? null, height: height ?? null }),
};
