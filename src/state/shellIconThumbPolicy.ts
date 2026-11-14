// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 图标视图内容缩略图策略（见 docs/shell-icon-thumbnail-loading.md）。
 * 目录与公共壳图标类型走快通道；仅图像/视频等走慢速内容缩略图。
 */

/** 图像类：系统缩略图有独立画面。 */
const IMAGE_THUMB_EXTS = new Set([
  "png",
  "jpg",
  "jpeg",
  "jpe",
  "jfif",
  "gif",
  "bmp",
  "dib",
  "webp",
  "ico",
  "tif",
  "tiff",
  "heic",
  "heif",
  "avif",
  "svg",
]);

/** 视频类：依赖 Shell/第三方缩略图器，最慢，绝不能挡壳图标。 */
const VIDEO_THUMB_EXTS = new Set([
  "mp4",
  "m4v",
  "mkv",
  "webm",
  "avi",
  "mov",
  "wmv",
  "flv",
  "mpeg",
  "mpg",
  "mpe",
  "ts",
  "m2ts",
  "3gp",
  "3g2",
]);

/**
 * 超过该滚动速度（px/ms）视为甩动，挂起内容缩略图排队。
 * 1.25 是内存安全阀，不可调高（2026-11-14 事故：曾试 4.0 以"对齐
 * Explorer 滚动中加载"，结果 1.25–4.0 的常规滚动速度带内每帧窗口推进
 * 都真实走 IPC→base64→解码全链路，形成每秒数十张的分配洪峰，GC 追不上，
 * renderer 瞬间吃满系统内存至 9GB+）。Explorer 走 OS 级缩略图缓存与独立
 * 进程，本应用在 renderer GC 堆内全链路解码，二者不可比。
 * 滚动中的体验由停稳 CONTENT_THUMB_SETTLE_MS 后整窗补齐保证；
 * 窗口对得准（几何实测）比滚动中流式加载更重要。
 */
export const CONTENT_THUMB_FLING_PX_PER_MS = 1.25;
/** 滚动停稳后再补内容缩略图的等待时间。 */
export const CONTENT_THUMB_SETTLE_MS = 140;

function fileExtensionLower(path: string): string {
  const base = path.replace(/[/\\]+$/, "").split(/[/\\]/).pop() ?? "";
  const dot = base.lastIndexOf(".");
  if (dot <= 0 || dot === base.length - 1) return "";
  return base.slice(dot + 1).toLowerCase();
}

/**
 * 是否应为该条目请求内容缩略图（慢路径）。
 * 目录、无扩展名、以及仅有公共壳图标的类型返回 false。
 */
export function entryWantsContentThumbnail(
  kind: string | undefined,
  path: string,
): boolean {
  if (kind === "dir") return false;
  const ext = fileExtensionLower(path);
  if (!ext) return false;
  return IMAGE_THUMB_EXTS.has(ext) || VIDEO_THUMB_EXTS.has(ext);
}
