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
 * 4.0：普通滚轮单格（~100px/40ms ≈ 2.5px/ms）不再触发挂起，滚动中照常
 * 加载（对齐 Windows Explorer）；仅触摸板/拖滚动条的快速甩动才挂起，
 * 停稳 CONTENT_THUMB_SETTLE_MS 后重排整窗。旧值 1.25 会让常规滚动清空
 * 排队，只剩 in-flight 一批（约 1-2 行），表现为"只加载一两行"。
 */
export const CONTENT_THUMB_FLING_PX_PER_MS = 4.0;
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
