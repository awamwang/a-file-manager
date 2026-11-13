// SPDX-License-Identifier: GPL-3.0-or-later
/** DnD 概念共址：模式 / 路径解析 / hover / 落入表面 / Hybrid 向外拖出（Explorer OLE 互通）。 */

export { dropTransferMode, type DropMode } from "./mode.ts";
export {
  dataTransferLooksExternal,
  fileUrlToPath,
  pathToFileUrl,
  pathsFromDataTransfer,
  setOutboundPathPayload,
} from "./paths.ts";
export { createDropHoverScheduler, DROP_HOVER_MS } from "./hover.ts";
export {
  acceptFolderDragOver,
  commitFolderDrop,
  noticeKeyForDropMode,
  previewDropMode,
  previewSourcePaths,
  type FolderDropCommit,
} from "./surface.ts";
export {
  armNativeFileDragOut,
  armOutboundPathsForPress,
  clearOutboundPathsForPress,
  startNativeFileDragOut,
} from "./dragOut.ts";
