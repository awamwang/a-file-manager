// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Explorer-like grouping for Navigator listings.
 * v17（ADR 0030）：布尔「按类型分组」泛化为 field × direction 分组
 * （name / size / modified / type），文件夹永远单独成组置顶。
 */

import { entryTypeKey, entryTypeLabel } from "./entryType.ts";

export type NavigatorGroupField = "none" | "name" | "size" | "modified" | "type";
export type GroupDirection = "asc" | "desc";

export type GroupableEntry = {
  name: string;
  path: string;
  kind: string;
  size?: number;
  modifiedMs?: number | null;
};

export type NavigatorGroup<T extends GroupableEntry = GroupableEntry> = {
  key: string;
  label: string;
  entries: T[];
};

export type GroupLabelFns = {
  folder: string;
  file: string;
  shortcut: string;
  typedFile: (ext: string) => string;
  /** 名称分组：非字母桶（数字/符号/未识别文字）。 */
  nameOther: string;
  sizeEmpty: string;
  sizeSmall: string;
  sizeMedium: string;
  sizeLarge: string;
  sizeHuge: string;
  dateUnknown: string;
  dateToday: string;
  dateYesterday: string;
  dateThisWeek: string;
  dateLastWeek: string;
  dateEarlier: string;
};

/** 文件夹组的固定 key（各 field 通用；始终置顶）。 */
export const FOLDER_GROUP_KEY = "folder";

const SIZE_1MB = 1024 * 1024;
const SIZE_128MB = 128 * SIZE_1MB;
const SIZE_1GB = 1024 * SIZE_1MB;
const DAY_MS = 86_400_000;

/** GB2312 拼音首字母经典边界表（拼音序，zh collation 下单调）。 */
const PINYIN_INITIALS = "ABCDEFGHJKLMNOPQRSTWXYZ";
const PINYIN_BOUND_CHARS = "啊芭擦搭蛾发噶哈击喀垃妈拿哦啪期然撒塌挖希压匝";
const zhCollator = new Intl.Collator("zh");

function asciiLetterBucket(ch: string): string | null {
  if (ch >= "a" && ch <= "z") return ch.toUpperCase();
  if (ch >= "A" && ch <= "Z") return ch;
  return null;
}

/** 汉字 → 拼音首字母（zh collation 二分）；非汉字/扩展区/生僻字返回 null。 */
function pinyinInitial(ch: string): string | null {
  const code = ch.codePointAt(0) ?? 0;
  if (code < 0x4e00 || code > 0x9fff) return null;
  if (zhCollator.compare(ch, PINYIN_BOUND_CHARS.charAt(0)) < 0) return null;
  let lo = 0;
  let hi = PINYIN_BOUND_CHARS.length - 1;
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1;
    if (zhCollator.compare(ch, PINYIN_BOUND_CHARS.charAt(mid)) >= 0) {
      lo = mid;
    } else {
      hi = mid - 1;
    }
  }
  return PINYIN_INITIALS.charAt(lo);
}

/** 名称桶：拉丁字母 → 大写字母；汉字 → 拼音首字母；其余 → "#"。 */
export function nameBucketKey(name: string): string {
  const first = name.trimStart().charAt(0);
  if (!first) return "#";
  return asciiLetterBucket(first) ?? pinyinInitial(first) ?? "#";
}

type SizeBucketKey = "empty" | "small" | "medium" | "large" | "huge";
const SIZE_RANK: Record<SizeBucketKey, number> = {
  empty: 0,
  small: 1,
  medium: 2,
  large: 3,
  huge: 4,
};
const SIZE_LABEL_KEY: Record<SizeBucketKey, string> = {
  empty: "sizeEmpty",
  small: "sizeSmall",
  medium: "sizeMedium",
  large: "sizeLarge",
  huge: "sizeHuge",
};

function sizeBucketKey(size: number): SizeBucketKey {
  if (size <= 0) return "empty";
  if (size < SIZE_1MB) return "small";
  if (size < SIZE_128MB) return "medium";
  if (size < SIZE_1GB) return "large";
  return "huge";
}

type DateBucketKey =
  | "today"
  | "yesterday"
  | "thisWeek"
  | "lastWeek"
  | "earlier"
  | "unknown";
const DATE_RANK: Record<DateBucketKey, number> = {
  earlier: 0,
  lastWeek: 1,
  thisWeek: 2,
  yesterday: 3,
  today: 4,
  unknown: 5,
};
const DATE_LABEL_KEY: Record<DateBucketKey, string> = {
  today: "dateToday",
  yesterday: "dateYesterday",
  thisWeek: "dateThisWeek",
  lastWeek: "dateLastWeek",
  earlier: "dateEarlier",
  unknown: "dateUnknown",
};

function startOfDay(ms: number): number {
  const date = new Date(ms);
  date.setHours(0, 0, 0, 0);
  return date.getTime();
}

/** 周一为一周开始（本地时区）。 */
function startOfWeekMonday(ms: number): number {
  const date = new Date(startOfDay(ms));
  const dow = (date.getDay() + 6) % 7;
  date.setDate(date.getDate() - dow);
  return date.getTime();
}

export function dateBucketKey(modifiedMs: number | null, nowMs: number): DateBucketKey {
  if (modifiedMs == null) return "unknown";
  const today = startOfDay(nowMs);
  if (modifiedMs >= today) return "today";
  if (modifiedMs >= today - DAY_MS) return "yesterday";
  const week = startOfWeekMonday(nowMs);
  if (modifiedMs >= week) return "thisWeek";
  if (modifiedMs >= week - 7 * DAY_MS) return "lastWeek";
  return "earlier";
}

type Bucket<T> = {
  key: string;
  label: string;
  /**越小越靠前（asc）；"#"/unknown 桶永远垫底，不随方向翻转。 */
  rank: number;
  pinnedLast: boolean;
  entries: T[];
};

function labelFor(labels: GroupLabelFns, labelKey: string, name: string): string {
  switch (labelKey) {
    case "nameOther":
      return labels.nameOther;
    case "sizeEmpty":
      return labels.sizeEmpty;
    case "sizeSmall":
      return labels.sizeSmall;
    case "sizeMedium":
      return labels.sizeMedium;
    case "sizeLarge":
      return labels.sizeLarge;
    case "sizeHuge":
      return labels.sizeHuge;
    case "dateToday":
      return labels.dateToday;
    case "dateYesterday":
      return labels.dateYesterday;
    case "dateThisWeek":
      return labels.dateThisWeek;
    case "dateLastWeek":
      return labels.dateLastWeek;
    case "dateEarlier":
      return labels.dateEarlier;
    case "dateUnknown":
      return labels.dateUnknown;
    default:
      return entryTypeLabel("file", name, labels);
  }
}

/**
 * 按字段分桶。文件夹（kind === "dir"）永远单独成组且置顶，
 * 不受 direction 影响；其余桶按 rank 升/降序排，方向只翻转桶间顺序。
 */
export function groupEntriesBy<T extends GroupableEntry>(
  entries: readonly T[],
  field: Exclude<NavigatorGroupField, "none">,
  direction: GroupDirection,
  labels: GroupLabelFns,
  nowMs: number,
): NavigatorGroup<T>[] {
  const dirs: T[] = [];
  const buckets = new Map<string, Bucket<T>>();
  const sign = direction === "asc" ? 1 : -1;

  for (const entry of entries) {
    if (entry.kind === "dir") {
      dirs.push(entry);
      continue;
    }
    let bucket: Bucket<T> | undefined;
    if (field === "type") {
      const key = entryTypeKey(entry.kind, entry.name);
      bucket = buckets.get(key);
      if (!bucket) {
        bucket = {
          key,
          label: entryTypeLabel(entry.kind, entry.name, labels),
          rank: 0,
          pinnedLast: false,
          entries: [],
        };
        buckets.set(key, bucket);
      }
    } else if (field === "name") {
      const key = nameBucketKey(entry.name);
      bucket = buckets.get(key);
      if (!bucket) {
        const isHash = key === "#";
        bucket = {
          key,
          label: isHash ? labels.nameOther : key,
          rank: isHash ? 0 : key.charCodeAt(0) - 64,
          pinnedLast: isHash,
          entries: [],
        };
        buckets.set(key, bucket);
      }
    } else if (field === "size") {
      const sizeKey = sizeBucketKey(entry.size ?? 0);
      bucket = buckets.get(sizeKey);
      if (!bucket) {
        bucket = {
          key: sizeKey,
          label: labelFor(labels, SIZE_LABEL_KEY[sizeKey], entry.name),
          rank: SIZE_RANK[sizeKey],
          pinnedLast: false,
          entries: [],
        };
        buckets.set(sizeKey, bucket);
      }
    } else {
      const dateKey = dateBucketKey(entry.modifiedMs ?? null, nowMs);
      bucket = buckets.get(dateKey);
      if (!bucket) {
        bucket = {
          key: dateKey,
          label: labelFor(labels, DATE_LABEL_KEY[dateKey], entry.name),
          rank: DATE_RANK[dateKey],
          pinnedLast: dateKey === "unknown",
          entries: [],
        };
        buckets.set(dateKey, bucket);
      }
    }
    bucket.entries.push(entry);
  }

  const groups: NavigatorGroup<T>[] = [];
  if (dirs.length) {
    groups.push({ key: FOLDER_GROUP_KEY, label: labels.folder, entries: dirs });
  }
  const ordered = [...buckets.values()].sort((a, b) => {
    if (a.pinnedLast !== b.pinnedLast) return a.pinnedLast ? 1 : -1;
    if (a.rank !== b.rank) return (a.rank - b.rank) * sign;
    return (
      a.label.localeCompare(b.label, undefined, { sensitivity: "base" }) * sign
    );
  });
  for (const bucket of ordered) {
    groups.push({ key: bucket.key, label: bucket.label, entries: bucket.entries });
  }
  return groups;
}
