import React from "react";
import {
  Button,
  Dialog,
  DialogSurface,
  DialogBody,
} from "@fluentui/react-components";
import { DismissRegular, ArrowMinimizeRegular } from "@fluentui/react-icons";
import { tokens } from "@fluentui/react-components";

function hostOf(url: string): string {
  try {
    const u = new URL(url);
    const parts = u.hostname.split(".");
    if (parts.length >= 2) return parts.slice(-2).join(".");
    return u.hostname;
  } catch {
    return url;
  }
}

function isPortraitVideo(url: string): boolean {
  return /douyin|kuaishou|xiaohongshu|weishipin|weixin\.qq\.com\/sph/i.test(url);
}

function isMusicEmbed(url: string): boolean {
  return /music\.apple\.com|spotify|soundcloud|music\.163\.com|163\.cn|xiaoyuzhoufm|podcasts\.apple|podbean|player\.fm/i.test(
    url,
  );
}

/** 音乐 embed 的合理播放器高度（按平台原生 embed 高度，避免黑边）。 */
function musicHeight(url: string): number {
  if (/spotify/i.test(url)) return 380;
  if (/soundcloud/i.test(url)) return 166;
  if (/xiaoyuzhoufm|podcasts\.apple/i.test(url)) return 460;
  if (/music\.163\.com|163\.cn/i.test(url)) return 450;
  return 450; // Apple Music embed 自带 height:450
}

interface MediaDialogProps {
  open: boolean;
  url: string | null;
  onClose: () => void;
  /** 最小化到后台（音乐）：关闭弹窗并开独立小窗继续播放。 */
  onMinimize?: (url: string) => void;
}

/**
 * 媒体播放弹窗：尺寸贴合内容，不铺全屏。
 * - 视频（YouTube/B站/腾讯/抖音等）：16:9 或竖屏 9:16 保形，占屏但留边
 * - 音乐（Apple/Spotify/网易云/播客）：按平台原生高度自适应，居中；可最小化到后台独立小窗
 */
export default function MediaDialog({ open, url, onClose, onMinimize }: MediaDialogProps) {
  const music = !!url && isMusicEmbed(url);
  const portrait = !!url && !music && isPortraitVideo(url);

  const vw = typeof window !== "undefined" ? window.innerWidth : 1280;
  const vh = typeof window !== "undefined" ? window.innerHeight : 800;
  // 全部用像素计算，避免 Fluent Dialog 的百分比高度链失效导致 iframe 塌陷
  const musicH = url ? musicHeight(url) : 480;
  const playerW = music
    ? Math.min(640, vw * 0.92)
    : portrait
      ? Math.min(vh * 0.52, vw * 0.44)
      : Math.min(vw * 0.94, vh * 0.82 * (16 / 9));
  const playerH = music ? musicH : Math.round(playerW * (portrait ? 16 / 9 : 9 / 16));
  const topBarH = 33;

  return (
    <Dialog open={open} onOpenChange={(_e, d) => !d.open && onClose()}>
      {url ? (
        <DialogSurface
          style={{
            width: Math.round(playerW),
            height: playerH + topBarH,
            maxWidth: "none",
            padding: 0,
            overflow: "hidden",
            borderRadius: 14,
          }}
        >
          <DialogBody style={{ padding: 0 }}>
            <div style={{ position: "relative", width: Math.round(playerW), height: playerH + topBarH }}>
              <div
                style={{
                  position: "absolute",
                  top: 0,
                  left: 0,
                  right: 0,
                  height: topBarH,
                  display: "flex",
                  alignItems: "center",
                  gap: 8,
                  padding: "0 8px 0 12px",
                  background: "rgba(255,255,255,0.06)",
                  backdropFilter: "blur(6px)",
                }}
              >
                <span
                  style={{
                    color: tokens.colorNeutralForeground1,
                    fontSize: 12,
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                    flex: 1,
                  }}
                >
                  ▶ {hostOf(url)}
                </span>
                {music && onMinimize && (
                  <Button
                    appearance="subtle"
                    size="small"
                    icon={<ArrowMinimizeRegular />}
                    title="最小化到后台播放"
                    onClick={() => onMinimize(url)}
                  />
                )}
                <Button
                  appearance="subtle"
                  size="small"
                  icon={<DismissRegular />}
                  onClick={onClose}
                />
              </div>
              <iframe
                sandbox="allow-scripts allow-same-origin allow-forms allow-popups allow-presentation"
                src={url}
                title="media"
                style={{
                  position: "absolute",
                  top: topBarH,
                  left: 0,
                  width: Math.round(playerW),
                  height: playerH,
                  border: "none",
                  background: "#000",
                }}
              />
            </div>
          </DialogBody>
        </DialogSurface>
      ) : (
        <DialogSurface style={{ maxWidth: 360, width: "90vw" }}>
          <DialogBody>
            <p
              style={{
                margin: 0,
                color: tokens.colorNeutralForeground3,
                fontSize: 13,
              }}
            >
              无法加载该媒体链接。
            </p>
          </DialogBody>
        </DialogSurface>
      )}
    </Dialog>
  );
}
