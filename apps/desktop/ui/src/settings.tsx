import React from "react";
import {
  Button,
  Checkbox,
  Dialog,
  DialogSurface,
  DialogBody,
  DialogTitle,
  DialogContent,
  DialogActions,
  Input,
  TabList,
  Tab,
  Text,
  Spinner,
  Radio,
  RadioGroup,
  Tooltip,
  makeStyles,
  tokens,
} from "@fluentui/react-components";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import {
  ArrowExportRegular,
  AddRegular,
  DeleteRegular,
  FolderAddRegular,
  RssRegular,
  ChevronDownRegular,
  ChevronRightRegular,
  DocumentRegular,
  DocumentSaveRegular,
  InfoRegular,
} from "@fluentui/react-icons";
import { api, Feed, Folder } from "./api";
import { useI18n } from "./i18n";

const useStyles = makeStyles({
  section: { marginBottom: "16px" },
  card: {
    background: tokens.colorNeutralBackground1,
    border: `1px solid ${tokens.colorNeutralStroke2}`,
    borderRadius: "8px",
    padding: "12px",
  },
  row: { display: "flex", alignItems: "center", gap: "10px", marginTop: "8px" },
  desc: { color: tokens.colorNeutralForeground3, marginTop: "4px" },
  mono: { fontFamily: "monospace", fontSize: "12px" },
  aboutLine: { marginTop: "4px" },
  aboutWrap: {
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    gap: "10px",
    padding: "32px 0 16px",
  },
  aboutLogo: {
    fontFamily: "ui-monospace, SFMono-Regular, Consolas, monospace",
    fontSize: "11px",
    lineHeight: 1.25,
    color: tokens.colorBrandForeground1,
    margin: 0,
    marginBottom: "18px",
    textAlign: "center",
    userSelect: "none",
  },
  aboutDesc: {
    color: tokens.colorNeutralForeground3,
    textAlign: "center",
    maxWidth: "420px",
  },
  aboutMeta: {
    marginTop: "16px",
    display: "flex",
    flexDirection: "column",
    alignItems: "center",
    gap: "4px",
  },
  tabs: { marginBottom: "14px" },
  feedRow: {
    display: "flex",
    alignItems: "center",
    gap: "8px",
    padding: "6px 8px",
    borderRadius: "6px",
    borderBottom: `1px solid ${tokens.colorNeutralStroke2}`,
    "&:hover": { backgroundColor: tokens.colorNeutralBackground1Hover },
  },
  feedRowSelected: {
    backgroundColor: tokens.colorBrandBackground2,
  },
  feedTitle: { flex: 1, minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", cursor: "pointer" },
  feedActions: { opacity: 0, "&:hover": { opacity: 1 } },
  favicon: { width: "16px", height: "16px", borderRadius: "3px", flexShrink: 0 },
  groupRow: {
    display: "flex",
    alignItems: "center",
    gap: "8px",
    padding: "4px 8px",
  },
  label: { width: "84px", flexShrink: 0 },
  feedsLayout: { display: "flex", gap: "14px", minHeight: "320px" },
  feedsTree: {
    width: "218px",
    flexShrink: 0,
    borderRight: `1px solid ${tokens.colorNeutralStroke2}`,
    paddingRight: "8px",
    overflowY: "auto",
    maxHeight: "360px",
  },
  groupHeader: {
    display: "flex",
    alignItems: "center",
    gap: "4px",
    padding: "5px 6px",
    borderRadius: "6px",
    cursor: "pointer",
    color: tokens.colorNeutralForeground1,
    "&:hover": { backgroundColor: tokens.colorNeutralBackground1Hover },
  },
  treeFeed: {
    display: "flex",
    alignItems: "center",
    gap: "6px",
    padding: "4px 6px 4px 26px",
    borderRadius: "6px",
    cursor: "pointer",
    "&:hover": { backgroundColor: tokens.colorNeutralBackground1Hover },
  },
  treeFeedSelected: { backgroundColor: tokens.colorBrandBackground2 },
  panel: { flex: 1, minWidth: 0 },
  panelTitle: { fontSize: "15px", marginBottom: "6px" },
  panelRow: {
    display: "flex",
    alignItems: "center",
    gap: "8px",
    marginTop: "8px",
  },
  panelLabel: {
    width: "72px",
    flexShrink: 0,
    color: tokens.colorNeutralForeground3,
    fontSize: "12px",
  },
  panelUrl: {
    fontSize: "12px",
    color: tokens.colorNeutralForeground3,
    wordBreak: "break-all",
    marginTop: "2px",
  },
});

type Props = {
  open: boolean;
  onClose: () => void;
  onDataChanged: () => void;
  themeMode: "system" | "light" | "dark";
  setThemeMode: (m: "system" | "light" | "dark") => void;
  decorations: boolean;
  setDecorations: (d: boolean) => void;
  fontSize: number;
  onFontSizeChange: (n: number) => void;
  cardSize: "small" | "medium" | "large";
  onCardSizeChange: (s: "small" | "medium" | "large") => void;
  articleSortOrder: string;
  onArticleSortOrderChange: (s: string) => void;
  /** 清缓存 + 全局强制刷新（由 App 层执行，带进度反馈）。 */
  onClearCache: () => Promise<void>;
};

const INTERVAL_OPTIONS: Array<[string, string]> = [
  ["", "跟随全局"],
  ["30", "30 分钟"],
  ["60", "1 小时"],
  ["120", "2 小时"],
  ["360", "6 小时"],
  ["1440", "每天"],
];

export default function SettingsDialog({
  open,
  onClose,
  onDataChanged,
  themeMode,
  setThemeMode,
  decorations,
  setDecorations,
  fontSize,
  onFontSizeChange,
  cardSize,
  onCardSizeChange,
  articleSortOrder,
  onArticleSortOrderChange,
  onClearCache,
}: Props) {
  const styles = useStyles();
  const { lang, setLang, t } = useI18n();
  const [tab, setTab] = React.useState("appearance");

  // 订阅源数据
  const [folders, setFolders] = React.useState<Folder[]>([]);
  const [feeds, setFeeds] = React.useState<Feed[]>([]);
  const [selectedFeedId, setSelectedFeedId] = React.useState<number | null>(null);
  const [selectedGroupId, setSelectedGroupId] = React.useState<number | null>(null);
  const [collapsedGroups, setCollapsedGroups] = React.useState<Record<string, boolean>>({});
  const [newFeedUrl, setNewFeedUrl] = React.useState("");
  const [newFolderName, setNewFolderName] = React.useState("");
  const [renameTitle, setRenameTitle] = React.useState("");
  const [feedUrlEdit, setFeedUrlEdit] = React.useState("");

  // 代理
  const [proxy, setProxy] = React.useState("");
  const [proxyTest, setProxyTest] = React.useState<null | "testing" | "ok" | "fail">(null);
  const [proxyMsg, setProxyMsg] = React.useState("");

  // 应用设置
  const [dataDir, setDataDir] = React.useState("");
  const [headlessEnabled, setHeadlessEnabled] = React.useState(false);
  const [forceHeadlessRender, setForceHeadlessRender] = React.useState(false);
  const [originalViewMode, setOriginalViewMode] = React.useState<"snapshot" | "page">("snapshot");
  const [migrate, setMigrate] = React.useState(true);
  const [globalInterval, setGlobalInterval] = React.useState("30");
  const [pruneDays, setPruneDays] = React.useState("30");
  const [pruneIncludeUnread, setPruneIncludeUnread] = React.useState(false);

  const [result, setResult] = React.useState<string | null>(null);
  const [importErrors, setImportErrors] = React.useState<string[]>([]);
  const [info, setInfo] = React.useState<{ version: string; tauri_version: string; license: string } | null>(null);

  const reloadFeeds = React.useCallback(async () => {
    const [f, fs] = await Promise.all([api.listFolders(), api.listFeeds()]);
    setFolders(f);
    setFeeds(fs);
  }, []);

  // 选中订阅时同步 URL 编辑框
  React.useEffect(() => {
    if (selectedFeedId == null) return;
    const f = feeds.find((x) => x.id === selectedFeedId);
    setFeedUrlEdit(f?.url ?? "");
  }, [selectedFeedId, feeds]);

  React.useEffect(() => {
    if (!open) return;
    api.getProxy().then((p) => setProxy(p ?? ""));
    api.getDataDir().then(setDataDir);
    api.getAppInfo().then(setInfo).catch(() => {});
    api.getSetting("refresh_interval").then((v) => v && setGlobalInterval(v));
    api.getSetting("prune_days").then((v) => v && setPruneDays(v));
    api.getSetting("prune_include_unread").then((v) => setPruneIncludeUnread(v === "1"));
    api.getSetting("headless_enabled").then((v) => setHeadlessEnabled(v === "1" || v === "true"));
    api.getSetting("force_headless_render").then((v) => setForceHeadlessRender(v === "1" || v === "true"));
    api.getSetting("original_view_mode").then((v) => {
      if (v === "page") setOriginalViewMode("page");
    });
    reloadFeeds();
    setProxyTest(null);
    setProxyMsg("");
    setResult(null);
    setImportErrors([]);
    setSelectedFeedId(null);
    setSelectedGroupId(null);
  }, [open, reloadFeeds]);

  const change = () => {
    onDataChanged();
  };

  // ---------- 外观 ----------

  // ---------- 订阅源操作 ----------
  const addFeed = async () => {
    if (!newFeedUrl.trim()) return;
    try {
      await api.addFeed(newFeedUrl.trim(), null, false);
      setNewFeedUrl("");
      setResult(t("status.added", { title: newFeedUrl, n: 0 }));
      await reloadFeeds();
      change();
    } catch (e) {
      setResult(t("status.addFailed") + e);
    }
  };

  const addFolder = async () => {
    if (!newFolderName.trim()) return;
    try {
      await api.addFolder(newFolderName.trim());
      setNewFolderName("");
      await reloadFeeds();
      change();
    } catch (e) {
      setResult(String(e));
    }
  };

  const saveRename = async (id: number, isFolder = false) => {
    if (!renameTitle.trim()) return;
    try {
      if (isFolder) {
        await api.renameFolder(id, renameTitle.trim());
        setSelectedGroupId(null);
      } else {
        await api.renameFeed(id, renameTitle.trim());
        setSelectedFeedId(null);
      }
      setRenameTitle("");
      await reloadFeeds();
      change();
    } catch (e) {
      setResult(String(e));
    }
  };

  const importOpml = async () => {
    const path = await openDialog({ multiple: false, filters: [{ name: "OPML", extensions: ["opml", "xml"] }] });
    if (!path) return;
    try {
      const res = await api.importOpmlFrom(path as string);
      setImportErrors(res.errors);
      setResult(t("status.imported", { a: res.feeds_added, e: res.feeds_existing, err: res.errors.length }));
      await reloadFeeds();
      change();
    } catch (e) {
      setImportErrors([]);
      setResult(t("status.importFailed") + e);
    }
  };

  const exportOpml = async () => {
    const path = await saveDialog({ defaultPath: "rss-reader.opml", filters: [{ name: "OPML", extensions: ["opml", "xml"] }] });
    if (!path) return;
    try {
      await api.exportOpmlTo(path as string);
      setResult(t("status.exported", { path: String(path) }));
    } catch (e) {
      setResult(t("status.exportFailed") + e);
    }
  };

  // ---------- 代理 / 数据 ----------
  const saveProxy = async () => {
    const value = proxy.trim();
    try {
      await api.setProxy(value || null);
      setProxyTest("ok");
      setProxyMsg(t("settings.proxySaved"));
    } catch (e) {
      setProxyTest("fail");
      setProxyMsg(t("settings.proxyFail") + e);
    }
  };

  const testProxy = async () => {
    setProxyTest("testing");
    setProxyMsg("");
    try {
      const value = proxy.trim();
      await api.setProxy(value || null);
      const r = await api.testConnection();
      setProxyTest(r.startsWith("OK") ? "ok" : "fail");
      setProxyMsg(r);
    } catch (e) {
      setProxyTest("fail");
      setProxyMsg(t("settings.proxyFail") + String(e));
    }
  };

  const applyDataDir = async () => {
    try {
      await api.setDataDir(dataDir, migrate);
      setResult(t("settings.dataDirRestart"));
    } catch (e) {
      setResult(String(e));
    }
  };

  const clearCache = async () => {
    try {
      setResult(t("status.clearingCache"));
      await onClearCache();
      await reloadFeeds();
      setResult(t("settings.clearCacheDone", { n: 0 }));
    } catch (e) {
      setResult(String(e));
    }
  };

  const applyGlobalInterval = async () => {
    const n = Number(globalInterval);
    if (n >= 1) {
      await api.setSetting("refresh_interval", String(n));
      setResult(t("settings.saved"));
    }
  };

  const applyPrune = async () => {
    const days = Number(pruneDays);
    if (days >= 1) {
      await api.setSetting("prune_days", String(days));
      await api.setSetting("prune_include_unread", pruneIncludeUnread ? "1" : "0");
      const n = await api.pruneArticles(days, pruneIncludeUnread);
      setResult(t("settings.pruned", { n }));
      change();
    }
  };

  const selectedFeed = feeds.find((f) => f.id === selectedFeedId);

  const nativeSel = (style: React.CSSProperties = {}) =>
    ({
      padding: "5px 26px 5px 10px",
      borderRadius: "6px",
      border: `1px solid ${tokens.colorNeutralStroke1}`,
      appearance: "none" as const,
      WebkitAppearance: "none",
      background: `linear-gradient(45deg, transparent 50%, ${tokens.colorNeutralForeground3} 50%) calc(100% - 14px) 50%/5px 5px no-repeat, linear-gradient(135deg, ${tokens.colorNeutralForeground3} 50%, transparent 50%) calc(100% - 9px) 50%/5px 5px no-repeat, ${tokens.colorNeutralBackground1}`,
      color: tokens.colorNeutralForeground1,
      outline: "none",
      font: "inherit",
      cursor: "pointer",
      transition: "border-color .1s ease",
      ...style,
    }) as const;

  return (
    <Dialog open={open} onOpenChange={(_e, d) => !d.open && onClose()}>
      <DialogSurface style={{ maxWidth: 640, width: "92vw", height: "min(76vh, 680px)" }}>
        <DialogBody style={{ height: "100%", display: "flex", flexDirection: "column" }}>
          <DialogTitle>{t("settings.title")}</DialogTitle>
          <DialogContent style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column", padding: "0 24px 24px" }}>
            {/* Tab 固定（不随内容滚动）；独立容器避免 flex 拉伸干扰选中指示器定位 */}
            <div style={{ flexShrink: 0, width: "100%" }}>
              <TabList className={styles.tabs} selectedValue={tab} onTabSelect={(_e, d) => setTab(d.value as string)}>
                <Tab value="appearance">{t("settings.tabAppearance")}</Tab>
                <Tab value="feeds">{t("settings.tabFeeds")}</Tab>
                <Tab value="app">{t("settings.tabApp")}</Tab>
                <Tab value="about">{t("settings.tabAbout")}</Tab>
              </TabList>
            </div>
            <div style={{ flex: 1, overflowY: "auto", minHeight: 0 }}>

            {/* ---------- 外观 ---------- */}
            {tab === "appearance" && (
              <>
                <div className={`${styles.section} ${styles.card}`}>
                  <Text weight="semibold" size={400}>{t("settings.language")}</Text>
                  <div className={styles.row}>
                    <select
                      value={lang}
                      onChange={(e) => setLang(e.target.value as "zh" | "en")}
                      style={nativeSel({ minWidth: 160 }) as React.CSSProperties}
                    >
                      <option value="zh">{t("settings.languageZh")}</option>
                      <option value="en">{t("settings.languageEn")}</option>
                    </select>
                  </div>
                </div>
                <div className={`${styles.section} ${styles.card}`}>
                  <Text weight="semibold" size={400}>{t("settings.theme")}</Text>
                  <div className={styles.row}>
                    <RadioGroup value={themeMode} onChange={(_e, d) => setThemeMode(d.value as "system" | "light" | "dark")}>
                      <Radio value="system" label={t("settings.themeSystem")} />
                      <Radio value="light" label={t("settings.themeLight")} />
                      <Radio value="dark" label={t("settings.themeDark")} />
                    </RadioGroup>
                  </div>
                </div>
                <div className={`${styles.section} ${styles.card}`}>
                  <Text weight="semibold" size={400}>{t("settings.defaultFontSize")}</Text>
                  <div className={styles.row}>
                    <Input
                      type="number"
                      min={8}
                      value={String(fontSize)}
                      onChange={(_e, d) => onFontSizeChange(Number(d.value) || 16)}
                      style={{ width: 120 }}
                    />
                  </div>
                </div>
                <div className={`${styles.section} ${styles.card}`}>
                  <Text weight="semibold" size={400}>{t("settings.sortOrder")}</Text>
                  <div className={styles.row}>
                    <select
                      value={articleSortOrder}
                      onChange={(e) => onArticleSortOrderChange(e.target.value)}
                      style={nativeSel({ minWidth: 160 }) as React.CSSProperties}
                    >
                      <option value="desc">{t("settings.sortDesc")}</option>
                      <option value="asc">{t("settings.sortAsc")}</option>
                      <option value="unread">{t("settings.sortUnread")}</option>
                      <option value="starred">{t("settings.sortStarred")}</option>
                      <option value="title">{t("settings.sortTitle")}</option>
                    </select>
                  </div>
                </div>
                <div className={`${styles.section} ${styles.card}`}>
                  <Text weight="semibold" size={400}>{t("settings.cardSize")}</Text>
                  <div className={styles.row}>
                    <RadioGroup value={cardSize} onChange={(_e, d) => onCardSizeChange(d.value as "small" | "medium" | "large")}>
                      <Radio value="small" label={t("settings.cardSmall")} />
                      <Radio value="medium" label={t("settings.cardMedium")} />
                      <Radio value="large" label={t("settings.cardLarge")} />
                    </RadioGroup>
                  </div>
                </div>
                <div className={`${styles.section} ${styles.card}`}>
                  <Text weight="semibold" size={400}>{t("settings.decorations")}</Text>
                  <div className={styles.row}>
                    <Checkbox
                      checked={decorations}
                      onChange={(_e, d) => setDecorations(d.checked === true)}
                      label={t("settings.decorationsDesc")}
                    />
                  </div>
                </div>
              </>
            )}

            {/* ---------- 订阅源 ---------- */}
            {tab === "feeds" && (
              <>
                <div className={styles.section}>
                  <Text weight="semibold" size={400}>{t("settings.feedManage")}</Text>
                  <div className={styles.row}>
                    <Input placeholder="https://example.com/feed.xml" value={newFeedUrl} onChange={(_e, d) => setNewFeedUrl(d.value)} style={{ flex: 1 }} />
                    <Button size="small" icon={<AddRegular />} title={t("toolbar.addFeed")} onClick={addFeed} />
                    <Button size="small" icon={<DocumentRegular />} title={t("toolbar.importOpml")} onClick={importOpml} />
                    <Button size="small" icon={<DocumentSaveRegular />} title={t("toolbar.exportOpml")} onClick={exportOpml} />
                  </div>
                </div>

                <div className={styles.section}>
                  <Text weight="semibold" size={400}>{t("settings.groups")}</Text>
                  <div className={styles.row}>
                    <Input placeholder={t("dialog.folderPlaceholder")} value={newFolderName} onChange={(_e, d) => setNewFolderName(d.value)} style={{ flex: 1 }} />
                    <Button icon={<FolderAddRegular />} onClick={addFolder}>{t("toolbar.newFolder")}</Button>
                  </div>
                </div>

                <div className={`${styles.card} ${styles.feedsLayout}`}>
                  {/* 左：分组树 */}
                  <div className={styles.feedsTree}>
                    <div
                      className={`${styles.groupHeader} ${selectedFeedId == null && selectedGroupId == null ? styles.treeFeedSelected : ""}`}
                      onClick={() => { setSelectedGroupId(null); setSelectedFeedId(null); setRenameTitle(""); }}
                    >
                      <RssRegular style={{ width: 15, height: 15 }} />
                      <Text size={300} truncate>{t("settings.allFeeds")}</Text>
                    </div>
                    {folders.map((folder) => {
                      const key = String(folder.id);
                      const collapsed = collapsedGroups[key] === true;
                      const count = feeds.filter((f) => f.folder_id === folder.id).length;
                      return (
                        <div key={key}>
                          <div
                            className={`${styles.groupHeader} ${selectedGroupId === folder.id ? styles.treeFeedSelected : ""}`}
                            onClick={(e) => {
                              e.stopPropagation();
                              if (selectedGroupId === folder.id) {
                                setCollapsedGroups((p) => ({ ...p, [key]: !collapsed }));
                              } else {
                                setSelectedGroupId(folder.id);
                                setSelectedFeedId(null);
                                setRenameTitle("");
                              }
                            }}
                          >
                            {collapsed ? <ChevronRightRegular style={{ width: 13, height: 13 }} /> : <ChevronDownRegular style={{ width: 13, height: 13 }} />}
                            <Text size={300} truncate style={{ flex: 1 }}>{folder.name}</Text>
                            <Text size={200} style={{ color: tokens.colorNeutralForeground3 }}>{count}</Text>
                          </div>
                          {!collapsed && (
                            <div>
                              {feeds.filter((f) => f.folder_id === folder.id).map((feed) => (
                                <div
                                  key={feed.id}
                                  className={`${styles.treeFeed} ${selectedFeedId === feed.id ? styles.treeFeedSelected : ""}`}
                                  onClick={() => { setSelectedFeedId(feed.id); setSelectedGroupId(null); setRenameTitle(feed.title); }}
                                >
                                  {feed.favicon_url ? (
                                    <img src={feed.favicon_url} alt="" width={15} height={15} className={styles.favicon} />
                                  ) : (
                                    <RssRegular style={{ width: 13, height: 13, flexShrink: 0 }} />
                                  )}
                                  <Text size={300} truncate>{feed.title}</Text>
                                </div>
                              ))}
                            </div>
                          )}
                        </div>
                      );
                    })}
                    {/* 未分组 */}
                    {(() => {
                      const ungrouped = feeds.filter((f) => f.folder_id == null);
                      const collapsed = collapsedGroups["__ungrouped"] === true;
                      return (
                        <div key="__ungrouped">
                          <div
                            className={`${styles.groupHeader} ${selectedGroupId === -1 ? styles.treeFeedSelected : ""}`}
                            onClick={(e) => {
                              e.stopPropagation();
                              if (selectedGroupId === -1) {
                                setCollapsedGroups((p) => ({ ...p, __ungrouped: !collapsed }));
                              } else {
                                setSelectedGroupId(-1);
                                setSelectedFeedId(null);
                                setRenameTitle("");
                              }
                            }}
                          >
                            {collapsed ? <ChevronRightRegular style={{ width: 13, height: 13 }} /> : <ChevronDownRegular style={{ width: 13, height: 13 }} />}
                            <Text size={300} truncate style={{ flex: 1 }}>{t("sidebar.ungrouped")}</Text>
                            <Text size={200} style={{ color: tokens.colorNeutralForeground3 }}>{ungrouped.length}</Text>
                          </div>
                          {!collapsed && (
                            <div>
                              {ungrouped.map((feed) => (
                                <div
                                  key={feed.id}
                                  className={`${styles.treeFeed} ${selectedFeedId === feed.id ? styles.treeFeedSelected : ""}`}
                                  onClick={() => { setSelectedFeedId(feed.id); setSelectedGroupId(null); setRenameTitle(feed.title); }}
                                >
                                  {feed.favicon_url ? (
                                    <img src={feed.favicon_url} alt="" width={15} height={15} className={styles.favicon} />
                                  ) : (
                                    <RssRegular style={{ width: 13, height: 13, flexShrink: 0 }} />
                                  )}
                                  <Text size={300} truncate>{feed.title}</Text>
                                </div>
                              ))}
                            </div>
                          )}
                        </div>
                      );
                    })()}
                  </div>

                  {/* 右：编辑面板 */}
                  <div className={styles.panel}>
                    {selectedFeedId != null && (
                      (() => {
                        const feed = feeds.find((f) => f.id === selectedFeedId);
                        if (!feed) return null;
                        return (
                          <>
                            {/* 标题：直接输入改名，失焦/回车自动保存 */}
                            <div className={styles.panelRow}>
                              <Input
                                value={renameTitle}
                                onChange={(_e, d) => setRenameTitle(d.value)}
                                onBlur={() => {
                                  const t = renameTitle.trim();
                                  if (t && t !== feed.title) saveRename(feed.id);
                                }}
                                onKeyDown={(e) => {
                                  if (e.key === "Enter") (e.target as HTMLInputElement).blur();
                                }}
                                placeholder={feed.title}
                                style={{ flex: 1, fontWeight: 600 }}
                              />
                            </div>
                            {/* URL：失焦/回车自动保存 */}
                            <div className={styles.panelRow}>
                              <Text size={300} className={styles.panelLabel}>{t("settings.feedUrl")}</Text>
                              <Input
                                value={feedUrlEdit}
                                onChange={(_e, d) => setFeedUrlEdit(d.value)}
                                onBlur={() => {
                                  const u = feedUrlEdit.trim();
                                  if (!u || u === feed.url) return;
                                  api.updateFeedUrl(feed.id, u)
                                    .then(() => reloadFeeds())
                                    .then(() => change())
                                    .catch((e) => setResult(String(e)));
                                }}
                                onKeyDown={(e) => {
                                  if (e.key === "Enter") (e.target as HTMLInputElement).blur();
                                }}
                                style={{ flex: 1 }}
                              />
                            </div>
                            <div className={styles.panelRow}>
                              <Text size={300} className={styles.panelLabel}>{t("settings.feedGroup")}</Text>
                              <select
                                value={feed.folder_id ?? ""}
                                style={nativeSel({ flex: 1 })}
                                onChange={async (e) => {
                                  const v = e.target.value;
                                  await api.setFeedFolder(feed.id, v === "" ? null : Number(v));
                                  await reloadFeeds();
                                  change();
                                }}
                              >
                                <option value="">{t("sidebar.ungrouped")}</option>
                                {folders.map((folder) => (
                                  <option key={folder.id} value={folder.id}>{folder.name}</option>
                                ))}
                              </select>
                            </div>
                            <div className={styles.panelRow}>
                              <Text size={300} className={styles.panelLabel}>{t("settings.refreshInterval")}</Text>
                              <select
                                value={feed.refresh_interval ?? ""}
                                style={nativeSel({ flex: 1 })}
                                onChange={async (e) => {
                                  const v = e.target.value;
                                  await api.setFeedRefreshInterval(feed.id, v === "" ? null : Number(v));
                                  await reloadFeeds();
                                  change();
                                }}
                              >
                                {INTERVAL_OPTIONS.map(([val, label]) => (
                                  <option key={val || "default"} value={val}>{label}</option>
                                ))}
                              </select>
                            </div>
                            <div className={styles.panelRow}>
                              <Checkbox
                                checked={feed.use_proxy}
                                onChange={async (_e, d) => {
                                  await api.setFeedUseProxy(feed.id, d.checked === true);
                                  await reloadFeeds();
                                  change();
                                }}
                                label={t("settings.feedUseProxy")}
                              />
                            </div>
                            <div className={styles.panelRow}>
                              <Checkbox
                                checked={feed.default_original}
                                onChange={async (_e, d) => {
                                  await api.setFeedDefaultOriginal(feed.id, d.checked === true);
                                  await reloadFeeds();
                                  change();
                                }}
                                label={t("settings.feedDefaultOriginal")}
                              />
                            </div>
                            <div className={styles.panelRow} style={{ marginTop: "20px" }}>
                              <Button
                                appearance="secondary"
                                icon={<DeleteRegular />}
                                onClick={async () => {
                                  await api.removeFeed(feed.id);
                                  setSelectedFeedId(null);
                                  await reloadFeeds();
                                  change();
                                }}
                              >
                                {t("menu.deleteFeed")}
                              </Button>
                            </div>
                          </>
                        );
                      })()
                    )}
                    {selectedGroupId != null && selectedFeedId == null && (
                      (() => {
                        if (selectedGroupId === -1) {
                          return (
                            <>
                              <Text weight="semibold" size={500} className={styles.panelTitle}>{t("sidebar.ungrouped")}</Text>
                              <Text size={300} style={{ color: tokens.colorNeutralForeground3 }}>
                                {t("settings.ungroupedDesc")}
                              </Text>
                            </>
                          );
                        }
                        const folder = folders.find((f) => f.id === selectedGroupId);
                        if (!folder) return null;
                        return (
                          <>
                            <Text weight="semibold" size={500} className={styles.panelTitle}>{folder.name}</Text>
                            <div className={styles.panelRow}>
                              <Input value={renameTitle} onChange={(_e, d) => setRenameTitle(d.value)} placeholder={t("settings.renameHint")} style={{ flex: 1 }} />
                              <Button size="small" appearance="primary" onClick={() => saveRename(folder.id, true)}>{t("settings.renameBtn")}</Button>
                            </div>
                            <div className={styles.panelRow} style={{ marginTop: "16px" }}>
                              <Button
                                appearance="secondary"
                                icon={<DeleteRegular />}
                                onClick={async () => {
                                  await api.removeFolder(folder.id);
                                  setSelectedGroupId(null);
                                  await reloadFeeds();
                                  change();
                                }}
                              >
                                {t("menu.deleteFolder")}
                              </Button>
                            </div>
                          </>
                        );
                      })()
                    )}
                    {selectedFeedId == null && selectedGroupId == null && (
                      <Text size={300} style={{ color: tokens.colorNeutralForeground3 }}>
                        {t("settings.feedListHint")}
                      </Text>
                    )}
                  </div>
                </div>
              </>
            )}

            {/* ---------- 应用设置 ---------- */}
            {tab === "app" && (
              <>
                <div className={`${styles.section} ${styles.card}`}>
                  <Text weight="semibold" size={400}>{t("settings.globalInterval")}</Text>
                  <div className={styles.row}>
                    <Input type="number" min={1} value={globalInterval} onChange={(_e, d) => setGlobalInterval(d.value)} style={{ width: 120 }} />
                    <Text size={300} className={styles.desc}>{t("settings.minutes")}</Text>
                    <Button onClick={applyGlobalInterval}>{t("settings.apply")}</Button>
                  </div>
                </div>

                <div className={`${styles.section} ${styles.card}`}>
                  <Text weight="semibold" size={400}>{t("settings.headless")}</Text>
                  <div className={styles.row}>
                    <Checkbox
                      checked={headlessEnabled}
                      onChange={async (_e, d) => {
                        const v = d.checked === true;
                        setHeadlessEnabled(v);
                        await api.setSetting("headless_enabled", v ? "1" : "0");
                      }}
                      label={t("settings.headlessDesc")}
                    />
                  </div>
                  <div className={styles.row}>
                    <Checkbox
                      checked={forceHeadlessRender}
                      onChange={async (_e, d) => {
                        const v = d.checked === true;
                        setForceHeadlessRender(v);
                        await api.setSetting("force_headless_render", v ? "1" : "0");
                      }}
                      label={t("settings.headlessOpenOriginal")}
                    />
                  </div>
                  <div className={styles.row}>
                    <RadioGroup
                      layout="vertical"
                      value={originalViewMode}
                      onChange={async (_e, d) => {
                        const v = d.value === "page" ? "page" : "snapshot";
                        setOriginalViewMode(v);
                        await api.setSetting("original_view_mode", v);
                      }}
                    >
                      <div className={styles.row} style={{ marginTop: 0 }}>
                        <Text size={300} weight="semibold">{t("settings.originalViewModeTitle")}</Text>
                        <Tooltip content={t("settings.originalViewMode")} relationship="label">
                          <InfoRegular style={{ color: tokens.colorNeutralForeground3, cursor: "help" }} />
                        </Tooltip>
                      </div>
                      <Radio
                        value="snapshot"
                        label={
                          <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
                            {t("settings.originalViewSnapshot")}
                            <Tooltip content={t("settings.originalViewSnapshotDesc")} relationship="label">
                              <InfoRegular style={{ color: tokens.colorNeutralForeground3, cursor: "help", fontSize: 12 }} />
                            </Tooltip>
                          </span>
                        }
                      />
                      <Radio
                        value="page"
                        label={
                          <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
                            {t("settings.originalViewPage")}
                            <Tooltip content={t("settings.originalViewPageDesc")} relationship="label">
                              <InfoRegular style={{ color: tokens.colorNeutralForeground3, cursor: "help", fontSize: 12 }} />
                            </Tooltip>
                          </span>
                        }
                      />
                    </RadioGroup>
                  </div>
                </div>

                <div className={`${styles.section} ${styles.card}`}>
                  <Text weight="semibold" size={400}>{t("settings.prune")}</Text>
                  <div className={styles.row}>
                    <Text size={300} className={styles.label}>{t("settings.pruneDays")}</Text>
                    <Input type="number" min={1} value={pruneDays} onChange={(_e, d) => setPruneDays(d.value)} style={{ width: 100 }} />
                    <Text size={300} className={styles.desc}>{t("settings.days")}</Text>
                  </div>
                  <div className={styles.row}>
                    <Checkbox checked={pruneIncludeUnread} onChange={(_e, d) => setPruneIncludeUnread(d.checked === true)} label={t("settings.pruneIncludeUnread")} />
                  </div>
                  <div className={styles.row}>
                    <Button onClick={applyPrune}>{t("settings.pruneNow")}</Button>
                  </div>
                </div>

                <div className={`${styles.section} ${styles.card}`}>
                  <Text weight="semibold" size={400}>{t("settings.proxy")}</Text>
                  <div className={styles.row}>
                    <Input placeholder={t("settings.proxyPlaceholder")} value={proxy} onChange={(_e, d) => setProxy(d.value)} style={{ flex: 1 }} />
                  </div>
                  <div className={styles.row}>
                    <Button onClick={saveProxy}>{t("settings.proxySave")}</Button>
                    <Button onClick={testProxy} disabled={proxyTest === "testing"}>
                      {proxyTest === "testing" ? <Spinner size="tiny" /> : t("settings.proxyTest")}
                    </Button>
                    {proxyTest && (
                      <Text size={300} style={{ color: proxyTest === "ok" ? tokens.colorStatusSuccessForeground1 : tokens.colorStatusDangerForeground1 }}>
                        {proxyMsg}
                      </Text>
                    )}
                  </div>
                </div>

                <div className={`${styles.section} ${styles.card}`}>
                  <Text weight="semibold" size={400}>{t("settings.dataDir")}</Text>
                  <div className={styles.desc}>{t("settings.dataDirCurrent")}: <Text className={styles.mono}>{dataDir || "…"}</Text></div>
                  <div className={styles.row}>
                    <Input value={dataDir} onChange={(_e, d) => setDataDir(d.value)} style={{ flex: 1 }} />
                    <Button onClick={async () => { const dir = await openDialog({ directory: true, multiple: false }); if (typeof dir === "string") { setDataDir(dir); setResult(t("settings.dataDirChosen")); } }}>
                      {t("settings.dataDirChoose")}
                    </Button>
                  </div>
                  <div className={styles.row}>
                    <Checkbox checked={migrate} onChange={(_e, d) => setMigrate(d.checked === true)} label={t("settings.dataDirMigrate")} />
                  </div>
                  <div className={styles.row}>
                    <Button onClick={applyDataDir}>{t("settings.dataDirSave")}</Button>
                  </div>
                </div>

                <div className={`${styles.section} ${styles.card}`}>
                  <Text weight="semibold" size={400}>{t("settings.clearCache")}</Text>
                  <div className={styles.desc}>{t("settings.clearCacheDesc")}</div>
                  <div className={styles.row}>
                    <Button onClick={clearCache}>{t("settings.clearCacheBtn")}</Button>
                  </div>
                </div>
              </>
            )}

            {/* ---------- 关于 ---------- */}
            {tab === "about" && (
              <div className={styles.aboutWrap}>
                <pre className={styles.aboutLogo}>{`████╗ ████╗ ████╗
██╔═╝ ██╔═╝ ██╔═╝
████╗ ████╗ ████╗
╚═██╗ ╚═██╗ ╚═██╗
████╝ ████╝ ████╝`}</pre>
                <Text size={500} weight="semibold">Rust RSS Reader</Text>
                <Text size={200} style={{ color: tokens.colorNeutralForeground3, letterSpacing: "0.3em" }}>RRR</Text>
                <Text size={300} className={styles.aboutDesc}>{t("settings.aboutDesc")}</Text>
                <div className={styles.aboutMeta}>
                  <Text size={300} className={styles.desc}>
                    {t("settings.aboutVersion")}: {info?.version ?? "…"} · {t("settings.aboutEngine")}: Tauri {info?.tauri_version ?? "…"}
                  </Text>
                  <Text size={300} className={styles.desc}>
                    {t("settings.aboutLicense")}: {info?.license ?? "…"}
                  </Text>
                  <Text size={300} className={styles.desc}>
                    {t("settings.aboutHomepage")}:{" "}
                    <a href="https://github.com/yang991178/fluent-reader" target="_blank" rel="noreferrer">yang991178/fluent-reader</a>
                  </Text>
                </div>
              </div>
            )}

            {result && (
              <div className={styles.section}>
                <Text size={300} block>{result}</Text>
                {importErrors.length > 0 && (
                  <div style={{ marginTop: 6, maxHeight: 140, overflowY: "auto" }}>
                    {importErrors.slice(0, 8).map((e, i) => (
                      <Text key={i} size={200} block style={{ color: tokens.colorStatusDangerForeground1 }}>
                        · {e}
                      </Text>
                    ))}
                    {importErrors.length > 8 && (
                      <Text size={200} block style={{ color: tokens.colorNeutralForeground3 }}>
                        … 共 {importErrors.length} 条失败
                      </Text>
                    )}
                   </div>
                )}
              </div>
            )}
            </div>
          </DialogContent>
          <DialogActions>
            <Button appearance="primary" onClick={onClose}>{t("dialog.close")}</Button>
          </DialogActions>
        </DialogBody>
      </DialogSurface>
    </Dialog>
  );
}
