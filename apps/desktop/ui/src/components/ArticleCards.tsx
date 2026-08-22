import React from "react";
import { Text, makeStyles, tokens } from "@fluentui/react-components";
import { StarFilled, RssRegular } from "@fluentui/react-icons";

import { Article } from "../api";

const useStyles = makeStyles({
  // ---- card 视图 ----
  card: {
    border: `1px solid ${tokens.colorNeutralStroke2}`,
    borderRadius: "6px",
    overflow: "hidden",
    cursor: "pointer",
    background: tokens.colorNeutralBackground1,
    "&:hover": { border: `1px solid ${tokens.colorBrandStroke1}` },
  },
  cardSelected: {
    border: `1px solid ${tokens.colorBrandStroke1}`,
    boxShadow: `0 0 0 1px ${tokens.colorBrandStroke1}`,
  },
  cardCover: {
    width: "100%",
    aspectRatio: "3 / 2",
    objectFit: "cover",
    display: "block",
  },
  cardCoverPlaceholder: {
    width: "100%",
    aspectRatio: "3 / 2",
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    background: `linear-gradient(135deg, ${tokens.colorBrandBackground2}, ${tokens.colorNeutralBackground3})`,
    color: tokens.colorNeutralForeground3,
  },
  cardBody: { padding: "8px" },

  // ---- 杂志编辑精选式 ----
  magazineGrid: {
    display: "grid",
    gridTemplateColumns: "repeat(4, 1fr)",
    gridAutoRows: "auto",
    gap: "12px",
    padding: "12px",
    alignItems: "start",
  },
  magazineCard: {
    borderRadius: "12px",
    overflow: "hidden",
    cursor: "pointer",
    background: tokens.colorNeutralBackground1,
    boxShadow: "0 1px 4px rgba(0,0,0,0.10)",
    transition: "transform 0.15s ease, box-shadow 0.15s ease",
    "&:hover": {
      transform: "translateY(-2px)",
      boxShadow: "0 6px 18px rgba(0,0,0,0.16)",
    },
  },
  magazineSelected: {
    boxShadow: "0 0 0 2px rgba(0,120,212,0.7)",
  },
  magazineCover: {
    width: "100%",
    objectFit: "cover",
    display: "block",
  },
  magazinePlaceholder: {
    width: "100%",
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    background: `linear-gradient(135deg, ${tokens.colorBrandBackground2}, ${tokens.colorNeutralBackground3})`,
    color: tokens.colorNeutralForeground3,
  },
  magazineBody: {
    padding: "12px 14px 14px",
    display: "flex",
    flexDirection: "column",
    gap: "6px",
  },
  magazineFeaturedBody: {
    padding: "18px 22px 20px",
    display: "flex",
    flexDirection: "column",
    gap: "8px",
  },
  magazineSnippet: {
    color: tokens.colorNeutralForeground3,
    overflow: "hidden",
    textOverflow: "ellipsis",
    display: "-webkit-box",
    WebkitLineClamp: 2,
    WebkitBoxOrient: "vertical",
  },
  magazineMeta: {
    color: tokens.colorNeutralForeground3,
    overflow: "hidden",
    textOverflow: "ellipsis",
    whiteSpace: "nowrap",
  },
  stripBody: {
    padding: "10px 14px",
    display: "flex",
    flexDirection: "column",
    gap: "5px",
    justifyContent: "center",
  },

  // ---- compact 视图 ----
  compactItem: {
    display: "flex",
    alignItems: "center",
    gap: "8px",
    padding: "10px 14px",
    borderRadius: "6px",
    cursor: "pointer",
    "&:hover": { backgroundColor: tokens.colorNeutralBackground1Hover },
  },
  compactSelected: { backgroundColor: tokens.colorBrandBackground2 },
  compactTitle: { flex: 1, minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" },

  // ---- list 视图 ----
  listItem: {
    display: "flex",
    flexDirection: "column",
    gap: "2px",
    padding: "8px 12px",
    borderRadius: "6px",
    cursor: "pointer",
    "&:hover": { backgroundColor: tokens.colorNeutralBackground1Hover },
  },
  listItemSelected: { backgroundColor: tokens.colorBrandBackground2 },
  listItemTitle: {
    display: "flex",
    alignItems: "center",
    gap: "6px",
  },
});

function firstImage(html: string | null): string | null {
  if (!html) return null;
  const m = html.match(/<img[^>]+src=["']([^"']+)["']/i);
  return m ? m[1] : null;
}

export interface ArticleCardsProps {
  article: Article;
  idx: number;
  viewMode: string;
  cardSize: "small" | "medium" | "large";
  selected: boolean;
  imgMap: Record<string, string>;
  feedName: (feedId: number) => string;
  onSelect: (a: Article) => void;
  onImgError: (url: string | null) => void;
}

/** 单篇文章的卡片渲染（list / compact / card / magazine 四视图）。 */
export function ArticleCards({
  article: a,
  idx,
  viewMode,
  cardSize,
  selected,
  imgMap,
  feedName,
  onSelect,
  onImgError,
}: ArticleCardsProps) {
  const styles = useStyles();
  const feedN = feedName(a.feed_id);
  const date = a.published_at ? new Date(a.published_at).toLocaleDateString() : "";

  if (viewMode === "compact") {
    return (
      <div
        key={a.id}
        data-idx={idx}
        className={`${styles.compactItem} ${selected ? styles.compactSelected : ""}`}
        onClick={() => onSelect(a)}
      >
        {a.is_starred && <StarFilled style={{ color: tokens.colorBrandForeground1, width: 12 }} />}
        <Text size={300} weight={a.is_read ? "regular" : "semibold"} className={styles.compactTitle} truncate>
          {a.title || "(untitled)"}
        </Text>
        <Text size={200} style={{ color: tokens.colorNeutralForeground3 }} truncate>
          {feedN}
        </Text>
        <Text size={200} style={{ color: tokens.colorNeutralForeground3 }}>
          {date}
        </Text>
      </div>
    );
  }

  if (viewMode === "card") {
    const imgRaw = firstImage(a.content || a.summary);
    const img = imgRaw ? (imgMap[imgRaw] || imgRaw) : null;
    return (
      <div
        key={a.id}
        data-idx={idx}
        className={`${styles.card} ${selected ? styles.cardSelected : ""}`}
        onClick={() => onSelect(a)}
      >
        {img ? (
          <img src={img} alt="" className={styles.cardCover} loading="lazy" onError={() => onImgError(imgRaw)} />
        ) : (
          <div className={styles.cardCoverPlaceholder}>
            <RssRegular style={{ width: 22, height: 22 }} />
          </div>
        )}
        <div className={styles.cardBody}>
          <Text size={300} weight={a.is_read ? "regular" : "semibold"} block truncate>
            {a.title || "(untitled)"}
          </Text>
          <Text size={200} style={{ color: tokens.colorNeutralForeground3 }} block truncate>
            {feedN} · {date}
          </Text>
        </div>
      </div>
    );
  }

  if (viewMode === "magazine") {
    const imgRaw = firstImage(a.content || a.summary);
    const img = imgRaw ? (imgMap[imgRaw] || imgRaw) : null;
    // 编辑精选式（简洁版）：idx%7 循环 = 通栏大卡 + 6 张等大横卡（4 列网格，无横条）
    const featured = idx % 7 === 0;
    const span = featured ? 4 : 1;
    const coverH = featured ? 300 : undefined;
    const aspect = featured ? "16 / 9" : "16 / 10";
    const cover = img ? (
      <img
        src={img}
        alt=""
        className={styles.magazineCover}
        style={{ height: coverH, aspectRatio: aspect }}
        loading="lazy"
        onError={() => onImgError(imgRaw)}
      />
    ) : (
      <div
        className={styles.magazinePlaceholder}
        style={{ height: coverH, aspectRatio: aspect }}
      >
        <RssRegular style={{ width: featured ? 34 : 26, height: featured ? 34 : 26 }} />
      </div>
    );
    const snippet = (a.content || a.summary || "")
      .replace(/<[^>]+>/g, "")
      .trim()
      .slice(0, 120);
    const body = featured ? (
      <div className={styles.magazineFeaturedBody}>
        <Text size={600} weight={a.is_read ? "regular" : "semibold"}>
          {a.title || "(untitled)"}
        </Text>
        {snippet && (
          <Text size={300} className={styles.magazineSnippet}>
            {snippet}
          </Text>
        )}
        <Text size={300} className={styles.magazineMeta}>
          {feedN} · {date}
        </Text>
      </div>
    ) : (
      <div className={styles.magazineBody}>
        <Text size={500} weight={a.is_read ? "regular" : "semibold"}>
          {a.title || "(untitled)"}
        </Text>
        <Text size={300} className={styles.magazineMeta}>
          {feedN} · {date}
        </Text>
      </div>
    );
    return (
      <div
        key={a.id}
        data-idx={idx}
        className={`${styles.magazineCard} ${selected ? styles.magazineSelected : ""}`}
        style={{ gridColumn: `span ${span}` }}
        onClick={() => onSelect(a)}
      >
        {cover}
        {body}
      </div>
    );
  }

  // list 视图（默认）
  return (
    <div
      key={a.id}
      data-idx={idx}
      className={`${styles.listItem} ${selected ? styles.listItemSelected : ""}`}
      onClick={() => onSelect(a)}
    >
      <div className={styles.listItemTitle}>
        {a.is_starred && <StarFilled style={{ color: tokens.colorBrandForeground1, width: 12 }} />}
        <Text size={300} weight={a.is_read ? "regular" : "semibold"} truncate>
          {a.title || "(untitled)"}
        </Text>
      </div>
      <Text size={200} style={{ color: tokens.colorNeutralForeground3 }}>
        {feedN} · {date}
      </Text>
    </div>
  );
}
