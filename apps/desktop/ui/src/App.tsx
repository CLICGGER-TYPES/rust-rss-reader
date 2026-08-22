import React from "react";
import {
  makeStyles,
  tokens,
  Button,
  Input,
  SearchBox,
  Divider,
  Text,
  Badge,
  Menu,
  MenuTrigger,
  MenuPopover,
  MenuList,
  MenuItem,
  MenuItemCheckbox,
  Spinner,
  Dialog,
  DialogTrigger,
  DialogSurface,
  DialogBody,
  DialogTitle,
  DialogContent,
  DialogActions,
  Tooltip,
} from "@fluentui/react-components";
import {
  AddRegular,
  FolderAddRegular,
  RssRegular,
  StarRegular,
  StarFilled,
  ArrowSyncRegular,
  OpenRegular,
  ArrowImportRegular,
  ArrowExportRegular,
  DocumentSaveRegular,
  DocumentMarkdownRegular,
  DocumentPdfRegular,
  DocumentTextRegular,
  CodeRegular,
  DeleteRegular,
  WeatherMoonRegular,
  WeatherSunnyRegular,
  SearchRegular,
  CheckmarkCircleRegular,
  MoreHorizontalRegular,
  PanelLeftRegular,
  TextColumnOneRegular,
  TextBulletListRegular,
  GridRegular,
  BookRegular,
  SettingsRegular,
  DesktopRegular,
  CopyRegular,
  EyeRegular,
  ChevronLeftRegular,
  ChevronRightRegular,
  DismissRegular,
  HomeRegular,
  MailUnreadRegular,
  SubtractRegular,
  MaximizeRegular,
  SquareRegular,
} from "@fluentui/react-icons";
import { openUrl } from "@tauri-apps/plugin-opener";
import { save } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { api, Article, Feed, Folder, PageResource, UnreadStats } from "./api";
import { opLog } from "./logger";
import { useI18n } from "./i18n";
import SettingsDialog from "./settings";
import MediaDialog from "./components/MediaDialog";
import { ArticleCards } from "./components/ArticleCards";

type Selection =
  | { kind: "all" }
  | { kind: "unread" }
  | { kind: "starred" }
  | { kind: "folder"; id: number }
  | { kind: "feed"; id: number };

type ViewMode = "list" | "compact" | "card" | "magazine";

type ResizeDir = "North" | "South" | "East" | "West" | "NorthEast" | "NorthWest" | "SouthEast" | "SouthWest";

const EDGE = 8;
const RESIZE_CURSOR: Record<ResizeDir, string> = {
  North: "ns-resize",
  South: "ns-resize",
  East: "ew-resize",
  West: "ew-resize",
  NorthEast: "nesw-resize",
  SouthWest: "nesw-resize",
  NorthWest: "nwse-resize",
  SouthEast: "nwse-resize",
};

/** 无边框窗口：四边透明覆盖条提供 resize 光标 + startResizeDragging（WebView 内容/iframe 会吞掉边缘事件，必须用覆盖层）。 */
function EdgeResizer() {
  const dirRef = React.useRef<ResizeDir | null>(null);
  const compute = (e: React.MouseEvent): ResizeDir | null => {
    const x = e.clientX;
    const y = e.clientY;
    const w = window.innerWidth;
    const h = window.innerHeight;
    const l = x <= EDGE;
    const r = x >= w - EDGE;
    const t = y <= EDGE;
    const b = y >= h - EDGE;
    if (l && t) return "NorthWest";
    if (r && t) return "NorthEast";
    if (l && b) return "SouthWest";
    if (r && b) return "SouthEast";
    if (l) return "West";
    if (r) return "East";
    if (t) return "North";
    if (b) return "South";
    return null;
  };
  const onMove = (e: React.MouseEvent) => {
    const d = compute(e);
    dirRef.current = d;
    document.body.style.cursor = d ? RESIZE_CURSOR[d] : "";
  };
  const onDown = (e: React.MouseEvent) => {
    const d = dirRef.current;
    if (!d) return;
    e.preventDefault();
    document.body.style.cursor = "";
    getCurrentWindow()
      .startResizeDragging(d)
      .catch(() => {});
  };
  const onLeave = () => {
    dirRef.current = null;
    document.body.style.cursor = "";
  };
  const side = (style: React.CSSProperties) => ({ position: "fixed" as const, zIndex: 99999, ...style });
  return (
    <>
      <div style={side({ top: 0, left: 0, right: 0, height: EDGE })} onMouseMove={onMove} onMouseDown={onDown} onMouseLeave={onLeave} />
      <div style={side({ bottom: 0, left: 0, right: 0, height: EDGE })} onMouseMove={onMove} onMouseDown={onDown} onMouseLeave={onLeave} />
      <div style={side({ top: 0, left: 0, bottom: 0, width: EDGE })} onMouseMove={onMove} onMouseDown={onDown} onMouseLeave={onLeave} />
      <div style={side({ top: 0, right: 0, bottom: 0, width: EDGE })} onMouseMove={onMove} onMouseDown={onDown} onMouseLeave={onLeave} />
    </>
  );
}

interface Props {
  dark: boolean;
  themeMode: "system" | "light" | "dark";
  setThemeMode: (m: "system" | "light" | "dark") => void;
  decorations: boolean;
  setDecorations: (d: boolean) => void;
}

const AUTO_REFRESH_MS = 30 * 60 * 1000;
const PAGE_SIZE = 30;

const useStyles = makeStyles({
  root: {
    display: "flex",
    flexDirection: "column",
    height: "100vh",
  },
  toolbarRow: {
    display: "flex",
    alignItems: "center",
    gap: "6px",
    flexWrap: "wrap",
    padding: "4px 10px",
    borderBottom: `1px solid ${tokens.colorNeutralStroke1}`,
    flexShrink: 0,
  },
  statusBar: {
    display: "flex",
    alignItems: "center",
    gap: "8px",
    padding: "3px 10px",
    borderTop: `1px solid ${tokens.colorNeutralStroke1}`,
    backgroundColor: tokens.colorNeutralBackground2,
    flexShrink: 0,
    minHeight: "22px",
  },
  statusIcon: {
    flexShrink: 0,
  },
  statusText: {
    flex: 1,
    whiteSpace: "nowrap",
    overflow: "hidden",
    textOverflow: "ellipsis",
  },
  body: {
    display: "flex",
    flex: 1,
    minHeight: 0,
  },
  sidebar: {
    width: "240px",
    minWidth: "240px",
    borderRight: `1px solid ${tokens.colorNeutralStroke1}`,
    overflowY: "auto",
    padding: "6px",
    backgroundColor: tokens.colorNeutralBackground2,
    transition: "width 0.15s ease, min-width 0.15s ease",
    overflowX: "hidden",
  },
  sidebarCollapsed: {
    width: "0px",
    minWidth: "0px",
    padding: 0,
    borderRight: "none",
  },
  viewSwitcher: {
    display: "flex",
    gap: "2px",
    padding: "4px 6px 6px",
    borderBottom: `1px solid ${tokens.colorNeutralStroke1}`,
  },
  viewBtn: {
    flex: 1,
    minWidth: 0,
  },
  titleBar: {
    display: "flex",
    alignItems: "center",
    gap: "6px",
    padding: "4px 10px",
    borderBottom: `1px solid ${tokens.colorNeutralStroke1}`,
    flexShrink: 0,
    flexWrap: "wrap",
  },
  appTitle: {
    marginLeft: "6px",
    userSelect: "none",
  },
  spacer: {
    flex: 1,
  },
  fullContent: {
    flex: 1,
    display: "flex",
    flexDirection: "column",
    minHeight: 0,
    width: "100%",
  },
  searchBar: {
    display: "flex",
    gap: "8px",
    alignItems: "center",
    padding: "8px 12px",
    flexShrink: 0,
  },
  searchBox: {
    flex: 1,
    maxWidth: "520px",
  },
  // ---- 浮层 ----
  overlay: {
    position: "fixed",
    inset: 0,
    zIndex: 100,
  },
  overlayScrim: {
    position: "absolute",
    inset: 0,
    backgroundColor: "rgba(0,0,0,0.45)",
  },
  overlayPanel: {
    position: "absolute",
    inset: "4vh 6vw",
    backgroundColor: tokens.colorNeutralBackground1,
    borderRadius: "12px",
    boxShadow: "0 24px 60px rgba(0,0,0,0.35)",
    display: "flex",
    flexDirection: "column",
    overflow: "hidden",
  },
  overlayTop: {
    display: "flex",
    alignItems: "center",
    gap: "6px",
    padding: "6px 10px",
    borderBottom: `1px solid ${tokens.colorNeutralStroke1}`,
    flexShrink: 0,
    flexWrap: "wrap",
  },
  overlaySource: {
    marginRight: "auto",
  },
  overlayFrame: {
    flex: 1,
    width: "100%",
    border: "none",
    background: "transparent",
    minHeight: 0,
  },
  overlayNav: {
    position: "absolute",
    top: "50%",
    transform: "translateY(-50%)",
    borderRadius: "50%",
    boxShadow: "0 2px 10px rgba(0,0,0,0.25)",
  },
  overlayNavPrev: {
    left: "12px",
  },
  overlayNavNext: {
    right: "12px",
  },
  // ---- 抽屉（非 List 视图的侧边栏） ----
  drawerScrim: {
    position: "absolute",
    inset: 0,
    zIndex: 50,
    backgroundColor: "rgba(0,0,0,0.3)",
  },
  drawer: {
    position: "absolute",
    left: 0,
    top: 0,
    bottom: 0,
    width: "240px",
    backgroundColor: tokens.colorNeutralBackground2,
    borderRight: `1px solid ${tokens.colorNeutralStroke1}`,
    zIndex: 51,
    overflowY: "auto",
    padding: "6px",
    boxShadow: "4px 0 20px rgba(0,0,0,0.15)",
  },
  sectionHeader: {
    padding: "8px 8px 2px",
  },
  navRow: {
    display: "flex",
    alignItems: "center",
    gap: "6px",
    padding: "6px 8px",
    borderRadius: "4px",
    cursor: "pointer",
    color: tokens.colorNeutralForeground1,
    "&:hover": { backgroundColor: tokens.colorNeutralBackground1Hover },
  },
  navRowSelected: {
    backgroundColor: tokens.colorBrandBackground2,
  },
  navLabel: {
    flex: 1,
    minWidth: 0,
    overflow: "hidden",
    textOverflow: "ellipsis",
    whiteSpace: "nowrap",
  },
  navCount: {
    color: tokens.colorBrandForeground1,
  },
  feedIndent: {
    paddingLeft: "20px",
  },
  favicon: {
    width: "16px",
    height: "16px",
    borderRadius: "3px",
    flexShrink: 0,
  },
  rowActions: {
    opacity: 0.55,
    transition: "opacity 0.15s ease",
    "&:hover": { opacity: 1 },
  },
  rowActionsVisible: {
    opacity: 1,
  },
  actionBtn: {
    minWidth: "24px",
    padding: "0 4px",
  },
  middle: {
    width: "360px",
    minWidth: "280px",
    display: "flex",
    flexDirection: "column",
    minHeight: 0,
    borderRight: `1px solid ${tokens.colorNeutralStroke1}`,
    flexShrink: 0,
  },
  search: {
    padding: "8px",
    flexShrink: 0,
  },
  filters: {
    display: "flex",
    gap: "8px",
    padding: "0 8px 8px",
    flexShrink: 0,
  },
  list: {
    flex: 1,
    overflowY: "auto",
    minHeight: 0,
  },
  // ---- 视图样式 ----
  cardGrid: {
    display: "grid",
    gridTemplateColumns: "repeat(auto-fill, minmax(160px, 1fr))",
    gap: "10px",
    padding: "10px",
  },
  card: {
    border: `1px solid ${tokens.colorNeutralStroke2}`,
    borderRadius: "6px",
    overflow: "hidden",
    cursor: "pointer",
    background: tokens.colorNeutralBackground1,
    "&:hover": { border: `1px solid ${tokens.colorBrandStroke1}` },
  },
  magazineMixGrid: {
    display: "grid",
    gridTemplateColumns: "repeat(4, 1fr)",
    gridAutoRows: "auto",
    gap: "12px",
    padding: "12px",
    alignItems: "start",
  },
  reader: {
    flex: 1,
    display: "flex",
    flexDirection: "column",
    minHeight: 0,
    padding: "8px 12px",
  },
  readerHeader: {
    marginBottom: "8px",
  },
  readerMeta: {
    display: "flex",
    alignItems: "center",
    gap: "10px",
    flexWrap: "wrap",
  },
  readerButtons: {
    marginBottom: "12px",
    display: "flex",
    gap: "6px",
    flexWrap: "wrap",
    alignItems: "center",
  },
  article: {
    lineHeight: 1.65,
    color: tokens.colorNeutralForeground1,
    "& img": { maxWidth: "100%" },
    "& pre": {
      backgroundColor: tokens.colorNeutralBackground2,
      padding: "12px",
      overflowX: "auto",
    },
    "& a": { color: tokens.colorBrandForeground1 },
  },
  empty: {
    display: "flex",
    flex: 1,
    alignItems: "center",
    justifyContent: "center",
    color: tokens.colorNeutralForeground3,
  },
  toolbarBtn: {
    minWidth: 0,
  },
});

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

// ---- 图片代理 / 媒体识别辅助 ----

function extractImgUrls(html: string): string[] {
  const urls: string[] = [];
  const re = /<img\b[^>]*?\bsrc=["']([^"']+)["']/gi;
  let m: RegExpExecArray | null;
  while ((m = re.exec(html)) !== null) {
    const u = m[1];
    if (u && (u.startsWith("http://") || u.startsWith("https://"))) urls.push(u);
  }
  return urls;
}

function replaceImages(html: string, map: Record<string, string>): string {
  if (Object.keys(map).length === 0) return html;
  let out = html;
  for (const [u, blob] of Object.entries(map)) {
    out = out.split(`src="${u}"`).join(`src="${blob}"`);
  }
  return out;
}

const MEDIA_HOSTS = [
  "music.163.com",
  "youtube.com",
  "youtu.be",
  "www.youtube.com",
  "player.bilibili.com",
  "bilibili.com",
  "vimeo.com",
  "player.vimeo.com",
  "spotify.com",
  "open.spotify.com",
  "soundcloud.com",
  "xiaoyuzhoufm.com",
  "podcasts.apple.com",
  "music.apple.com",
  "v.qq.com",
  "youku.com",
  "player.youku.com",
  "ixigua.com",
  "iqiyi.com",
  "tv.cctv.com",
];

function hostOf(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return "";
  }
}

function isMediaHost(url: string): boolean {
  const h = hostOf(url);
  return MEDIA_HOSTS.some((m) => h === m || h.endsWith("." + m));
}

/** 提取正文里的第三方播放器 URL（白名单 iframe）。直链音/视频保留原生控件，不收集。 */
function extractMediaUrls(html: string): string[] {
  const urls: string[] = [];
  const iframeRe = /<iframe\b[^>]*?\bsrc=["']([^"']+)["']/gi;
  let m: RegExpExecArray | null;
  while ((m = iframeRe.exec(html)) !== null) {
    if (isMediaHost(m[1])) urls.push(m[1]);
  }
  return urls;
}

/** 白名单第三方播放器 iframe → 内嵌"▶ 播放"按钮（点击 postMessage 给父页面弹窗播放）。 */
function replaceMediaIframes(html: string): string {
  const iframeRe = /<iframe\b[^>]*?>.*?<\/iframe>|<iframe\b[^>]*?\/>/gis;
  let idx = -1;
  const seen = new Map<string, number>();
  return html.replace(iframeRe, (tag) => {
    const srcM = tag.match(/src=["']([^"']+)["']/i);
    if (!srcM || !isMediaHost(srcM[1])) return tag;
    let n = seen.get(srcM[1]);
    if (n === undefined) {
      idx += 1;
      n = idx;
      seen.set(srcM[1], n);
    }
    const host = hostOf(srcM[1]);
    return `<p style="margin:14px 0;text-align:center"><button class="rss-play" data-idx="${n}" style="padding:8px 18px;border:none;border-radius:20px;background:#3a7bd5;color:#fff;font-size:14px;cursor:pointer">▶ 播放 · ${host}</button></p>`;
  });
}

/** base64 → data URL（体积超限返回空，保留原 URL）。 */
function b64ToDataUrl(img: { content_type: string; data_b64: string }): string {
  // 上限放宽到 16MB base64（约 12MB 原图），避免 geekpark 等大图因降级原 URL 而裂
  if (!img.data_b64 || img.data_b64.length > 16_000_000) return "";
  return `data:${img.content_type || "image/png"};base64,${img.data_b64}`;
}

function firstImage(html: string | null): string | null {
  if (!html) return null;
  const m = html.match(/<img[^>]+src=["']([^"']+)["']/i);
  return m ? m[1] : null;
}

/** 清洗 RSS 摘要里的订阅源模板行（HN：Article URL/Comments URL/Points/# Comments 等）。 */
function stripFeedTemplate(html: string): string {
  return html
    .split("\n")
    .map((line) => {
      const l = line.replace(/<[^>]+>/g, "").trim().toLowerCase();
      if (
        l.startsWith("article url:") ||
        l.startsWith("comments url:") ||
        l.startsWith("points:") ||
        l.startsWith("# comments:") ||
        l === "url:" ||
        l === "hn:" ||
        (l.startsWith("url:") && !l.includes("http"))
      ) {
        return "";
      }
      return line;
    })
    .join("\n");
}

function htmlToMarkdown(html: string): string {
  const doc = new DOMParser().parseFromString(html, "text/html");
  const out: string[] = [];
  const walk = (node: ChildNode, depth: number) => {
    node.childNodes.forEach((child) => {
      const el = child as HTMLElement;
      const tag = (el.tagName ?? "").toLowerCase();
      const isBlock = ["DIV", "P", "LI", "PRE", "BLOCKQUOTE", "H1", "H2", "H3", "H4", "H5", "H6"].includes(el.tagName ?? "");
      if (el.nodeType === Node.TEXT_NODE) {
        const t = el.textContent ?? "";
        if (t.trim()) out.push(t.trim() + " ");
        return;
      }
      if (!tag) return;
      if (/^h[1-6]$/.test(tag)) {
        const level = Number(tag[1]);
        out.push("\n" + "#".repeat(level) + " " + (el.textContent ?? "").trim() + "\n\n");
        return;
      }
      if (tag === "a") {
        const href = el.getAttribute("href");
        out.push(`[${el.textContent}](${href})`);
        return;
      }
      if (tag === "img") {
        const src = el.getAttribute("src");
        if (src) out.push(`![image](${src}) `);
        return;
      }
      if (tag === "strong" || tag === "b") {
        out.push(`**${el.textContent}**`);
        return;
      }
      if (tag === "em" || tag === "i") {
        out.push(`*${el.textContent}*`);
        return;
      }
      if (tag === "code") {
        out.push(`\`${el.textContent}\``);
        return;
      }
      if (tag === "br") {
        out.push("\n");
        return;
      }
      if (isBlock) {
        walk(child, depth + 1);
        if (tag === "LI") out.push("\n");
        else if (tag !== "DIV") out.push("\n");
        return;
      }
      walk(child, depth + 1);
    });
  };
  walk(doc.body, 0);
  return out
    .join("")
    .replace(/\n{3,}/g, "\n\n")
    .replace(/ {2,}/g, " ")
    .trim();
}

export default function App({ dark, themeMode, setThemeMode, decorations, setDecorations }: Props) {
  const styles = useStyles();
  const { t } = useI18n();

  const [isMaximized, setIsMaximized] = React.useState(false);
  React.useEffect(() => {
    if (decorations) return;
    const w = getCurrentWindow();
    w.isMaximized().then(setIsMaximized).catch(() => {});
    const un = w.onResized(() => {
      w.isMaximized().then(setIsMaximized).catch(() => {});
    });
    return () => {
      un.then((f) => f()).catch(() => {});
    };
  }, [decorations]);

  // 自绘标题栏拖拽：mousedown 触发 startDragging（交互元素除外），解决 data-tauri-drag-region 被按钮拦截无法拖动
  const onTitleBarMouseDown = (e: React.MouseEvent) => {
    if (decorations) return;
    if (e.button !== 0) return;
    const t = e.target as HTMLElement;
    // 排除交互元素 + 菜单/菜单项（Fluent MenuPopover 渲染在触发器附近，冒泡到标题栏，
    // 若触发 startDragging 会吞掉菜单项点击 → 视图切换失效）
    if (t.closest("button, input, select, a, [role='tab'], [role^='menu'], [role^='menuitem']")) return;
    getCurrentWindow().startDragging();
  };

  const [folders, setFolders] = React.useState<Folder[]>([]);
  const [feeds, setFeeds] = React.useState<Feed[]>([]);
  const [unread, setUnread] = React.useState<UnreadStats>({ total: 0, per_feed: [] });
  const [selection, setSelection] = React.useState<Selection>({ kind: "all" });
  const [articles, setArticles] = React.useState<Article[]>([]);
  const [selectedArticleId, setSelectedArticleId] = React.useState<number | null>(null);
  // 渲染时同步当前选中文章 id：async 闭包里判断"是否仍是当前文章"用
  const selectedArticleIdRef = React.useRef<number | null>(null);
  selectedArticleIdRef.current = selectedArticleId;
  // 抓取/图片加载的运行序号：重试或内容更新后旧批不碰新状态
  const fetchRunRef = React.useRef(0);
  const imgRunRef = React.useRef(0);
  // 进行中的图片 url 集合：pending 为空 ≠ 加载完，必须等 inflight 清空
  const imgInFlightRef = React.useRef<Set<string>>(new Set());
  const [search, setSearch] = React.useState("");
  const [refreshing, setRefreshing] = React.useState(false);
  const [status, setStatus] = React.useState("");
  const [showAddFeed, setShowAddFeed] = React.useState(false);
  const [showAddFolder, setShowAddFolder] = React.useState(false);
  const [showSettings, setShowSettings] = React.useState(false);
  const [showHelp, setShowHelp] = React.useState(false);
  const [addFeedUrl, setAddFeedUrl] = React.useState("");
  const [addFolderName, setAddFolderName] = React.useState("");
  const [viewMode, setViewMode] = React.useState<ViewMode>("list");
  const [cardSize, setCardSize] = React.useState<"small" | "medium" | "large">("medium");
  const [sidebarCollapsed, setSidebarCollapsed] = React.useState(false);
  const [drawerOpen, setDrawerOpen] = React.useState(false);
  const [fontSize, setFontSize] = React.useState(16);
  const [openOriginal, setOpenOriginal] = React.useState(false);
  const [originalBlocked, setOriginalBlocked] = React.useState(false);
  const [fileView, setFileView] = React.useState<PageResource | null>(null);
  const [loadingOriginal, setLoadingOriginal] = React.useState(false);
  const [loadingFull, setLoadingFull] = React.useState(false);
  const [fetchFailed, setFetchFailed] = React.useState(false);
  const [fullRetryCount, setFullRetryCount] = React.useState(0);
  const [hasMore, setHasMore] = React.useState(false);
  const [loadingMore, setLoadingMore] = React.useState(false);
  const [sortOrder, setSortOrder] = React.useState("desc");
  const sortOrderRef = React.useRef("desc");
  sortOrderRef.current = sortOrder;
  const [imgMap, setImgMap] = React.useState<Record<string, string>>({});
  const [imgStatus, setImgStatus] = React.useState("");
  const [imgLoading, setImgLoading] = React.useState(false);
  const [mediaUrls, setMediaUrls] = React.useState<string[]>([]);
  const [playIdx, setPlayIdx] = React.useState<number | null>(null);
  const fetchedImgsRef = React.useRef<Set<string>>(new Set());
  const imgRetriesRef = React.useRef<Record<string, number>>({});
  const listRef = React.useRef<HTMLDivElement | null>(null);
  const fetchedFullRef = React.useRef<Set<number>>(new Set());
  const articlesRef = React.useRef<Article[]>([]);
  articlesRef.current = articles;

  const selectedArticle = React.useMemo(
    () => (selectedArticleId != null ? articles.find((a) => a.id === selectedArticleId) ?? null : null),
    [articles, selectedArticleId]
  );

  const visibleArticles = React.useMemo(() => articles, [articles]);

  const unreadByFeed = React.useMemo(() => {
    const m = new Map<number, number>();
    for (const u of unread.per_feed) m.set(u.feed_id, u.unread);
    return m;
  }, [unread]);

  const reloadData = React.useCallback(async () => {
    try {
      const [f, feeds2, u] = await Promise.all([
        api.listFolders(),
        api.listFeeds(),
        api.unreadStats(),
      ]);
      setFolders(f);
      setFeeds(feeds2);
      setUnread(u);
    } catch (e) {
      setStatus("ERR: " + e);
    }
  }, []);

  // 清缓存：清空文章 + 全部抓取缓存 → 全局强制刷新 → 重载列表，带进度反馈
  const clearCacheAndRefresh = async () => {
    setRefreshing(true);
    setStatus(t("status.clearingCache"));
    try {
      const n = await api.clearContentCache();
      // 立即清空前端列表，明确"已清空"反馈
      setArticles([]);
      setSelectedArticleId(null);
      setOpenOriginal(false);
      setStatus(`${t("status.clearingCache")} · ${t("status.refreshing")}`);
      // 只重抓各源 RSS 文章元数据（不 backfill 全文，避免网络风暴卡死）；
      // 全文在打开文章时按需抓取（content_fetched=0 保证自动抓）
      const res = await api.refresh(false);
      setStatus(
        t("status.clearedRefreshed", { n, m: res.articles_new }) +
          (res.errors.length ? t("status.errors", { n: res.errors.length }) : ""),
      );
    } catch (e) {
      setStatus(t("status.clearCacheFailed") + e);
    } finally {
      // 无论成败都同步侧边栏计数 + 文章列表，避免"源空/首页残留旧文章"
      await Promise.all([reloadData(), loadArticles(selection, search)]).catch(() => {});
      setRefreshing(false);
    }
  };

  React.useEffect(() => {
    api.getSetting("sort").then((v) => {
      if (["asc", "unread", "starred", "title"].includes(v || "")) setSortOrder(v as string);
    });
    api.getSetting("cardSize").then((v) => {
      if (v === "small" || v === "large") setCardSize(v);
    });
  }, []);

  const loadArticles = React.useCallback(async (sel: Selection, q: string, sortOverride?: string) => {
    try {
      const sort = sortOverride ?? sortOrderRef.current;
      const filter: Record<string, unknown> = { search: q || null, limit: PAGE_SIZE, offset: 0, sort };
      if (sel.kind === "feed") filter.feedId = sel.id;
      if (sel.kind === "folder") filter.folderId = sel.id;
      if (sel.kind === "unread") filter.unreadOnly = true;
      if (sel.kind === "starred") filter.starredOnly = true;
      const list = await api.listArticles(filter as never);
      setArticles(list);
      setHasMore(list.length >= PAGE_SIZE);
      opLog.articlesLoaded(list.length, list.length >= PAGE_SIZE);
      setSelectedArticleId(null);
      setOpenOriginal(false);
      setFileView(null);
    } catch (e) {
      setStatus("ERR: " + e);
    }
  }, []);

  const loadMore = React.useCallback(async () => {
    if (loadingMore || !hasMore) return;
    setLoadingMore(true);
    try {
      const filter: Record<string, unknown> = {
        search: search || null,
        limit: PAGE_SIZE,
        offset: articlesRef.current.length,
        sort: sortOrderRef.current,
      };
      if (selection.kind === "feed") filter.feedId = selection.id;
      if (selection.kind === "folder") filter.folderId = selection.id;
      if (selection.kind === "unread") filter.unreadOnly = true;
      if (selection.kind === "starred") filter.starredOnly = true;
      const list = await api.listArticles(filter as never);
      setArticles((prev) => [...prev, ...list]);
      setHasMore(list.length >= PAGE_SIZE);
      opLog.articlesLoaded(list.length, list.length >= PAGE_SIZE);
    } catch (e) {
      setStatus("ERR: " + e);
    } finally {
      setLoadingMore(false);
    }
  }, [loadingMore, hasMore, search, selection]);

  const scrollRafRef = React.useRef(0);
  const onListScroll = React.useCallback(() => {
    // rAF 节流：避免滚动事件里同步触发 loadMore 导致卡顿
    if (scrollRafRef.current) return;
    scrollRafRef.current = requestAnimationFrame(() => {
      scrollRafRef.current = 0;
      const el = listRef.current;
      if (!el) return;
      if (el.scrollTop + el.clientHeight >= el.scrollHeight - 120) {
        loadMore();
      }
    });
  }, [loadMore]);

  React.useEffect(() => {
    reloadData();
  }, [reloadData]);

  React.useEffect(() => {
    loadArticles(selection, search);
  }, [selection, loadArticles, search]);

  React.useEffect(() => {
    const id = window.setInterval(() => doRefresh(false, true), AUTO_REFRESH_MS);
    return () => window.clearInterval(id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const selectArticle = async (a: Article) => {
    setSelectedArticleId(a.id);
    setOpenOriginal(false);
    setFileView(null);
    setLoadingFull(false);
    setLoadingOriginal(false);
    if (!a.is_read) {
      try {
        await api.markRead([a.id], true);
      } catch (e) {
        setStatus("ERR: " + e);
      }
      setArticles((prev) => prev.map((x) => (x.id === a.id ? { ...x, is_read: true } : x)));
      await reloadData();
    }
  };

  // 打开文章时若正文缺失/过短，自动抓取全文；抓取期间整个阅读区转圈等待。
  // 不用 cancelled cleanup（会被同文章重跑误杀，导致 loadingFull 卡 true）：
  // 用 run token + 当前文章 id 判断，旧批不碰新状态，loading 状态保证最终收敛。
  const runKeyRef = React.useRef<string>("");
  React.useEffect(() => {
    if (!selectedArticleId) {
      runKeyRef.current = "";
      setLoadingFull(false);
      setLoadingOriginal(false);
      setFetchFailed(false);
      return;
    }
    const a = articlesRef.current.find((x) => x.id === selectedArticleId);
    if (!a) return;
    const reqKey = `${selectedArticleId}:${fullRetryCount}`;
    if (runKeyRef.current !== reqKey) {
      runKeyRef.current = reqKey;
      setLoadingFull(false);
      setLoadingOriginal(false);
      setFetchFailed(false);
      setStatus("");
    }
    const feed = feeds.find((f) => f.id === a.feed_id);
    opLog.openArticle(
      a.id,
      a.title ?? "",
      a.content?.length ?? 0,
      a.content_fetched,
      a.summary?.length ?? 0,
      feed?.default_original ? "skip" : "pending",
    );
    // 该源设置"默认应用内阅读原文"（正文抓不全的源）→ 直接打开原文
    if (feed?.default_original) {
      setOpenOriginal(true);
      return;
    }
    // 已有完整正文（>500 字，如 freebuf 全文 / CSDN 自带 content:encoded）→ 信任直接显示；
    // 否则（摘要/过短）一律抓全文。content_fetched 不作为跳过依据：内容可能陈旧/不完整。
    const hasFull = !!a.content && a.content.trim().length > 500;
    if (hasFull || fetchedFullRef.current.has(a.id)) {
      opLog.openArticle(a.id, a.title ?? "", a.content?.length ?? 0, a.content_fetched, a.summary?.length ?? 0, "trust");
      return;
    }
    fetchedFullRef.current.add(a.id);
    setLoadingFull(true);
    opLog.readerState({ loadingFull: true });
    const run = ++fetchRunRef.current;
    const myId = a.id;
    (async () => {
      try {
        const ok = await api.fetchFullContent(myId);
        const updated = await api.getArticle(myId);
        if (updated) {
          setArticles((prev) => prev.map((x) => (x.id === updated.id ? updated : x)));
          opLog.fetchFull(updated.id, ok, updated.content?.length ?? 0);
        } else {
          opLog.fetchFull(myId, ok, 0, "no_updated_article");
        }
        if (!ok && selectedArticleIdRef.current === myId) {
          // 抓不到全文 → 会话内可重试 + 提示（不再自动跳原文，避免打扰）
          fetchedFullRef.current.delete(myId);
          setFetchFailed(true);
          opLog.readerState({ fetchFailed: true });
        } else if (ok) {
          fetchedFullRef.current.add(myId);
        }
      } catch (e) {
        if (selectedArticleIdRef.current === myId) {
          fetchedFullRef.current.delete(myId);
          setFetchFailed(true);
          opLog.fetchFull(myId, false, 0, String(e));
          opLog.readerState({ fetchFailed: true });
        }
      } finally {
        // 只有最新一次运行 + 仍是当前文章才关转圈；切走/重试后旧批不碰状态
        if (run === fetchRunRef.current && selectedArticleIdRef.current === myId) {
          setLoadingFull(false);
          opLog.readerState({ loadingFull: false });
        }
      }
    })();
  }, [selectedArticleId, feeds, fullRetryCount]);

  // 正文图片代理：扫描当前文章正文的图，全部加载完成后一次性提交 imgMap（iframe 只重建一次）。
  // 加载期间阅读区转圈。不用 cancelled（会被重跑误杀导致 imgLoading 卡 true），
  // 用 run token + inflight 集合：pending 为空≠加载完，必须等 inflight 清空 + 最新运行才关转圈。
  React.useEffect(() => {
    const curId = selectedArticleId;
    const rawBody =
      selectedArticle?.content ||
      selectedArticle?.summary ||
      "";
    const needed = new Set<string>();
    extractImgUrls(rawBody).forEach((u) => needed.add(u));
    const pending = [...needed].filter(
      (u) => !imgMap[u] && !fetchedImgsRef.current.has(u) && (imgRetriesRef.current[u] ?? 0) <= 2,
    );
    if (pending.length === 0) {
      // 没有新图要加载：仅当没有进行中的图时才关闭转圈（进行中由本批负责）
      if (imgInFlightRef.current.size === 0) setImgLoading(false);
      return;
    }
    const run = ++imgRunRef.current;
    pending.forEach((u) => {
      imgInFlightRef.current.add(u);
      fetchedImgsRef.current.add(u);
    });
    let done = 0;
    let failed = 0;
    let timeout = 0;
    const results: Record<string, string> = {};
    setImgLoading(true);
    setImgStatus(`0/${pending.length}`);
    opLog.imagesPending(pending.length);
    opLog.readerState({ imgLoading: true });
    (async () => {
      const worker = async (start: number) => {
        const retryFail = (u: string) => {
          const n = (imgRetriesRef.current[u] ?? 0) + 1;
          imgRetriesRef.current[u] = n;
          // 允许再试 2 轮（后端 fetch_image 已内置 3 次重试，此兜底缓解偶发图裂）
          if (n <= 2) fetchedImgsRef.current.delete(u);
        };
        for (let i = start; i < pending.length; i += 3) {
          const u = pending[i];
          try {
            // Referer 用文章源站 URL（CDN 防盗链多按引用页域校验），图片代理命中磁盘缓存二次秒出
            // 单图 8s 超时：网络慢/代理挂起时跳过该图，避免 0/x 卡死
            const img = await Promise.race([
              api.fetchImage(u, selectedArticle?.url ?? undefined),
              new Promise<never>((_, rej) => setTimeout(() => rej(new Error("img timeout")), 8000)),
            ]);
            const data = b64ToDataUrl(img);
            if (data) {
              results[u] = data;
            } else {
              retryFail(u);
              failed += 1;
            }
          } catch (e) {
            retryFail(u);
            if (String(e) === "img timeout" || String(e).includes("img timeout")) timeout += 1;
            else failed += 1;
          }
          imgInFlightRef.current.delete(u);
          done += 1;
          setImgStatus(`${done}/${pending.length}`);
        }
      };
      await Promise.all([worker(0), worker(1), worker(2)]);
      // 无条件合入全局图缓存（无害；切换文章后由清理 effect 删除非当前图）
      setImgMap((prev) => {
        const next = { ...prev };
        let changed = false;
        for (const [k, v] of Object.entries(results)) {
          if (!prev[k]) {
            next[k] = v;
            changed = true;
          }
        }
        return changed ? next : prev;
      });
      // 只有最新一次运行 + 没有进行中的图 + 仍是当前文章才关转圈
      if (run === imgRunRef.current && imgInFlightRef.current.size === 0 && selectedArticleIdRef.current === curId) {
        setImgLoading(false);
        setImgStatus("");
        opLog.imagesDone(done, pending.length, failed, timeout);
        opLog.readerState({ imgLoading: false });
      }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedArticleId, selectedArticle?.content, selectedArticle?.summary]);

  // 列表封面图代理：后台加载，不阻塞阅读区转圈（失败静默，可重试）
  React.useEffect(() => {
    const needed = new Set<string>();
    visibleArticles.forEach((a) => {
      const img = firstImage(a.content || a.summary);
      if (img) needed.add(img);
    });
    const pending = [...needed].filter(
      (u) => !imgMap[u] && !fetchedImgsRef.current.has(u) && (imgRetriesRef.current[u] ?? 0) <= 2,
    );
    if (pending.length === 0) return;
    pending.forEach((u) => {
      imgInFlightRef.current.add(u);
      fetchedImgsRef.current.add(u);
    });
    (async () => {
      const worker = async (start: number) => {
        for (let i = start; i < pending.length; i += 3) {
          const u = pending[i];
          try {
            const img = await Promise.race([
              api.fetchImage(u, undefined),
              new Promise<never>((_, rej) => setTimeout(() => rej(new Error("img timeout")), 8000)),
            ]);
            const data = b64ToDataUrl(img);
            if (data) {
              setImgMap((prev) => (prev[u] ? prev : { ...prev, [u]: data }));
            } else {
              const n = (imgRetriesRef.current[u] ?? 0) + 1;
              imgRetriesRef.current[u] = n;
              if (n <= 2) fetchedImgsRef.current.delete(u);
            }
          } catch {
            const n = (imgRetriesRef.current[u] ?? 0) + 1;
            imgRetriesRef.current[u] = n;
            if (n <= 2) fetchedImgsRef.current.delete(u);
          }
          imgInFlightRef.current.delete(u);
        }
      };
      await Promise.all([worker(0), worker(1), worker(2)]);
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [visibleArticles]);

  // 内存优化：切换文章时清理非当前文章的 data URL 图（保留当前正文图 + 可见列表封面；
  // 磁盘缓存兜底，二次查看秒出）
  React.useEffect(() => {
    if (selectedArticleId == null) return;
    const keep = new Set<string>();
    const cur = selectedArticle?.content || selectedArticle?.summary || "";
    extractImgUrls(cur).forEach((u) => keep.add(u));
    visibleArticles.forEach((a) => {
      const img = firstImage(a.content || a.summary);
      if (img) keep.add(img);
    });
    setImgMap((prev) => {
      const drop = Object.keys(prev).filter((u) => !keep.has(u));
      if (drop.length === 0) return prev;
      const next = { ...prev };
      drop.forEach((u) => delete next[u]);
      return next;
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedArticleId]);

  // 媒体识别：正文里的第三方播放器/音视频直链
  React.useEffect(() => {
    const rawBody =
      selectedArticle?.content ||
      selectedArticle?.summary ||
      "";
    setMediaUrls(extractMediaUrls(rawBody));
  }, [selectedArticle]);

  // 正文内嵌播放按钮 → postMessage → 弹窗播放
  React.useEffect(() => {
    const onMsg = (e: MessageEvent) => {
      const d = e.data;
      if (d && typeof d === "object" && d.type === "rss-play" && typeof d.idx === "number") {
        setPlayIdx(d.idx);
      }
    };
    window.addEventListener("message", onMsg);
    return () => window.removeEventListener("message", onMsg);
  }, []);

  const toggleStar = async (a: Article) => {
    try {
      await api.toggleStar(a.id);
    } catch (e) {
      setStatus("ERR: " + e);
      return;
    }
    setArticles((prev) => prev.map((x) => (x.id === a.id ? { ...x, is_starred: !a.is_starred } : x)));
    reloadData();
  };

  const doRefresh = async (fetchFull = false, silent = false) => {
    if (refreshing) return;
    setRefreshing(true);
    // 有选中文章 → 强制重新抓取当前文章全文；否则刷新全部订阅
    if (selectedArticleId != null) {
      setStatus(t("status.fetching"));
      try {
        setOpenOriginal(false);
        setFileView(null);
        const ok = await api.fetchFullContent(selectedArticleId);
        const updated = await api.getArticle(selectedArticleId);
        if (updated) {
          setArticles((prev) => prev.map((x) => (x.id === updated.id ? updated : x)));
        }
        setStatus(ok ? t("status.fetched") : t("status.noFull"));
        if (!ok) setOpenOriginal(true);
      } catch (e) {
        setStatus(t("status.fetchFailed") + e);
      } finally {
        setRefreshing(false);
      }
      return;
    }
    if (!silent) setStatus(t("status.refreshing"));
    try {
      const res = await api.refresh(fetchFull);
      setStatus(
        t("status.refreshed", { n: res.feeds_checked, m: res.articles_new }) +
          (res.errors.length ? t("status.errors", { n: res.errors.length }) : "")
      );
      await Promise.all([reloadData(), loadArticles(selection, search)]);
    } catch (e) {
      setStatus(t("status.refreshFailed") + e);
    } finally {
      setRefreshing(false);
    }
  };

  const removeFeed = async (id: number) => {
    try {
      await api.removeFeed(id);
    } catch (e) {
      setStatus("ERR: " + e);
      return;
    }
    if (selection.kind === "feed" && selection.id === id) setSelection({ kind: "all" });
    await Promise.all([reloadData(), loadArticles({ kind: "all" }, search)]);
  };

  const removeFolder = async (id: number) => {
    try {
      await api.removeFolder(id);
    } catch (e) {
      setStatus("ERR: " + e);
      return;
    }
    if (selection.kind === "folder" && selection.id === id) setSelection({ kind: "all" });
    await Promise.all([reloadData(), loadArticles({ kind: "all" }, search)]);
  };

  const addFeed = async () => {
    const url = addFeedUrl.trim();
    if (!url) return;
    setShowAddFeed(false);
    setAddFeedUrl("");
    setStatus(t("status.adding", { url }));
    try {
      const res = await api.addFeed(url, selection.kind === "folder" ? selection.id : null, false);
      setStatus(t("status.added", { title: res.feed.title, n: res.articles_new }));
      await Promise.all([reloadData(), loadArticles(selection, search)]);
    } catch (e) {
      setStatus(t("status.addFailed") + e);
    }
  };

  const addFolder = async () => {
    const name = addFolderName.trim();
    if (!name) return;
    setShowAddFolder(false);
    setAddFolderName("");
    try {
      await api.addFolder(name);
      setStatus(t("status.folderCreated", { name }));
      await reloadData();
    } catch (e) {
      setStatus(t("status.folderFailed") + e);
    }
  };

  const markAllRead = async () => {
    const keepId = selectedArticleId;
    try {
      if (selection.kind === "feed") await api.markFeedRead(selection.id);
      else if (selection.kind === "folder") await api.markFolderRead(selection.id);
      else await api.markAllRead();
      setStatus(t("status.markedRead"));
      await Promise.all([reloadData(), loadArticles(selection, search)]);
      // 保留当前文章（若仍在列表里）
      if (keepId != null) setSelectedArticleId(keepId);
    } catch (e) {
      setStatus(t("status.markReadFailed") + e);
    }
  };

  const offsetArticle = (offset: number) => {
    if (visibleArticles.length === 0) return;
    const cur = selectedArticleId
      ? visibleArticles.findIndex((a) => a.id === selectedArticleId)
      : -1;
    const next = cur < 0 ? 0 : Math.max(0, Math.min(cur + offset, visibleArticles.length - 1));
    const a = visibleArticles[next];
    if (a) {
      setSelectedArticleId(a.id);
      setOpenOriginal(false);
      setFileView(null);
    }
  };

  const markCurrentRead = async (read: boolean) => {
    if (!selectedArticle) return;
    await api.markRead([selectedArticle.id], read);
    setArticles((prev) => prev.map((x) => (x.id === selectedArticle.id ? { ...x, is_read: read } : x)));
    reloadData();
  };

  const fetchFullCurrent = async () => {
    if (!selectedArticle) return;
    setStatus(t("status.fetching"));
    try {
      const ok = await api.fetchFullContent(selectedArticle.id);
      const updated = await api.getArticle(selectedArticle.id);
      if (updated) setArticles((prev) => prev.map((x) => (x.id === updated.id ? updated : x)));
      setStatus(ok ? t("status.fetched") : t("status.noFull"));
    } catch (e) {
      setStatus(t("status.fetchFailed") + e);
    }
  };

  const copyMarkdown = async () => {
    if (!selectedArticle) return;
    const html = selectedArticle.content || selectedArticle.summary || "";
    try {
      await navigator.clipboard.writeText(htmlToMarkdown(html));
      setStatus(t("status.copied"));
    } catch (e) {
      setStatus("copy failed: " + e);
    }
  };

  React.useEffect(() => {
    api.getSetting("fontSize").then((v) => {
      const n = Number(v);
      if (n >= 12 && n <= 26) setFontSize(n);
    });
  }, []);

  const setFontSizePersist = (n: number) => {
    setFontSize(n);
    api.setSetting("fontSize", String(n));
  };

  const readerDocHtml = React.useMemo(() => {
    if (!selectedArticle) return "";
    const isDark = dark;
    const fg = isDark ? "#e6e6e6" : "#242424";
    const sub = isDark ? "#8f8f8f" : "#6a6a6a";
    const border = isDark ? "rgba(255,255,255,0.12)" : "rgba(0,0,0,0.1)";
    let bodyHtml =
      selectedArticle.content || stripFeedTemplate(selectedArticle.summary || "") || "";
    // 纯文本摘要（无 HTML 标签）→ 转义并保留换行，避免一坨 URL
    if (bodyHtml && !/<[a-zA-Z\/]/.test(bodyHtml)) {
      bodyHtml = `<p>${escapeHtml(bodyHtml).replace(/\n+/g, "<br>")}</p>`;
    }
    const bodyHtmlReady = replaceMediaIframes(replaceImages(bodyHtml, imgMap));
    const meta = [
      feedName(selectedArticle.feed_id),
      selectedArticle.author ? `${t("reader.by")} ${selectedArticle.author}` : null,
      selectedArticle.published_at
        ? new Date(selectedArticle.published_at).toLocaleString()
        : null,
    ]
      .filter(Boolean)
      .join(" · ");
    return `<!doctype html><html><head><meta charset="utf-8"><style>
      *{box-sizing:border-box}
      body{max-width:760px;margin:0 auto;padding:28px 30px 80px;font-size:${fontSize}px;line-height:1.85;font-family:system-ui,-apple-system,"Segoe UI",Roboto,"PingFang SC","Microsoft YaHei",sans-serif;color:${fg};background:transparent;word-break:break-word}
      .head{margin-bottom:26px;padding-bottom:18px;border-bottom:1px solid ${border}}
      .title{font-size:${fontSize + 7}px;font-weight:700;line-height:1.4;margin:0 0 10px}
      .meta{color:${sub};font-size:${fontSize - 2}px}
      img{max-width:100%;max-height:75vh;height:auto;border-radius:8px;display:block;margin:14px auto;object-fit:contain}
      a img{border-radius:8px}
      figure{margin:22px 0} figcaption{font-size:${fontSize - 3}px;color:${sub};text-align:center;margin-top:8px}
      p{margin:0 0 16px}
      h1,h2,h3,h4,h5,h6{line-height:1.4;margin:30px 0 14px;font-weight:700}
      h1{font-size:${fontSize + 4}px} h2{font-size:${fontSize + 2}px;border-bottom:1px solid ${border};padding-bottom:8px}
      h3{font-size:${fontSize + 1}px} h4,h5,h6{font-size:${fontSize}px}
      pre{background:rgba(127,127,127,.08);padding:16px 18px;border-radius:10px;overflow:auto;font-size:${fontSize - 1}px;line-height:1.6}
      code{font-family:ui-monospace,SFMono-Regular,Consolas,monospace;background:rgba(127,127,127,.12);padding:2px 5px;border-radius:4px;font-size:.92em}
      pre code{background:none;padding:0;font-size:inherit}
      blockquote{border-left:4px solid #8ab4f8;margin:22px 0;padding:10px 18px;color:${sub};background:rgba(127,127,127,.05);border-radius:0 8px 8px 0}
      blockquote p{margin:6px 0}
      a{color:#3a7bd5;text-decoration:none} a:hover{text-decoration:underline}
      table{border-collapse:collapse;width:100%;margin:18px 0} th,td{border:1px solid ${border};padding:9px 12px;text-align:left}
      th{background:rgba(127,127,127,.06)}
      hr{border:none;border-top:1px solid ${border};margin:26px 0}
      ul,ol{padding-left:24px;margin:0 0 16px}
      li{margin:5px 0}
      iframe,video{max-width:100%;border-radius:8px;display:block;margin:14px auto}
      audio{width:100%;max-width:520px;margin:14px 0}
      .mark{background:rgba(255,214,0,.2);border-radius:3px;padding:0 2px}
      sup,sub{font-size:.75em}
      .video-wrap{position:relative;width:100%;aspect-ratio:16/9;margin:14px auto} .video-wrap iframe{position:absolute;inset:0;width:100%;height:100%}
      /* 抓取残留清理：隐藏空占位/内联零尺寸元素，避免排版出现空洞 */
      div:empty,span:empty{display:none}
      div[style*="height:0"],div[style*="height: 0"],div[style*="width:0"],div[style*="width: 0"],img[style*="height:0"]{display:none !important}
      /* 站内推荐/推广残留 */
      .recommend-box,.recommend_list,.article-bottom,.share-tools,.related-news,.recommend{display:none !important}
    </style></head><body>
      <header class="head"><h1 class="title">${escapeHtml(
        selectedArticle.title || ""
      )}</h1>${meta ? `<div class="meta">${escapeHtml(meta)}</div>` : ""}</header>
      <article>${bodyHtmlReady}</article>
      <script>
        document.addEventListener('click', function(e){
          var b = e.target && e.target.closest ? e.target.closest('button.rss-play') : null;
          if (b) { parent.postMessage({type:'rss-play', idx: +b.getAttribute('data-idx')}, '*'); }
        });
      </script>
    </body></html>`;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedArticle, imgMap, fontSize, dark]);

  const toggleSidebar = () => {
    if (viewMode === "list") setSidebarCollapsed((v) => !v);
    else setDrawerOpen((v) => !v);
  };
  // "应用内打开原文"：探测类型 → HTML 内嵌真实网页（像浏览器）；文件（PDF 等）走文件视图。
  const toggleOriginal = async () => {
    if (!selectedArticle?.url) return;
    if (loadingOriginal) return;
    setLoadingOriginal(true);
    setOriginalBlocked(false);
    try {
      const res = await api.probeUrl(selectedArticle.url);
      if (res.kind === "file") {
        setFileView(res);
        setOpenOriginal(false);
      } else {
        // 站点禁止被 iframe 内嵌（X-Frame-Options/CSP）→ 直接提示浏览器打开
        if (res.allow_embed === false) {
          setOriginalBlocked(true);
          setOpenOriginal(true);
          setFileView(null);
        } else {
          // 抓一次原始 HTML 检测 CF 验证页，避免 iframe 白屏无提示
          try {
            const page = await api.fetchOriginalHtml(selectedArticle.url);
            const low = (page.content || "").toLowerCase();
            setOriginalBlocked(
              low.includes("just a moment") ||
                low.includes("challenges.cloudflare.com") ||
                low.includes("attention required"),
            );
          } catch {
            setOriginalBlocked(false);
          }
          setOpenOriginal(true);
          setFileView(null);
        }
      }
    } catch (e) {
      // 探测失败也允许直接内嵌真实网页
      setOpenOriginal(true);
      setFileView(null);
      setStatus("原文探测失败，已直接加载原网页: " + e);
    } finally {
      setLoadingOriginal(false);
    }
  };
  // 回到正文视图（清除原文/文件视图）
  const backToArticle = () => {
    setOpenOriginal(false);
    setFileView(null);
  };

  const wrapHtmlDoc = (bodyHtml: string) =>
    `<!doctype html><html><head><meta charset="utf-8"><style>
      body{max-width:760px;margin:40px auto;line-height:1.75;font-family:system-ui,-apple-system,sans-serif;color:#222;padding:0 20px}
      img{max-width:100%}pre{background:#f4f4f4;padding:14px;overflow:auto;border-radius:6px}
      a{color:#0b6bcb}blockquote{border-left:3px solid #ccc;margin:0;padding-left:16px;color:#555}
    </style></head><body>${bodyHtml}</body></html>`;

  const exportArticle = async (kind: "md" | "html" | "txt") => {
    if (!selectedArticle) return;
    const html = selectedArticle.content || selectedArticle.summary || "";
    const feedN = feedName(selectedArticle.feed_id);
    let content = "";
    let ext = "txt";
    if (kind === "md") {
      ext = "md";
      content = `# ${selectedArticle.title || ""}\n\n> ${feedN} · ${
        selectedArticle.published_at ? new Date(selectedArticle.published_at).toLocaleString() : ""
      }\n\n${htmlToMarkdown(html)}`;
    } else if (kind === "html") {
      ext = "html";
      const head = `<h1>${escapeHtml(selectedArticle.title || "")}</h1><p style="color:#888">${escapeHtml(
        feedN
      )} · ${escapeHtml(selectedArticle.author || "")}</p>`;
      content = wrapHtmlDoc(head + html);
    } else {
      ext = "txt";
      const doc = new DOMParser().parseFromString(html, "text/html");
      content = `${selectedArticle.title || ""}\n${feedN}\n\n${(doc.body.innerText || "").trim()}`;
    }
    const path = await save({
      defaultPath: `article-${selectedArticle.id}.${ext}`,
      filters: [{ name: ext.toUpperCase(), extensions: [ext] }],
    });
    if (!path) return;
    try {
      await api.writeTextFile(path as string, content);
      setStatus(t("status.exported", { path: String(path) }));
    } catch (e) {
      setStatus(t("status.exportFailed") + e);
    }
  };

  const printArticle = () => {
    if (!selectedArticle) return;
    const html = selectedArticle.content || selectedArticle.summary || "";
    const docHtml = wrapHtmlDoc(
      `<h1>${escapeHtml(selectedArticle.title || "")}</h1><p style="color:#888">${escapeHtml(
        feedName(selectedArticle.feed_id)
      )}</p>${html}`
    );
    const iframe = document.createElement("iframe");
    iframe.style.position = "fixed";
    iframe.style.right = "0";
    iframe.style.bottom = "0";
    iframe.style.width = "0";
    iframe.style.height = "0";
    iframe.style.border = "0";
    document.body.appendChild(iframe);
    const doc = iframe.contentDocument;
    if (doc) {
      doc.open();
      doc.write(docHtml);
      doc.close();
      setTimeout(() => {
        iframe.contentWindow?.print();
        setTimeout(() => iframe.remove(), 1000);
      }, 400);
    }
  };

  // 键盘快捷键
  React.useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      const tag = target?.tagName?.toLowerCase();
      if (tag === "input" || tag === "textarea" || (target?.isContentEditable ?? false)) return;
      if (e.metaKey || e.ctrlKey) return;
      if (showAddFeed || showAddFolder || showSettings) return;

      const key = e.key.toLowerCase();
      switch (key) {
        case "?":
        case "f1":
          e.preventDefault();
          setShowHelp((v) => !v);
          return;
        case "1":
          e.preventDefault();
          setSelection({ kind: "all" });
          return;
        case "2":
          e.preventDefault();
          setSelection({ kind: "unread" });
          return;
        case "3":
          e.preventDefault();
          setSelection({ kind: "starred" });
          return;
        case "r":
          e.preventDefault();
          doRefresh(false);
          return;
        case "/":
          e.preventDefault();
          (document.querySelector('input[placeholder*="Search"], input[placeholder*="搜索"]') as HTMLInputElement | null)?.focus();
          return;
      }
      if (visibleArticles.length === 0) return;
      const cur = selectedArticleId ? visibleArticles.findIndex((a) => a.id === selectedArticleId) : -1;
      if (key === "j" || key === "arrowdown") {
        e.preventDefault();
        const next = cur < 0 ? 0 : Math.min(cur + 1, visibleArticles.length - 1);
        setSelectedArticleId(visibleArticles[next].id);
        scrollListTo(next);
      } else if (key === "k" || key === "arrowup") {
        e.preventDefault();
        const next = cur <= 0 ? 0 : cur - 1;
        setSelectedArticleId(visibleArticles[next].id);
        scrollListTo(next);
      } else if (key === "g") {
        e.preventDefault();
        if (visibleArticles[0]) setSelectedArticleId(visibleArticles[0].id);
        scrollListTo(0);
      } else if (key === "enter" && cur >= 0) {
        e.preventDefault();
        const a = visibleArticles[cur];
        if (!a.is_read) {
          api.markRead([a.id], true).then(() => {
            setArticles((prev) => prev.map((x) => (x.id === a.id ? { ...x, is_read: true } : x)));
            reloadData();
          });
        }
      } else if (key === "s" && cur >= 0) {
        e.preventDefault();
        toggleStar(visibleArticles[cur]);
      } else if (key === " " && cur >= 0) {
        e.preventDefault();
        markCurrentRead(!visibleArticles[cur].is_read);
      } else if (key === "o" && cur >= 0 && visibleArticles[cur].url) {
        e.preventDefault();
        openUrl(visibleArticles[cur].url!);
      } else if (key === "f" && cur >= 0) {
        e.preventDefault();
        fetchFullCurrent();
      } else if (key === "m") {
        e.preventDefault();
        markAllRead();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [visibleArticles, selectedArticleId, showAddFeed, showAddFolder, showSettings]);

  const scrollListTo = (idx: number) => {
    requestAnimationFrame(() => {
      const el = listRef.current?.querySelector(`[data-idx="${idx}"]`);
      el?.scrollIntoView({ block: "nearest" });
    });
  };

  function feedName(feedId: number): string {
    return feeds.find((f) => f.id === feedId)?.title ?? "";
  }

  // ---------- 渲染辅助 ----------

  const renderFavicon = (feed: Feed, size = 16) => (
    feed.favicon_url ? (
      <img src={feed.favicon_url} alt="" width={size} height={size} className={styles.favicon} />
    ) : (
      <RssRegular style={{ width: size - 2, height: size - 2, flexShrink: 0 }} />
    )
  );

  const feedMenu = (items: React.ReactNode) => (
    <Menu>
      <MenuTrigger disableButtonEnhancement>
        <Button
          size="small"
          appearance="subtle"
          icon={<MoreHorizontalRegular />}
          className={styles.actionBtn}
          aria-label="actions"
        />
      </MenuTrigger>
      <MenuPopover>
        <MenuList>{items}</MenuList>
      </MenuPopover>
    </Menu>
  );

  const renderNavRow = (
    key: string,
    sel: Selection,
    label: string,
    count: number | null,
    icon: React.ReactElement | null,
    actions?: React.ReactNode,
    indented = false
  ) => {
    const active = JSON.stringify(selection) === JSON.stringify(sel);
    return (
      <div
        key={key}
        className={`${styles.navRow} ${active ? styles.navRowSelected : ""} ${indented ? styles.feedIndent : ""}`}
        onClick={() => {
          setSelection(sel);
          setSelectedArticleId(null);
        }}
      >
        {icon}
        <Text size={300} className={styles.navLabel} truncate>
          {label}
        </Text>
        {count != null && count > 0 && (
          <Badge appearance="filled" color="brand" size="small">
            {count}
          </Badge>
        )}
        {actions && <div className={styles.rowActionsVisible}>{actions}</div>}
      </div>
    );
  };

  const retryImgOnError = React.useCallback((u: string | null) => {
    if (!u) return;
    setImgMap((prev) => {
      if (!prev[u]) return prev;
      const next = { ...prev };
      delete next[u];
      return next;
    });
    fetchedImgsRef.current.delete(u);
  }, []);


  const renderListContent = () => {
    if (visibleArticles.length === 0) {
      return <div className={styles.empty}>{t("list.noArticles")}</div>;
    }
    const renderCards = () =>
      visibleArticles.map((a, i) => (
        <ArticleCards
          key={a.id}
          article={a}
          idx={i}
          viewMode={viewMode}
          cardSize={cardSize}
          selected={selectedArticleId === a.id}
          imgMap={imgMap}
          feedName={feedName}
          onSelect={selectArticle}
          onImgError={retryImgOnError}
        />
      ));
    if (viewMode === "card") {
      const minW = cardSize === "small" ? 120 : cardSize === "large" ? 200 : 160;
      return (
        <div className={styles.cardGrid} style={{ gridTemplateColumns: `repeat(auto-fill, minmax(${minW}px, 1fr))` }}>
          {renderCards()}
        </div>
      );
    }
    if (viewMode === "magazine") {
      return (
        <div className={styles.magazineMixGrid}>{renderCards()}</div>
      );
    }
    return <>{renderCards()}</>;
  };

  const viewButtons: { mode: ViewMode; icon: React.ReactElement; title: string }[] = [
    { mode: "list", icon: <TextColumnOneRegular />, title: t("view.list") },
    { mode: "compact", icon: <TextBulletListRegular />, title: t("view.compact") },
    { mode: "card", icon: <GridRegular />, title: t("view.card") },
    { mode: "magazine", icon: <BookRegular />, title: t("view.magazine") },
  ];

  // 阅读区操作栏（列表视图右侧 & 浮层共用）
  const renderReaderActions = () =>
    selectedArticle ? (
      <>
        <Button
          icon={<StarFilled />}
          appearance={selectedArticle.is_starred ? "primary" : "subtle"}
          onClick={() => toggleStar(selectedArticle)}
        >
          {selectedArticle.is_starred ? t("reader.starred") : t("reader.star")}
        </Button>
        {selectedArticle.url && (
          <Button
            icon={<OpenRegular />}
            appearance="subtle"
            onClick={() => openUrl(selectedArticle.url!)}
          >
            {t("reader.openBrowser")}
          </Button>
        )}
        {selectedArticle.url && (
          <Button
            icon={<DesktopRegular />}
            appearance={openOriginal || fileView ? "primary" : "subtle"}
            onClick={openOriginal || fileView ? backToArticle : toggleOriginal}
            disabled={loadingOriginal}
          >
            {loadingOriginal
              ? "…"
              : openOriginal || fileView
              ? t("reader.backArticle")
              : t("reader.openOriginal")}
          </Button>
        )}
        {loadingFull && <Spinner size="tiny" />}
        <Menu checkedValues={{ fontSize: [String(fontSize)] }}>
          <MenuTrigger disableButtonEnhancement>
            <Button appearance="subtle" icon={<MoreHorizontalRegular />} aria-label="more" />
          </MenuTrigger>
          <MenuPopover>
            <MenuList>
              <Menu>
                <MenuTrigger disableButtonEnhancement>
                  <MenuItem icon={<DocumentSaveRegular />}>{t("reader.export")}</MenuItem>
                </MenuTrigger>
                <MenuPopover>
                  <MenuList>
                    <MenuItem icon={<DocumentMarkdownRegular />} onClick={() => exportArticle("md")}>
                      Markdown
                    </MenuItem>
                    <MenuItem icon={<CodeRegular />} onClick={() => exportArticle("html")}>
                      HTML
                    </MenuItem>
                    <MenuItem icon={<DocumentPdfRegular />} onClick={printArticle}>
                      PDF
                    </MenuItem>
                    <MenuItem icon={<DocumentTextRegular />} onClick={() => exportArticle("txt")}>
                      TXT
                    </MenuItem>
                  </MenuList>
                </MenuPopover>
              </Menu>
              <MenuItem icon={<CopyRegular />} onClick={copyMarkdown}>
                {t("reader.copyMarkdown")}
              </MenuItem>
              <Divider />
              <MenuItem onClick={() => setFontSizePersist(Math.max(8, fontSize - 1))}>
                A−
              </MenuItem>
              <MenuItem onClick={() => setFontSizePersist(fontSize + 1)}>
                A+
              </MenuItem>
            </MenuList>
          </MenuPopover>
        </Menu>
      </>
    ) : null;

  // 侧边栏/抽屉顶部的搜索 + 筛选
  const renderSidebarSearch = () => (
    <div style={{ padding: "0 0 10px", borderBottom: `1px solid ${tokens.colorNeutralStroke1}`, marginBottom: "6px" }}>
      <SearchBox
        placeholder={t("list.searchPlaceholder")}
        value={search}
        onChange={(_e, d) => setSearch(d.value)}
        contentBefore={<SearchRegular />}
        size="small"
        style={{ width: "100%" }}
      />
    </div>
  );

  const renderReaderFrame = () => {
    if (!selectedArticle) return null;
    // 文件视图（PDF 等）
    if (fileView) {
      return (
        <div style={{ flex: 1, display: "flex", flexDirection: "column", minHeight: 0 }}>
          <div style={{ padding: "8px 12px", display: "flex", gap: "8px", alignItems: "center" }}>
            <Text size={300} style={{ color: tokens.colorNeutralForeground3 }}>
              {fileView.content_type || "file"}
            </Text>
            {selectedArticle.url && (
              <Button
                appearance="primary"
                icon={<OpenRegular />}
                onClick={() => openUrl(selectedArticle.url!)}
              >
                {t("reader.openSystem")}
              </Button>
            )}
          </div>
          <iframe
            sandbox=""
            src={selectedArticle.url!}
            title="file"
            style={{ flex: 1, width: "100%", border: "none", minHeight: 0, background: "#fff" }}
          />
        </div>
      );
    }
    // 打开原文：内嵌真实网页（像浏览器，允许脚本/表单/弹窗，禁止导航主窗口）
    if (openOriginal && selectedArticle.url) {
      return (
        <div style={{ flex: 1, display: "flex", flexDirection: "column", minHeight: 0 }}>
          <div style={{ padding: "6px 12px", display: "flex", gap: "8px", alignItems: "center", borderBottom: `1px solid ${tokens.colorNeutralStroke1}` }}>
            <Text size={300} style={{ color: tokens.colorNeutralForeground3 }} truncate>
              {selectedArticle.url}
            </Text>
            <div style={{ flex: 1 }} />
            <Button appearance="subtle" size="small" icon={<OpenRegular />} onClick={() => openUrl(selectedArticle.url!)}>
              {t("reader.openBrowser")}
            </Button>
            <Button appearance="subtle" size="small" icon={<DesktopRegular />} onClick={() => api.openMediaWindow(selectedArticle.url!)}>
              {t("reader.openWindow")}
            </Button>
          </div>
          <div style={{ flex: 1, minHeight: 0, position: "relative" }}>
            <iframe
              sandbox="allow-scripts allow-same-origin allow-popups allow-forms allow-modals"
              src={selectedArticle.url}
              title="original"
              style={{ width: "100%", height: "100%", border: "none", minHeight: 0, background: "#fff" }}
            />
            {originalBlocked && (
              <div
                style={{
                  position: "absolute",
                  inset: 0,
                  display: "flex",
                  flexDirection: "column",
                  alignItems: "center",
                  justifyContent: "center",
                  gap: 10,
                  background: "rgba(255,255,255,0.94)",
                  color: tokens.colorNeutralForeground2,
                }}
              >
                <Text size={400} weight="semibold">
                  {t("reader.originalBlocked")}
                </Text>
                <Text size={300} style={{ color: tokens.colorNeutralForeground3 }}>
                  {t("reader.originalBlockedDesc")}
                </Text>
                <Button appearance="primary" icon={<OpenRegular />} onClick={() => openUrl(selectedArticle.url!)}>
                  {t("reader.openBrowser")}
                </Button>
              </div>
            )}
          </div>
        </div>
      );
    }
    // 刷新中且正看着某篇文章（正在等待内容）→ 阅读区转圈；无选中文章刷新不转圈
    if (refreshing) {
      return (
        <div className={styles.empty} style={{ flex: 1, flexDirection: "column", gap: 8 }}>
          <Spinner />
          <Text size={300} style={{ color: tokens.colorNeutralForeground3 }}>
            {t("status.refreshing")}
          </Text>
        </div>
      );
    }
    // 加载中：正在抓全文 或 正在加载图片 → 整区转圈等待，不显示半成品。
    // 抓全文优先显示"正在加载全文"，全文就绪后才轮到图片进度。
    if (loadingFull || imgLoading) {
      const label = loadingFull
        ? t("status.fetchingFull")
        : imgLoading
        ? `${t("status.images")}${imgStatus ? ` ${imgStatus}` : ""}`
        : "";
      return (
        <div className={styles.empty} style={{ flex: 1, flexDirection: "column", gap: 8 }}>
          <Spinner />
          <Text size={300} style={{ color: tokens.colorNeutralForeground3 }}>
            {label}
          </Text>
        </div>
      );
    }
    // 正文/摘要都为空（抓取失败）→ 友好空态，引导打开原文
    if (!selectedArticle.content && !selectedArticle.summary) {
      return (
        <div className={styles.empty} style={{ flex: 1, flexDirection: "column", gap: 12 }}>
          <Text size={300} style={{ color: tokens.colorNeutralForeground3 }}>
            {t("reader.noContent")}
          </Text>
          {selectedArticle.url && (
            <Button appearance="primary" icon={<DesktopRegular />} onClick={toggleOriginal}>
              {t("reader.openOriginal")}
            </Button>
          )}
        </div>
      );
    }
    // 正文有内容 → 阅读 iframe + 顶部状态条（正在加载全文 / 抓取失败可重试）
    const showStatusBar = !selectedArticle.content_fetched && (loadingFull || fetchFailed);
    return (
      <div style={{ flex: 1, display: "flex", flexDirection: "column", minHeight: 0 }}>
        {showStatusBar && (
          <div
            style={{
              padding: "5px 12px",
              display: "flex",
              gap: "10px",
              alignItems: "center",
              borderBottom: `1px solid ${tokens.colorNeutralStroke1}`,
            }}
          >
            {loadingFull ? (
              <>
                <Spinner size="tiny" />
                <Text size={300} style={{ color: tokens.colorNeutralForeground3 }}>
                  {t("status.fetchingFull")}
                </Text>
              </>
            ) : (
              <>
                <Text size={300} style={{ color: tokens.colorStatusDangerForeground1 }}>
                  {t("status.fetchFailedFull")}
                </Text>
                <Button size="small" appearance="secondary" onClick={() => setFullRetryCount((c) => c + 1)}>
                  {t("reader.retryFull")}
                </Button>
                {selectedArticle.url && (
                  <Button size="small" appearance="secondary" onClick={toggleOriginal}>
                    {t("reader.openOriginal")}
                  </Button>
                )}
              </>
            )}
          </div>
        )}
        <iframe
          sandbox="allow-scripts"
          srcDoc={readerDocHtml}
          title="reader"
          style={{ width: "100%", flex: 1, border: "none", background: "transparent", minHeight: 0 }}
        />
      </div>
    );
  };

  return (
    <div className={styles.root}>
      {/* ---------- 顶栏 ---------- */}
      {/* 自绘标题栏：JS 拖拽（data-tauri-drag-region 对按钮区无效，改用 startDragging 排除交互元素） */}
      <div className={styles.titleBar} onMouseDown={onTitleBarMouseDown}>
        <Tooltip content={t("sidebar.all")} relationship="label">
          <Button icon={<PanelLeftRegular />} appearance="subtle" onClick={toggleSidebar} />
        </Tooltip>
        <Text weight="semibold" size={400} className={styles.appTitle}>
          Rust RSS Reader
        </Text>
        <div className={styles.spacer} />
        <Button
          appearance="subtle"
          icon={<AddRegular />}
          onClick={() => setShowAddFeed(true)}
          title={t("toolbar.addFeed")}
        />
        <Button
          appearance="subtle"
          icon={<ArrowSyncRegular />}
          onClick={() => doRefresh(false)}
          disabled={refreshing}
          title={t("toolbar.refresh")}
        />
        <Button
          appearance="subtle"
          icon={<CheckmarkCircleRegular />}
          onClick={markAllRead}
          title={t("toolbar.markAllRead")}
        />
        <Menu checkedValues={{ view: [viewMode] }}>
          <MenuTrigger disableButtonEnhancement>
            <Button appearance="subtle" icon={<EyeRegular />} title={t("view.switch")} />
          </MenuTrigger>
          <MenuPopover>
            <MenuList>
              {viewButtons.map((vb) => (
                <MenuItemCheckbox
                  key={vb.mode}
                  name="view"
                  value={vb.mode}
                  icon={vb.icon}
                  onClick={() => setViewMode(vb.mode)}
                >
                  {vb.title}
                </MenuItemCheckbox>
              ))}
            </MenuList>
          </MenuPopover>
        </Menu>
        <Button appearance="subtle" icon={<SettingsRegular />} onClick={() => setShowSettings(true)} title={t("toolbar.settings")} />
        <Button
          appearance="subtle"
          icon={dark ? <WeatherSunnyRegular /> : <WeatherMoonRegular />}
          onClick={() => setThemeMode(dark ? "light" : "dark")}
        />
        {!decorations && (
          <>
            <Button
              appearance="subtle"
              icon={<SubtractRegular />}
              title={t("nav.minimize")}
              onClick={() => getCurrentWindow().minimize()}
            />
            <Button
              appearance="subtle"
              icon={isMaximized ? <SquareRegular /> : <MaximizeRegular />}
              title={t("nav.maximize")}
              onClick={() => {
                const w = getCurrentWindow();
                w.toggleMaximize();
              }}
            />
            <Button
              appearance="subtle"
              icon={<DismissRegular />}
              title={t("dialog.close")}
              onClick={() => getCurrentWindow().close()}
            />
          </>
        )}
      </div>

      <div className={styles.body}>
        {viewMode === "list" ? (
        <>
        {/* ---------- 侧边栏（扁平列表，仅列表视图常驻） ---------- */}
        <div className={`${styles.sidebar} ${sidebarCollapsed ? styles.sidebarCollapsed : ""}`}>
          {!sidebarCollapsed && (
            <>
              {renderSidebarSearch()}
              {renderNavRow("all", { kind: "all" }, t("sidebar.all"), unread.total, <HomeRegular />)}
              {renderNavRow("unread", { kind: "unread" }, t("sidebar.unread"), unread.total, <MailUnreadRegular />)}
              {renderNavRow("starred", { kind: "starred" }, t("sidebar.starred"), null, <StarRegular />)}

              {folders.length > 0 && (
                <div className={styles.sectionHeader}>
                  <Text weight="semibold" size={300} style={{ color: tokens.colorNeutralForeground3 }}>
                    {t("sidebar.folders")}
                  </Text>
                </div>
              )}
              {folders.map((folder) => {
                const folderFeeds = feeds.filter((f) => f.folder_id === folder.id);
                const folderUnread = folderFeeds.reduce(
                  (sum, f) => sum + (unreadByFeed.get(f.id) ?? 0),
                  0
                );
                return (
                  <React.Fragment key={folder.id}>
                    {renderNavRow(
                      `folder-${folder.id}`,
                      { kind: "folder", id: folder.id },
                      folder.name,
                      folderUnread,
                      <FolderAddRegular style={{ opacity: 0.6 }} />,
                      feedMenu([
                        <MenuItem
                          key="del"
                          icon={<DeleteRegular />}
                          onClick={() => removeFolder(folder.id)}
                        >
                          {t("menu.deleteFolder")}
                        </MenuItem>,
                      ])
                    )}
                    {folderFeeds.map((feed) =>
                      renderNavRow(
                        `feed-${feed.id}`,
                        { kind: "feed", id: feed.id },
                        feed.title,
                        unreadByFeed.get(feed.id) ?? 0,
                        renderFavicon(feed),
                        feedMenu([
                          <MenuItem
                            key="refresh"
                            icon={<ArrowSyncRegular />}
                            onClick={() => api.refreshFeed(feed.id)}
                          >
                            {t("menu.refreshFeed")}
                          </MenuItem>,
                          <MenuItem
                            key="del"
                            icon={<DeleteRegular />}
                            onClick={() => removeFeed(feed.id)}
                          >
                            {t("menu.deleteFeed")}
                          </MenuItem>,
                        ]),
                        true
                      )
                    )}
                  </React.Fragment>
                );
              })}

              {feeds.filter((f) => f.folder_id === null).length > 0 && (
                <div className={styles.sectionHeader}>
                  <Text weight="semibold" size={300} style={{ color: tokens.colorNeutralForeground3 }}>
                    {t("sidebar.feeds")}
                  </Text>
                </div>
              )}
              {feeds
                .filter((f) => f.folder_id === null)
                .map((feed) =>
                  renderNavRow(
                    `feed-${feed.id}`,
                    { kind: "feed", id: feed.id },
                    feed.title,
                    unreadByFeed.get(feed.id) ?? 0,
                    renderFavicon(feed),
                    feedMenu([
                      <MenuItem
                        key="refresh"
                        icon={<ArrowSyncRegular />}
                        onClick={() => api.refreshFeed(feed.id)}
                      >
                        {t("menu.refreshFeed")}
                      </MenuItem>,
                      <MenuItem
                        key="del"
                        icon={<DeleteRegular />}
                        onClick={() => removeFeed(feed.id)}
                      >
                        {t("menu.deleteFeed")}
                      </MenuItem>,
                    ]),
                    false
                  )
                )}
            </>
          )}
        </div>

        {/* ---------- 文章列表 ---------- */}
        <div className={styles.middle}>
          <div className={styles.list} ref={listRef} onScroll={onListScroll}>
            {renderListContent()}
            {loadingMore && (
              <div style={{ padding: "12px", textAlign: "center" }}>
                <Spinner size="small" />
              </div>
            )}
            {!hasMore && visibleArticles.length > 0 && (
              <div style={{ padding: "10px", textAlign: "center", color: tokens.colorNeutralForeground3 }}>
                <Text size={200}>{t("list.end")}</Text>
              </div>
            )}
          </div>
        </div>

        {/* ---------- 阅读区（列表视图常驻） ---------- */}
        <div className={styles.reader}>
          {selectedArticle ? (
            <>
              <div className={styles.readerButtons}>{renderReaderActions()}</div>
              {renderReaderFrame()}
            </>
          ) : (
            <div className={styles.empty}>
              <div style={{ textAlign: "center", color: tokens.colorNeutralForeground3 }}>
                <RssRegular style={{ width: 48, height: 48, opacity: 0.4 }} />
                <div style={{ marginTop: 8 }}>{t("list.readerPlaceholder")}</div>
              </div>
            </div>
          )}
        </div>
        </>
        ) : (
        <>
          {/* 非列表视图：☰ 弹出侧边栏抽屉 */}
          {drawerOpen && (
            <>
              <div className={styles.drawerScrim} onClick={() => setDrawerOpen(false)} />
              <div className={styles.drawer}>
                {renderSidebarSearch()}
                {renderNavRow("all", { kind: "all" }, t("sidebar.all"), unread.total, <HomeRegular />)}
                {renderNavRow("unread", { kind: "unread" }, t("sidebar.unread"), unread.total, <MailUnreadRegular />)}
                {renderNavRow("starred", { kind: "starred" }, t("sidebar.starred"), null, <StarRegular />)}
                {folders.length > 0 && (
                  <div className={styles.sectionHeader}>
                    <Text weight="semibold" size={300} style={{ color: tokens.colorNeutralForeground3 }}>
                      {t("sidebar.folders")}
                    </Text>
                  </div>
                )}
                {folders.map((folder) => {
                  const folderFeeds = feeds.filter((f) => f.folder_id === folder.id);
                  const folderUnread = folderFeeds.reduce(
                    (sum, f) => sum + (unreadByFeed.get(f.id) ?? 0),
                    0
                  );
                  return (
                    <React.Fragment key={folder.id}>
                      {renderNavRow(
                        `folder-${folder.id}`,
                        { kind: "folder", id: folder.id },
                        folder.name,
                        folderUnread,
                        <FolderAddRegular style={{ opacity: 0.6 }} />,
                        feedMenu([
                          <MenuItem key="del" icon={<DeleteRegular />} onClick={() => removeFolder(folder.id)}>
                            {t("menu.deleteFolder")}
                          </MenuItem>,
                        ])
                      )}
                      {folderFeeds.map((feed) =>
                        renderNavRow(
                          `feed-${feed.id}`,
                          { kind: "feed", id: feed.id },
                          feed.title,
                          unreadByFeed.get(feed.id) ?? 0,
                          renderFavicon(feed),
                          feedMenu([
                            <MenuItem key="refresh" icon={<ArrowSyncRegular />} onClick={() => api.refreshFeed(feed.id)}>
                              {t("menu.refreshFeed")}
                            </MenuItem>,
                            <MenuItem key="del" icon={<DeleteRegular />} onClick={() => removeFeed(feed.id)}>
                              {t("menu.deleteFeed")}
                            </MenuItem>,
                          ]),
                          true
                        )
                      )}
                    </React.Fragment>
                  );
                })}
                {feeds.filter((f) => f.folder_id === null).length > 0 && (
                  <div className={styles.sectionHeader}>
                    <Text weight="semibold" size={300} style={{ color: tokens.colorNeutralForeground3 }}>
                      {t("sidebar.feeds")}
                    </Text>
                  </div>
                )}
                {feeds
                  .filter((f) => f.folder_id === null)
                  .map((feed) =>
                    renderNavRow(
                      `feed-${feed.id}`,
                      { kind: "feed", id: feed.id },
                      feed.title,
                      unreadByFeed.get(feed.id) ?? 0,
                      renderFavicon(feed),
                      feedMenu([
                        <MenuItem key="refresh" icon={<ArrowSyncRegular />} onClick={() => api.refreshFeed(feed.id)}>
                          {t("menu.refreshFeed")}
                        </MenuItem>,
                        <MenuItem key="del" icon={<DeleteRegular />} onClick={() => removeFeed(feed.id)}>
                          {t("menu.deleteFeed")}
                        </MenuItem>,
                      ]),
                      false
                    )
                  )}
              </div>
            </>
          )}

          {/* 整页内容布局 */}
          <div className={styles.fullContent}>
            <div className={styles.list} ref={listRef} onScroll={onListScroll}>
              {renderListContent()}
              {loadingMore && (
                <div style={{ padding: "12px", textAlign: "center" }}>
                  <Spinner size="small" />
                </div>
              )}
              {!hasMore && visibleArticles.length > 0 && (
                <div style={{ padding: "10px", textAlign: "center", color: tokens.colorNeutralForeground3 }}>
                  <Text size={200}>{t("list.end")}</Text>
                </div>
              )}
            </div>
          </div>

          {/* 点开文章 → 全屏阅读浮层 */}
          {selectedArticle && (
            <div className={styles.overlay}>
              <div className={styles.overlayScrim} onClick={() => setSelectedArticleId(null)} />
              <div className={styles.overlayPanel}>
                <div className={styles.overlayTop}>
                  <Text size={300} className={styles.overlaySource} truncate>
                    {feedName(selectedArticle.feed_id)}
                  </Text>
                  {renderReaderActions()}
                  <Button
                    appearance="subtle"
                    icon={<DismissRegular />}
                    title={t("dialog.close")}
                    onClick={() => setSelectedArticleId(null)}
                  />
                </div>
                {renderReaderFrame()}
              </div>
              <Button
                appearance="subtle"
                className={`${styles.overlayNav} ${styles.overlayNavPrev}`}
                icon={<ChevronLeftRegular />}
                onClick={() => offsetArticle(-1)}
                title="◀"
              />
              <Button
                appearance="subtle"
                className={`${styles.overlayNav} ${styles.overlayNavNext}`}
                icon={<ChevronRightRegular />}
                onClick={() => offsetArticle(1)}
                title="▶"
              />
            </div>
          )}
        </>
        )}
      </div>

      {/* ---------- 对话框 ---------- */}
      <Dialog open={showAddFeed} onOpenChange={(_e, d) => setShowAddFeed(d.open)}>
        <DialogSurface>
          <DialogBody>
            <DialogTitle>{t("dialog.addFeedTitle")}</DialogTitle>
            <DialogContent>
              <Input
                placeholder={t("dialog.addFeedPlaceholder")}
                value={addFeedUrl}
                onChange={(_e, d) => setAddFeedUrl(d.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") addFeed();
                }}
                style={{ width: "100%" }}
              />
            </DialogContent>
            <DialogActions>
              <DialogTrigger disableButtonEnhancement>
                <Button appearance="secondary">{t("dialog.cancel")}</Button>
              </DialogTrigger>
              <Button appearance="primary" onClick={addFeed}>
                {t("dialog.addFeedConfirm")}
              </Button>
            </DialogActions>
          </DialogBody>
        </DialogSurface>
      </Dialog>

      <Dialog open={showAddFolder} onOpenChange={(_e, d) => setShowAddFolder(d.open)}>
        <DialogSurface>
          <DialogBody>
            <DialogTitle>{t("dialog.newFolderTitle")}</DialogTitle>
            <DialogContent>
              <Input
                placeholder={t("dialog.folderPlaceholder")}
                value={addFolderName}
                onChange={(_e, d) => setAddFolderName(d.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") addFolder();
                }}
                style={{ width: "100%" }}
              />
            </DialogContent>
            <DialogActions>
              <DialogTrigger disableButtonEnhancement>
                <Button appearance="secondary">{t("dialog.cancel")}</Button>
              </DialogTrigger>
              <Button appearance="primary" onClick={addFolder}>
                {t("dialog.create")}
              </Button>
            </DialogActions>
          </DialogBody>
        </DialogSurface>
      </Dialog>

      <Dialog open={showHelp} onOpenChange={(_e, d) => setShowHelp(d.open)}>
        <DialogSurface>
          <DialogBody>
            <DialogTitle>{t("dialog.helpTitle")}</DialogTitle>
            <DialogContent>
              <div style={{ display: "grid", gridTemplateColumns: "auto 1fr", gap: "6px 16px" }}>
                <Text weight="semibold">1 / 2 / 3</Text>
                <Text>{t("dialog.help1")}</Text>
                <Text weight="semibold">j / k</Text>
                <Text>{t("dialog.help2")}</Text>
                <Text weight="semibold">g</Text>
                <Text>{t("dialog.help3")}</Text>
                <Text weight="semibold">Enter</Text>
                <Text>{t("dialog.help4")}</Text>
                <Text weight="semibold">Space</Text>
                <Text>{t("dialog.help5")}</Text>
                <Text weight="semibold">s</Text>
                <Text>{t("dialog.help6")}</Text>
                <Text weight="semibold">o</Text>
                <Text>{t("dialog.help7")}</Text>
                <Text weight="semibold">f</Text>
                <Text>{t("dialog.help8")}</Text>
                <Text weight="semibold">r</Text>
                <Text>{t("dialog.help9")}</Text>
                <Text weight="semibold">m</Text>
                <Text>{t("dialog.help10")}</Text>
                <Text weight="semibold">/</Text>
                <Text>{t("dialog.help11")}</Text>
                <Text weight="semibold">?</Text>
                <Text>{t("dialog.help12")}</Text>
              </div>
            </DialogContent>
            <DialogActions>
              <DialogTrigger disableButtonEnhancement>
                <Button appearance="primary">{t("dialog.close")}</Button>
              </DialogTrigger>
            </DialogActions>
          </DialogBody>
        </DialogSurface>
      </Dialog>

      {/* ---------- 媒体播放弹窗 ---------- */}
      <MediaDialog
        open={playIdx != null}
        url={playIdx != null && mediaUrls[playIdx] ? mediaUrls[playIdx] : null}
        onClose={() => setPlayIdx(null)}
        onMinimize={(u) => {
          // 最小化到后台：关弹窗 + 独立小窗继续播放
          api.openMediaWindow(u, 420, 640);
          setPlayIdx(null);
        }}
      />

      <SettingsDialog
        open={showSettings}
        onClose={() => {
          setShowSettings(false);
          reloadData();
        }}
        onDataChanged={reloadData}
        themeMode={themeMode}
        setThemeMode={setThemeMode}
        decorations={decorations}
        setDecorations={setDecorations}
        fontSize={fontSize}
        onFontSizeChange={setFontSizePersist}
        cardSize={cardSize}
        onCardSizeChange={(s) => {
          setCardSize(s);
          api.setSetting("cardSize", s);
        }}
        articleSortOrder={sortOrder}
        onArticleSortOrderChange={(s: string) => {
          setSortOrder(s);
          api.setSetting("sort", s);
          loadArticles(selection, search, s);
        }}
        onClearCache={clearCacheAndRefresh}
      />

      {/* ---------- 底部状态栏 ---------- */}
      <div className={styles.statusBar}>
        <Text size={300} className={styles.statusText}>
          {imgStatus ? `${t("status.images")} ${imgStatus} ` : ""}
          {status}
        </Text>
        <Badge appearance="outline" color="brand" size="small">
          {unread.total} {t("sidebar.unread")}
        </Badge>
      </div>
      {/* 无边框窗口 resize 覆盖条 */}
      <EdgeResizer />
    </div>
  );
}
