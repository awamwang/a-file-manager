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
  | "extensions";

/** 固定分组顺序：命令 → 文件 → 项目 → 工作组 → 工具 → 扩展。 */
export const SEARCH_SOURCE_ORDER: readonly SearchSourceId[] = [
  "commands",
  "files",
  "projects",
  "workgroups",
  "tools",
  "extensions",
];

/** 每组结果上限。 */
export const SEARCH_GROUP_LIMIT = 8;

/** 行尾类型标签：命令 / 文件 / 文件夹 / 项目 / 工作组 / 工具 / 能力 / 扩展。 */
export type SearchItemType =
  | "command"
  | "file"
  | "folder"
  | "project"
  | "workgroup"
  | "tool"
  | "capability"
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
};

/** 组尾「查看全部」入口：带当前查询跳转到该源的完整结果页。 */
export type SearchViewAll = {
  labelKey: string;
  action(query: string): void | Promise<void>;
};

export type SearchSource = {
  id: SearchSourceId;
  /** 范围 chip / 分组标题 i18n key。 */
  labelKey: string;
  /** 会话内是否参与（如 Pack 未启用时源与 chip 一起隐藏）。 */
  enabled(): boolean;
  /** 源不可用时的状态行文案 i18n key。 */
  unavailableKey: string;
  /** 状态行引导动作（如「打开项目页」）。 */
  unavailableActionLabelKey?: string;
  unavailableAction?(): void | Promise<void>;
  /** 组尾「查看全部」入口（如文件源跳 Discovery 带查询）。 */
  viewAll?: SearchViewAll;
  /**
   * 检索：空 query 时命令源返回全量、其余源返回空列表；
   * 失败时自行捕获并返回 `status: "unavailable"`。
   */
  search(query: string): SearchSourceResult | Promise<SearchSourceResult>;
};
