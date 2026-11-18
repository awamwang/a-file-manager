// SPDX-License-Identifier: GPL-3.0-or-later
/** 统一搜索面板（Search Palette）公共契约：搜索源、结果行与范围组合。 */

import type { FluentGlyphId } from "../components/icons/glyphs.ts";

/** 搜索源 id（持久化范围组合即存这些 id）。 */
export type SearchSourceId =
  | "commands"
  | "files"
  | "projects"
  | "workgroups"
  | "tools"
  | "bookmarks"
  | "extensions"
  | "knowledge";

/** 范围 chip / Alt+数字顺序：命令 → 文件 → 项目 → 工作组 → 工具 → 书签 → 知识内容 → 扩展。 */
export const SEARCH_SOURCE_ORDER: readonly SearchSourceId[] = [
  "commands",
  "files",
  "projects",
  "workgroups",
  "tools",
  "bookmarks",
  "knowledge",
  "extensions",
];

/** 结果分组显示顺序：知识内容组排在文件组之前，文件组排最后（ADR 0016）。 */
export const SEARCH_RESULT_GROUP_ORDER: readonly SearchSourceId[] = [
  "commands",
  "projects",
  "workgroups",
  "tools",
  "bookmarks",
  "extensions",
  "knowledge",
  "files",
];

/** 每组结果上限。 */
export const SEARCH_GROUP_LIMIT = 8;

/** 行尾类型标签：命令 / 文件 / 文件夹 / 项目 / 工作组 / 工具 / 能力 / 书签 / 扩展。 */
export type SearchItemType =
  | "command"
  | "file"
  | "folder"
  | "project"
  | "workgroup"
  | "tool"
  | "capability"
  | "bookmark"
  | "extension";

export type SearchActivateOptions = {
  /** Ctrl+Enter（按源定义的备用动作，如文件在资源管理器定位）。 */
  ctrl: boolean;
};

/** Enter 后由面板转成 toast 的提示（如工作组 REQUIRE_SAVE_PROMPT）。 */
export type SearchActivateNotice = {
  toastKey: string;
  kind: "info" | "error";
};

export type SearchActivateOutcome = SearchActivateNotice | void;

export type SearchResultItem = {
  /** 稳定 key：sourceId + 条目 id，供列表渲染。 */
  key: string;
  sourceId: SearchSourceId;
  type: SearchItemType;
  title: string;
  /** 副标题：字面量或 i18n key（二选一）。 */
  subtitle?: string;
  subtitleKey?: string;
  /** 副标题 i18n 参数。 */
  subtitleParams?: Record<string, string | number>;
  /** 行首图标覆盖（缺省按 type 映射）。 */
  icon?: FluentGlyphId;
  /** 壳图标路径（文件/文件夹行：优先显示系统图标）。 */
  iconPath?: string;
  /** 命令行的分组（Navigate/View/Actions），供 commandGroupIcon。 */
  group?: string;
  activate(options: SearchActivateOptions):
    | Promise<SearchActivateOutcome>
    | SearchActivateOutcome;
};

export type SearchSourceStatus = "ready" | "unavailable";

export type SearchSourceResult = {
  status: SearchSourceStatus;
  items: SearchResultItem[];
  /** 源级提示 i18n key（如书签快照过期只读）。 */
  hintKey?: string;
};

/** 检索选项：`fuzzy` = 面板底部「模糊搜索」勾选（默认关，本地过滤源切子序列匹配）。 */
export type SearchQueryOptions = {
  fuzzy: boolean;
};

/**
 * 组尾「查看全部」入口：
 * - 默认带当前查询跳转到该源的完整结果页（`action`）；
 * - `overlay === true` 时改由搜索面板在内部展开浮层，调用源的 `searchAll` 分页取全量结果，
 *   不再跳转（`action` 保留作深链兜底）。
 */
export type SearchViewAll = {
  labelKey: string;
  action(query: string): void | Promise<void>;
  /** 在面板内展开浮层展示全部结果（仅文件源启用）。 */
  overlay?: boolean;
};

/** 「查看全部」浮层分页结果：完整命中列表的一页 + 是否还有更多。 */
export type SearchAllPage = {
  items: SearchResultItem[];
  totalCount: number;
  offset: number;
  hasMore: boolean;
};

export type SearchSource = {
  id: SearchSourceId;
  /** 范围 chip / 分组标题 i18n key。 */
  labelKey: string;
  /** 会话内是否参与（如 Pack 未启用时源与 chip 一起隐藏）。 */
  enabled(): boolean;
  /** 源不可用时的状态行文案 i18n key。 */
  unavailableKey: string;
  /** 状态行引导动作（如「打开项目页」/「刷新书签」）。 */
  unavailableActionLabelKey?: string;
  unavailableAction?(): void | Promise<void>;
  /** 状态行动作后保留面板打开并重跑检索（如刷新快照）。 */
  retainPanelOnStatusAction?: boolean;
  /** 组尾「查看全部」入口（如文件源跳 Discovery 带查询，或面板内展开浮层）。 */
  viewAll?: SearchViewAll;
  /**
   * 「查看全部」浮层（`viewAll.overlay` 为真时）按偏移分页取该源的全量结果；
   * 返回条目的 `activate` 行为须与面板内点击一致（沿用搜索面板行为）。
   */
  searchAll?(
    query: string,
    options: SearchQueryOptions,
    offset: number,
    limit: number,
  ): Promise<SearchAllPage>;
  /**
   * 检索：空 query 时命令源返回全量、其余源返回空列表；
   * 失败时自行捕获并返回 `status: "unavailable"`。
   * `options.fuzzy` 开启时本地过滤的源改用子序列模糊匹配。
   */
  search(
    query: string,
    options?: SearchQueryOptions,
  ): SearchSourceResult | Promise<SearchSourceResult>;
};
