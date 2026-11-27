// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Navigator type filter — filter the visible rows by file type.
 *
 * Two-level model (like modern store filters):
 *  - **Major categories** — folder / image / video / audio / document /
 *    archive / code / other. A fixed registry maps extensions to categories.
 *  - **Specific types** — the extensions actually present in the pane's rows,
 *    collected dynamically per category (facets). Selecting a specific type
 *    refines its category; the category must be selected for any of its
 *    types to apply.
 *
 * The selection is session state (not persisted): it is view state, not a
 * preference, and it deliberately survives navigation so it keeps filtering
 * while the user walks a drill-through tree.
 *
 * Pure helpers are exported for the gate (`navigator-type-filter-checks.ts`).
 */

import { computed, ref, shallowRef } from "vue";
import { entryExtension } from "./entryType.ts";
import type { NavigatorEntry } from "./navigatorBrowse.ts";

/** Minimal row shape the pure logic needs (real and mock entries satisfy it). */
export type TypeFilterEntryLike = Pick<NavigatorEntry, "kind" | "name">;

export type TypeFilterCategoryKey =
  | "folder"
  | "image"
  | "video"
  | "audio"
  | "document"
  | "archive"
  | "code"
  | "other";

/** Display order of the major categories (folders first, catch-all last). */
export const TYPE_FILTER_CATEGORY_ORDER: readonly TypeFilterCategoryKey[] = [
  "folder",
  "image",
  "video",
  "audio",
  "document",
  "archive",
  "code",
  "other",
];

function categorySet(...exts: string[]): Set<string> {
  return new Set(exts);
}

/** Extension registry per category ("folder" / "other" are structural). */
const CATEGORY_EXTENSIONS: Record<
  Exclude<TypeFilterCategoryKey, "folder" | "other">,
  Set<string>
> = {
  image: categorySet(
    "png", "jpg", "jpeg", "gif", "bmp", "webp", "svg", "ico", "tif", "tiff",
    "heic", "avif", "raw", "cr2", "nef",
  ),
  video: categorySet(
    "mp4", "mkv", "avi", "mov", "wmv", "flv", "webm", "m4v", "mpg", "mpeg",
    "3gp",
  ),
  audio: categorySet(
    "mp3", "wav", "flac", "aac", "ogg", "m4a", "wma", "aiff", "opus", "mid",
    "midi",
  ),
  document: categorySet(
    "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "txt", "md", "rtf",
    "odt", "ods", "odp", "csv", "epub",
  ),
  archive: categorySet(
    "zip", "rar", "7z", "tar", "gz", "bz2", "xz", "tgz", "iso", "cab",
  ),
  code: categorySet(
    "js", "mjs", "cjs", "ts", "tsx", "jsx", "py", "java", "c", "h", "cpp",
    "hpp", "cs", "go", "rs", "rb", "php", "sh", "bat", "ps1", "html", "css",
    "json", "xml", "yml", "yaml", "toml", "sql", "lua", "kt", "swift", "vue",
  ),
};

/** Category for an extension; everything unregistered lands in "other". */
export function typeCategoryForExtension(ext: string): TypeFilterCategoryKey {
  const key = ext.trim().toLowerCase();
  for (const category of TYPE_FILTER_CATEGORY_ORDER) {
    if (category === "folder" || category === "other") continue;
    if (CATEGORY_EXTENSIONS[category].has(key)) return category;
  }
  return "other";
}

/** Category of a row: folders are their own category, files go by extension. */
export function entryTypeCategory(
  kind: string,
  name: string,
): TypeFilterCategoryKey {
  if (kind === "dir") return "folder";
  return typeCategoryForExtension(entryExtension(name));
}

/* —— Facets: what the current rows actually contain (dynamic) —— */

export type TypeFacetExtension = { ext: string; count: number };
export type TypeFacet = {
  key: TypeFilterCategoryKey;
  count: number;
  /** Extensions present in the rows, most common first. */
  extensions: TypeFacetExtension[];
};

/**
 * Collect the type facets of `entries`: only categories that are actually
 * present, and within each only the extensions actually present — the
 * specific-type level is always derived from the current rows.
 */
export function collectTypeFacets(
  entries: readonly TypeFilterEntryLike[],
): TypeFacet[] {
  const counts = new Map<TypeFilterCategoryKey, number>();
  const extCounts = new Map<string, number>();
  for (const entry of entries) {
    const category = entryTypeCategory(entry.kind, entry.name);
    counts.set(category, (counts.get(category) ?? 0) + 1);
    if (entry.kind !== "dir") {
      const ext = entryExtension(entry.name);
      if (ext) extCounts.set(ext, (extCounts.get(ext) ?? 0) + 1);
    }
  }
  const facets: TypeFacet[] = [];
  for (const key of TYPE_FILTER_CATEGORY_ORDER) {
    const count = counts.get(key);
    if (!count) continue;
    const extensions = [...extCounts.entries()]
      .filter(([ext]) => typeCategoryForExtension(ext) === key)
      .map(([ext, extCount]) => ({ ext, count: extCount }))
      .sort((a, b) => b.count - a.count || a.ext.localeCompare(b.ext));
    facets.push({ key, count, extensions });
  }
  return facets;
}

/* —— Selection semantics —— */

/** A selection filters only when at least one category is picked. */
export function isTypeFilterActive(
  categories: readonly string[],
  _extensions: readonly string[],
): boolean {
  return categories.length > 0;
}

/**
 * Whether one row survives the selection.
 *
 * - filter off → everything matches.
 * - folders match only when the "folder" category is picked.
 * - a file matches when its category is picked AND (no specific type of that
 *   category is picked, or its own extension is one of them).
 */
export function entryMatchesTypeFilter(
  entry: TypeFilterEntryLike,
  categories: readonly string[],
  extensions: readonly string[],
): boolean {
  if (!isTypeFilterActive(categories, extensions)) return true;
  if (entry.kind === "dir") return categories.includes("folder");
  const category = entryTypeCategory(entry.kind, entry.name);
  if (!categories.includes(category)) return false;
  const picked = extensions.filter(
    (ext) => typeCategoryForExtension(ext) === category,
  );
  if (picked.length === 0) return true;
  return picked.includes(entryExtension(entry.name));
}

/** Convenience: filter a row array with the given selection. */
export function filterEntriesByType<T extends TypeFilterEntryLike>(
  entries: readonly T[],
  categories: readonly string[],
  extensions: readonly string[],
): T[] {
  if (!isTypeFilterActive(categories, extensions)) return [...entries];
  return entries.filter((entry) =>
    entryMatchesTypeFilter(entry, categories, extensions),
  );
}

function orderedCategories(keys: readonly string[]): string[] {
  return TYPE_FILTER_CATEGORY_ORDER.filter((key) => keys.includes(key));
}

/**
 * Selection invariant: extensions only refine selected categories. Dropping a
 * category silently drops its refinements; duplicates collapse in first-seen
 * order. Pure — the actions run it after every change.
 */
export function normalizeTypeFilterSelection(
  categories: readonly string[],
  extensions: readonly string[],
): { categories: string[]; extensions: string[] } {
  const seenCategories = new Set<string>();
  for (const key of categories) {
    if ((TYPE_FILTER_CATEGORY_ORDER as readonly string[]).includes(key)) {
      seenCategories.add(key);
    }
  }
  const seenExtensions = new Set<string>();
  for (const ext of extensions) {
    const key = ext.trim().toLowerCase();
    if (!key || seenExtensions.has(key)) continue;
    if (!seenCategories.has(typeCategoryForExtension(key))) continue;
    seenExtensions.add(key);
  }
  return {
    categories: orderedCategories([...seenCategories]),
    extensions: [...seenExtensions],
  };
}

/* —— Reactive session state —— */

export const typeFilterCategories = ref<string[]>([]);
export const typeFilterExtensions = ref<string[]>([]);
export const typeFilterDialogOpen = ref(false);

/**
 * The rows the dialog's facets describe — the primary pane's unfiltered row
 * source (search hits / drill rows / folder listing). NavigatorView keeps
 * this in sync; the dialog only reads it.
 */
export const typeFilterFacetEntries = shallowRef<readonly TypeFilterEntryLike[]>(
  [],
);

export const typeFilterActive = computed(() =>
  isTypeFilterActive(typeFilterCategories.value, typeFilterExtensions.value),
);

/** Replace the facet source (called by NavigatorView on row-source change). */
export function setTypeFilterFacetEntries(
  entries: readonly TypeFilterEntryLike[],
): void {
  typeFilterFacetEntries.value = entries;
}

export function toggleTypeFilterCategory(key: string): void {
  const categories = new Set(typeFilterCategories.value);
  if (categories.has(key)) {
    categories.delete(key);
  } else {
    categories.add(key);
  }
  const next = normalizeTypeFilterSelection(
    [...categories],
    typeFilterExtensions.value,
  );
  typeFilterCategories.value = next.categories;
  typeFilterExtensions.value = next.extensions;
}

export function toggleTypeFilterExtension(ext: string): void {
  const key = ext.trim().toLowerCase();
  if (!key) return;
  const extensions = new Set(typeFilterExtensions.value);
  const categories = new Set(typeFilterCategories.value);
  if (extensions.has(key)) {
    extensions.delete(key);
  } else {
    extensions.add(key);
    // A specific type implies its category (auto-check the parent) — the
    // invariant below would otherwise drop the orphan refinement.
    categories.add(typeCategoryForExtension(key));
  }
  const next = normalizeTypeFilterSelection(
    [...categories],
    [...extensions],
  );
  typeFilterCategories.value = next.categories;
  typeFilterExtensions.value = next.extensions;
}

export function clearTypeFilter(): void {
  typeFilterCategories.value = [];
  typeFilterExtensions.value = [];
}

export function closeTypeFilterDialog(): void {
  typeFilterDialogOpen.value = false;
}

export function toggleTypeFilterDialog(): void {
  typeFilterDialogOpen.value = !typeFilterDialogOpen.value;
}
