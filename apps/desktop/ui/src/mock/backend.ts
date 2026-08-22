// 前端开发/测试用的 mock 后端。在普通浏览器里跑同一套 App.tsx，
// 用于复现和验证交互行为（菜单/切换/对话框/视图模式等）。
// 仅通过 vite 的 mock 别名注入，不影响真实 Tauri 构建。

export type MockInvoke = (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;

export const callLog: { cmd: string; args: unknown; time: number }[] = [];

interface MockArticle {
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

let articleSeq = 100;
function mkArticle(partial: Partial<MockArticle>): MockArticle {
  articleSeq += 1;
  return {
    id: articleSeq,
    feed_id: 1,
    title: "Untitled",
    url: "https://example.com/article",
    author: "Mock Author",
    summary: "<p>Summary of the article.</p>",
    content: null,
    content_fetched: false,
    published_at: new Date(Date.now() - 3600_000).toISOString(),
    fetched_at: new Date().toISOString(),
    is_read: false,
    is_starred: false,
    guid: `g${articleSeq}`,
    ...partial,
  };
}

interface MockFeed {
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

let feeds: MockFeed[] = [
  { id: 1, folder_id: 1, title: "Hacker News", url: "https://hnrss.org/frontpage", site_url: "https://news.ycombinator.com", description: null, favicon_url: null, last_updated: null, error: null, refresh_interval: null, use_proxy: false, default_original: false },
  { id: 2, folder_id: 1, title: "Rust Blog", url: "https://blog.rust-lang.org/feed.xml", site_url: "https://blog.rust-lang.org", description: null, favicon_url: null, last_updated: null, error: null, refresh_interval: null, use_proxy: false, default_original: false },
  { id: 3, folder_id: null, title: "BBC News", url: "https://feeds.bbci.co.uk/news/rss.xml", site_url: "https://www.bbc.com/news", description: null, favicon_url: null, last_updated: null, error: null, refresh_interval: null, use_proxy: false, default_original: false },
  { id: 4, folder_id: null, title: "机核网", url: "https://www.gcores.com/rss", site_url: "https://www.gcores.com", description: null, favicon_url: null, last_updated: null, error: null, refresh_interval: null, use_proxy: false, default_original: false },
];
let folders = [{ id: 1, name: "Tech", position: 0 }];
let articles: MockArticle[] = [
  mkArticle({ feed_id: 1, title: "Show HN: A museum for solid colors", url: "https://example.com/1", is_starred: true, content: "<p>Full content of article one with an <img src='https://picsum.photos/400/200'> inline image and a <b>bold</b> paragraph.</p><p>Second paragraph here.</p>" }),
  mkArticle({ feed_id: 1, title: "Linear algebra done right", url: "https://example.com/2", is_read: true, content: "<p>Content two.</p>" }),
  mkArticle({ feed_id: 2, title: "Announcing Rust 1.97", url: "https://example.com/3", content: "<p>Rust blog post with code <pre>fn main() {}</pre></p>" }),
  mkArticle({ feed_id: 2, title: "Crates.io improvements", url: "https://example.com/4", summary: "<p>Only a summary, no full content.</p>" }),
  mkArticle({ feed_id: 3, title: "World news roundup", url: "https://example.com/5", is_read: true }),
  mkArticle({ feed_id: 1, title: "Another HN story", url: "https://example.com/6", is_starred: true, content: "<h2>Heading</h2><p>Text with <a href='https://example.com'>a link</a>.</p>" }),
  mkArticle({ feed_id: 1, title: "One more test article", url: "https://example.com/7" }),
  mkArticle({ feed_id: 3, title: "Economy explained", url: "https://example.com/8", summary: "<p>Summary only.</p>" }),
  mkArticle({ feed_id: 1, title: "How does IKEA come up with names for its products?", summary: "HN 摘要含模板行\n\nArticle URL: https://www.ikea.com/se/en/customer-service/knowledge/articles/6f564c4d.html\n\nComments URL: https://news.ycombinator.com/item?id=49349984\n\nPoints: 196\n\n# Comments: 125", content: null }),
  mkArticle({ feed_id: 1, title: "截断摘要测试（RSS content 有截断，应自动抓全文）", content: "<p>这是 RSS 提供的截断内容，只有这一段话，没有全文也没有图片，长度超过八十个字符所以很容易被当成已有全文。</p>", summary: "<p>RSS 截断摘要。</p>" }),
  mkArticle({ feed_id: 1, title: "视频测试：内置 YouTube 播放器", url: "https://example.com/9", content: "<p>正文有播放器按钮。</p><div class=\"video\"><iframe src=\"https://www.youtube.com/embed/dQw4w9WgXcQ\" width=\"560\" height=\"315\"></iframe></div>" }),
  mkArticle({
    feed_id: 2,
    title: "5月音乐推荐（含播放器）",
    url: "https://example.com/media",
    content: `<p>文章正文，含防盗链图片和音乐播放器。</p><img src="https://cdn.sspai.com/2026/08/14/example.jpg" data-original="https://cdn.sspai.com/2026/08/14/example-orig.jpg"/><p>下面是一个网易云播放器 iframe：</p><iframe src="https://music.163.com/outchain/player?type=2&id=12345&auto=0" width="330" height="86"></iframe><p>以及一个直链音频：</p><audio src="https://cdn.example.com/song.mp3" controls></audio>`,
  }),
  mkArticle({
    feed_id: 4,
    title: "《影之刃零》试玩体验：硬核武侠动作的另一种解法",
    url: "https://example.com/gcores/1",
    author: "白广大",
    published_at: new Date().toISOString(),
    content: `<p>在今年的展会现场，《影之刃零》终于向玩家完整展示了它的战斗系统。作为一款强调"见招拆招"的武侠动作游戏，它的每一次拼刀、每一记弹反，都在试图回答一个问题：我们记忆里那些港式武侠电影的打斗，究竟能不能被做成可操作的游戏？</p><img src="https://image.gcores.com/demo-01-1600-900.jpg"/><p>开场不到十分钟，制作组就把玩家丢进了一场一对多的巷战。刀光、残影、敌人的惨叫混在一起，屏幕上几乎没有一秒是静止的。与传统动作游戏不同，本作的体力条并不限制攻击频率，而是鼓励你在一轮连招的末尾通过"架势互换"继续压制。</p><h2 id="h2-1">拼刀背后的博弈</h2><p>《影之刃零》的核心是"破招"系统。敌人出招瞬间，如果你能准确把握时机按下防御，就能进入短暂的子弹时间并触发高伤害反击。这种设计并不新鲜，但它与武侠题材的结合却产生了奇妙的化学反应——你不再是躲在盾牌后的骑士，而是要在刀光剑影中读出对手的意图。</p><blockquote>"武侠的精髓从来不是硬碰硬，而是在对方出招前的一刹那看穿破绽。"——制作人访谈</blockquote><h2 id="h2-2">美术与氛围</h2><p>游戏采用虚幻引擎 5 的 Lumen 全局光照，水墨风格的场景在昼夜交替时会呈现出完全不同的气质。雨夜的城楼、风沙中的驿站、大雪封山的古道——每一个场景都像是一幅会动的山水画。</p><img src="https://image.gcores.com/demo-02-1600-900.jpg"/><p>角色设计上，主角的造型参考了上世纪八十年代香港武侠片的美学，布衣、斗笠、长剑，配以克制的色彩，反而比花哨的"时装"更有辨识度。</p><h2 id="h2-3">目前的短板</h2><p>当然，试玩版本也存在一些遗憾：</p><ul><li>敌人的 AI 在低难度下略显迟钝，精英怪的读招行为还有优化空间</li><li>部分场景的加载时间偏长，读档后的演出衔接有些生硬</li><li>地图指引不够明确，容易在开阔区域迷失方向</li></ul><p>但这些都属于打磨阶段常见的问题。真正让人期待的是，制作组已经证明了自己有能力把"武侠"这个被反复消费的题材，做出属于自己的节奏。</p><h2 id="h2-4">结语</h2><p>如果你喜欢《只狼》式的战斗博弈，或者只是单纯怀念香港动作片里的刀光剑影，《影之刃零》都值得放入你的愿望单。它不完美，但它确实在认真地做一件难做的事——让武侠游戏重新变得"见招拆招"。</p>`,
  }),
];
let settings = new Map<string, string>([["theme", "light"]]);
let starredOnlyToggled = false;

function unreadOf(article: MockArticle) {
  return article.is_read ? 0 : 1;
}

async function dispatch(cmd: string, args: Record<string, unknown> = {}): Promise<unknown> {
  callLog.push({ cmd, args, time: Date.now() });
  const a = args as Record<string, number | string | boolean | null | undefined>;

  switch (cmd) {
    case "list_folders":
      return folders;
    case "list_feeds":
      return feeds;
    case "unread_stats": {
      const per_feed = feeds.map((f) => ({
        feed_id: f.id,
        folder_id: f.folder_id,
        title: f.title,
        unread: articles.filter((x) => x.feed_id === f.id && !x.is_read).length,
      }));
      return { total: articles.filter((x) => !x.is_read).length, per_feed };
    }
    case "list_articles": {
      let list = articles.slice();
      if (a.feedId != null) list = list.filter((x) => x.feed_id === a.feedId);
      if (a.folderId != null) {
        const ids = feeds.filter((f) => f.folder_id === a.folderId).map((f) => f.id);
        list = list.filter((x) => ids.includes(x.feed_id));
      }
      if (a.unreadOnly) list = list.filter((x) => !x.is_read);
      if (a.starredOnly) list = list.filter((x) => x.is_starred);
      if (typeof a.search === "string" && a.search) {
        const q = String(a.search).toLowerCase();
        list = list.filter(
          (x) =>
            (x.title ?? "").toLowerCase().includes(q) ||
            (x.summary ?? "").toLowerCase().includes(q) ||
            (x.content ?? "").toLowerCase().includes(q)
        );
      }
      list.sort((x, y) => {
        const s = String(a.sort ?? "desc");
        if (s === "asc") return (x.published_at ?? "").localeCompare(y.published_at ?? "");
        if (s === "unread") return Number(x.is_read) - Number(y.is_read) || (y.published_at ?? "").localeCompare(x.published_at ?? "");
        if (s === "starred") return Number(y.is_starred) - Number(x.is_starred) || (y.published_at ?? "").localeCompare(x.published_at ?? "");
        if (s === "title") return (x.title ?? "").localeCompare(y.title ?? "");
        return (y.published_at ?? "").localeCompare(x.published_at ?? "");
      });
      const limit = Number(a.limit ?? 200);
      const offset = Number(a.offset ?? 0);
      return list.slice(offset, offset + limit);
    }
    case "get_article": {
      return articles.find((x) => x.id === a.id) ?? null;
    }
    case "mark_read": {
      const ids = (a.ids as unknown as number[]) ?? [];
      const read = !!a.read;
      for (const x of articles) if (ids.includes(x.id)) x.is_read = read;
      return null;
    }
    case "mark_all_read":
      for (const x of articles) x.is_read = true;
      return null;
    case "mark_feed_read": {
      for (const x of articles) if (x.feed_id === a.feedId) x.is_read = true;
      return null;
    }
    case "mark_folder_read": {
      const ids = feeds.filter((f) => f.folder_id === a.folderId).map((f) => f.id);
      for (const x of articles) if (ids.includes(x.feed_id)) x.is_read = true;
      return null;
    }
    case "toggle_star": {
      const x = articles.find((y) => y.id === a.id);
      if (x) x.is_starred = !x.is_starred;
      return x?.is_starred ?? false;
    }
    case "fetch_full_content": {
      const x = articles.find((y) => y.id === a.id);
      if (x) {
        x.content_fetched = true;
        x.content = "<p>Fetched full content via readability-like mock.</p>";
      }
      return !!x;
    }
    case "add_feed": {
      const title = `New feed ${feeds.length + 1}`;
      const feed = { id: 100 + feeds.length, folder_id: (a.folderId as number | null) ?? null, title, url: String(a.url), site_url: null, description: null, favicon_url: null, last_updated: null, error: null, refresh_interval: null, use_proxy: false, default_original: false };
      feeds = [...feeds, feed];
      return { feed, articles_new: 2, existed: false };
    }
    case "remove_feed": {
      feeds = feeds.filter((f) => f.id !== a.id);
      articles = articles.filter((x) => x.feed_id !== a.id);
      return null;
    }
    case "add_folder": {
      folders = [...folders, { id: 200 + folders.length, name: String(a.name), position: folders.length }];
      return folders.length;
    }
    case "remove_folder": {
      folders = folders.filter((f) => f.id !== a.id);
      return null;
    }
    case "rename_folder":
      return null;
    case "refresh":
      return { feeds_checked: feeds.length, articles_new: 2, errors: [] };
    case "refresh_feed":
      return 1;
    case "import_opml_from":
      return { feeds_added: 3, feeds_existing: 1, errors: [] };
    case "export_opml_to":
      return null;
    case "export_opml":
      return "<?xml version=\"1.0\" encoding=\"UTF-8\"?><opml version=\"2.0\"><body></body></opml>";
    case "get_setting":
      return settings.get(String(a.key)) ?? null;
    case "set_setting": {
      settings.set(String(a.key), String(a.value));
      return null;
    }
    case "set_feed_folder": {
      const f = feeds.find((x) => x.id === a.feedId);
      if (f) f.folder_id = (a.folderId as number | null) ?? null;
      return null;
    }
    case "rename_feed": {
      const f = feeds.find((x) => x.id === a.feedId);
      if (f) f.title = String(a.title);
      return null;
    }
    case "set_feed_refresh_interval": {
      const f = feeds.find((x) => x.id === a.feedId);
      if (f) f.refresh_interval = (a.minutes as number | null) ?? null;
      return null;
    }
    case "prune_articles": {
      const days = Number(a.days ?? 30);
      const includeUnread = !!a.includeUnread;
      const cutoff = Date.now() - days * 86400000;
      const before = articles.length;
      articles = articles.filter(
        (x) => !(includeUnread || x.is_read) || (new Date(x.published_at ?? 0).getTime() >= cutoff)
      );
      return before - articles.length;
    }
    case "test_connection":
      return "OK 120ms (HTTP 200)";
    case "probe_url":
      return { kind: "html", content_type: "text/html", content: "", allow_embed: true };
    case "fetch_original_html":
      return {
        kind: "html",
        content_type: "text/html",
        content: `<html><head><style>body{font-family:sans-serif;padding:20px;line-height:1.6}</style></head><body><h1>原始网页（mock）</h1><p>模拟的原始网页 HTML，内联渲染。</p><img src="https://cdn.sspai.com/mock-original.png"></body></html>`,
      };
    case "fetch_image": {
      // 1x1 透明 PNG
      const png =
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=";
      return { content_type: "image/png", data_b64: png };
    }
    case "open_media_window":
      console.log("[mock] open_media_window:", a.url);
      return null;
    case "write_text_file":
      console.log("[mock] writeTextFile:", a.path, "len=", String(a.content ?? "").length);
      return null;
    default:
      throw new Error(`mock: unknown command ${cmd}`);
  }
}

export const mockInvoke: MockInvoke = async (cmd, args) => {
  // 模拟异步
  await new Promise((r) => setTimeout(r, 40));
  return dispatch(cmd, args ?? {});
};
