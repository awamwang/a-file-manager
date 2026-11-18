<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!--
  知识弹窗（ADR 0017，单实例）：搜索命中知识文件行 Enter 打开；
  - Rust `knowledge_service_read_doc` 读取索引根下文档全文（服务端强制路径边界）；
  - markdown 渲染（marked + DOMPurify 消毒）；
  - 文内掩码 token（[REDACTED:label:hash8] / [vault:label:hash8]）→ 私用区占位符 →
    消毒后替换为 chip 槽位，手动挂载 KnowledgeSecretChip（复制 / 5s 短显）。
  - 单实例：重复打开复用同一弹窗并替换内容；z-index 1050——面板(1000)之上、
    解锁确认框(1100)之下，chip 解锁确认不被遮挡。
  明文边界：明文只经 Rust 返回值进 vaultChannel 非响应式 Map（5s 自动回掩码）。
-->
<script setup lang="ts">
import {
  computed,
  nextTick,
  onMounted,
  onUnmounted,
  ref,
  watch,
  type VNode,
} from "vue";
import { useI18n } from "vue-i18n";
import { createVNode, render } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import DOMPurify from "dompurify";
import { marked } from "marked";
import KnowledgeSecretChip from "./KnowledgeSecretChip.vue";
import FluentIcon from "../icons/FluentIcon.vue";
import { vaultErrorCode } from "../../state/vaultAccess.ts";
import {
  closeKnowledgeDoc,
  knowledgeDocFile,
  knowledgeDocOpen,
} from "../../state/knowledgeDoc.ts";
import { confirmDialogOpen } from "../../state/confirmDialog.ts";
import { promptDialogOpen } from "../../state/promptDialog.ts";

/** 掩码 token（两种形态归一到条目名 [REDACTED:label:hash8]）。 */
const TOKEN_RX = /\[(?:REDACTED|vault):([A-Za-z0-9_-]+):([0-9a-f]{8})\]/g;
/** 私用区占位符：\uE000<idx>\uE001（marked / DOMPurify 原样保留）。 */
const PLACEHOLDER_RX = /\uE000(\d+)\uE001/g;
const PLACEHOLDER_OPEN = "\uE000";
const PLACEHOLDER_CLOSE = "\uE001";

type ChipInfo = {
  entry: string;
  label: string;
};

type KnowledgeDoc = {
  ok: boolean;
  path: string;
  root: string | null;
  content: string;
};

const { t } = useI18n();

const fileQuery = computed(() => knowledgeDocFile.value);

const loading = ref(false);
/** 加载失败 i18n key（空串 = 无错）。 */
const loadErrorKey = ref("");
const docTitle = ref("");
const docRelPath = ref("");
const docRoot = ref<string | null>(null);
const renderedHtml = ref("");
const chipInfos = ref<ChipInfo[]>([]);

const docEl = ref<HTMLElement | null>(null);

/** 手动挂载的 chip 实例槽位（文档切换/弹窗关闭时释放）。 */
let mountedSlots: HTMLElement[] = [];

let loadEpoch = 0;

const absPath = computed(() => {
  if (!docRoot.value || !docRelPath.value) return "";
  return `${docRoot.value}\\${docRelPath.value.replace(/\//g, "\\")}`;
});

function unmountChips(): void {
  for (const slot of mountedSlots) render(null, slot);
  mountedSlots = [];
}

function mountChips(): void {
  const root = docEl.value;
  if (!root) return;
  const slots = root.querySelectorAll<HTMLElement>("[data-afm-chip]");
  for (const slot of slots) {
    const idx = Number(slot.dataset.afmChip);
    const chip = chipInfos.value[idx];
    if (!chip) continue;
    const vnode: VNode = createVNode(KnowledgeSecretChip, {
      entry: chip.entry,
      label: chip.label,
      t: (key: string) => t(key),
    });
    render(vnode, slot);
    mountedSlots.push(slot);
  }
}

/** 掩码 token → 私用区占位符；chips 按条目名去重（label 保留首个）。 */
function extractChips(content: string): { text: string; chips: ChipInfo[] } {
  const chips: ChipInfo[] = [];
  const index = new Map<string, number>();
  const text = content.replace(
    TOKEN_RX,
    (_match: string, label: string, hash: string): string => {
      const entry = `[REDACTED:${label}:${hash}]`;
      let chipIdx = index.get(entry);
      if (chipIdx === undefined) {
        chipIdx = chips.length;
        index.set(entry, chipIdx);
        chips.push({ entry, label });
      }
      return `${PLACEHOLDER_OPEN}${chipIdx}${PLACEHOLDER_CLOSE}`;
    },
  );
  return { text, chips };
}

/** 渲染管线：token→占位符 → marked → DOMPurify 消毒 → 占位符替换为 chip 槽位。 */
function renderMarkdown(content: string): { html: string; chips: ChipInfo[] } {
  const { text, chips } = extractChips(content);
  const parsed = marked.parse(text, { async: false, gfm: true, breaks: false });
  const safe = DOMPurify.sanitize(parsed, {
    FORBID_TAGS: ["style", "form", "input", "iframe"],
    FORBID_ATTR: ["style", "onerror", "onload", "onclick"],
  });
  const html = safe.replace(
    PLACEHOLDER_RX,
    (_match: string, idx: string): string =>
      `<span data-afm-chip="${Number(idx)}"></span>`,
  );
  return { html, chips };
}

function titleFromPath(rel: string): string {
  const base = rel.split(/[\\/]/).pop() ?? rel;
  return base.replace(/\.md$/i, "");
}

function errorKeyOf(code: string): string {
  if (code === "doc_not_found") return "knowledgeView.notFound";
  if (code === "doc_too_large") return "knowledgeView.tooLarge";
  if (code === "invalid_path") return "knowledgeView.invalidPath";
  if (code === "offline") return "knowledgeView.offline";
  return "knowledgeView.loadFailed";
}

async function loadDoc(rel: string): Promise<void> {
  const seq = ++loadEpoch;
  loading.value = true;
  loadErrorKey.value = "";
  renderedHtml.value = "";
  chipInfos.value = [];
  docTitle.value = "";
  docRelPath.value = "";
  docRoot.value = null;
  unmountChips();
  try {
    const doc = await invoke<KnowledgeDoc>("knowledge_service_read_doc", {
      path: rel,
    });
    if (seq !== loadEpoch) return;
    docRelPath.value = doc.path;
    docRoot.value = doc.root;
    docTitle.value = titleFromPath(doc.path);
    const { html, chips } = renderMarkdown(doc.content);
    renderedHtml.value = html;
    chipInfos.value = chips;
    loading.value = false;
    await nextTick();
    if (seq !== loadEpoch) return;
    mountChips();
  } catch (error) {
    if (seq !== loadEpoch) return;
    loadErrorKey.value = errorKeyOf(vaultErrorCode(error));
    loading.value = false;
  }
}

watch(
  fileQuery,
  (rel) => {
    if (rel) void loadDoc(rel);
    else {
      loadEpoch++;
      loading.value = false;
      loadErrorKey.value = "";
      renderedHtml.value = "";
      chipInfos.value = [];
      docTitle.value = "";
      docRelPath.value = "";
      docRoot.value = null;
      unmountChips();
    }
  },
  { immediate: true },
);

function onKeydown(event: KeyboardEvent): void {
  if (!knowledgeDocOpen.value) return;
  // chip 解锁确认框 / 输入框打开时由各自 Host 优先接管 Esc。
  if (confirmDialogOpen.value || promptDialogOpen.value) return;
  // Alt+Esc 等系统切换组合键会向应用派发带修饰键的 Esc：切换应用不应关掉本弹窗。
  if (event.altKey || event.metaKey || event.ctrlKey) return;
  if (event.key === "Escape") {
    closeKnowledgeDoc();
    event.preventDefault();
    event.stopPropagation();
  }
}

onMounted(() => {
  window.addEventListener("keydown", onKeydown, true);
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown, true);
  unmountChips();
});

function onRevealInExplorer(): void {
  if (absPath.value) void revealItemInDir(absPath.value);
}

function onReload(): void {
  if (fileQuery.value) void loadDoc(fileQuery.value);
}

/** 外链交由系统浏览器打开（拦截 Webview 默认导航）。 */
function onDocClick(event: MouseEvent): void {
  const anchor = (event.target as HTMLElement | null)?.closest?.("a");
  if (!anchor) return;
  const href = anchor.getAttribute("href") ?? "";
  if (/^https?:\/\//i.test(href)) {
    event.preventDefault();
    void openUrl(href);
  }
}
</script>

<template>
  <div
    v-if="knowledgeDocOpen"
    class="knowledge-dialog-backdrop"
    role="presentation"
  >
    <!-- 背景不响应点击关闭：切换应用后点回本窗口落在遮罩上会误关；仅 Esc / 关闭按钮关闭。 -->
    <div
      class="knowledge-dialog fluent-surface"
      role="dialog"
      aria-modal="true"
      :aria-label="docTitle || t('knowledgeView.title')"
    >
      <header class="knowledge-header">
        <FluentIcon name="library" size="sm" class="knowledge-header-icon" />
        <div class="knowledge-header-main">
          <h1 class="knowledge-title">
            {{ docTitle || t("knowledgeView.title") }}
          </h1>
          <p v-if="docRelPath" class="knowledge-path">{{ docRelPath }}</p>
        </div>
        <button
          v-if="absPath"
          type="button"
          class="knowledge-action fluent-control"
          @click="onRevealInExplorer"
        >
          {{ t("knowledgeView.openInExplorer") }}
        </button>
        <button
          v-if="fileQuery"
          type="button"
          class="knowledge-action fluent-control"
          @click="onReload"
        >
          {{ t("knowledgeView.reload") }}
        </button>
        <button
          type="button"
          class="knowledge-action fluent-control"
          @click="closeKnowledgeDoc"
        >
          {{ t("chrome.close") }}
        </button>
      </header>
      <main class="knowledge-body">
        <p v-if="loading" class="knowledge-status">
          {{ t("knowledgeView.loading") }}
        </p>
        <p
          v-else-if="loadErrorKey"
          class="knowledge-status knowledge-status--error"
        >
          {{ t(loadErrorKey) }}
        </p>
        <article
          v-else
          ref="docEl"
          class="knowledge-doc"
          @click="onDocClick"
          v-html="renderedHtml"
        ></article>
      </main>
    </div>
  </div>
</template>

<style scoped>
.knowledge-dialog-backdrop {
  position: fixed;
  inset: 0;
  /* 面板 1000 之上、解锁确认框 1100 之下：chip 解锁确认始终盖在本弹窗上（ADR 0017）。 */
  z-index: 1050;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--fluent-space-lg);
  background: color-mix(in srgb, var(--fluent-color-text-primary) 28%, transparent);
}

.knowledge-dialog {
  /* 近乎文件列表区域大小：随视口响应式缩放，不再固定 56rem 上限。 */
  width: min(92vw, 1720px);
  height: min(90vh, 1040px);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  box-shadow: var(--fluent-shadow-flyout, 0 12px 32px rgba(0, 0, 0, 0.22));
}

.knowledge-header {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  padding: var(--fluent-space-sm) var(--fluent-space-md);
  border-bottom: var(--fluent-stroke-width) solid
    var(--fluent-color-stroke-divider);
}

.knowledge-header-icon {
  color: var(--fluent-color-fill-accent);
}

.knowledge-header-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.knowledge-title {
  margin: 0;
  font-size: var(--fluent-font-size-subtitle);
  font-weight: var(--fluent-font-weight-semibold);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.knowledge-path {
  margin: 0;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.knowledge-action {
  flex-shrink: 0;
}

.knowledge-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: var(--fluent-space-lg);
}

.knowledge-status {
  margin: var(--fluent-space-xl) auto;
  max-width: 44rem;
  text-align: center;
  color: var(--fluent-color-text-secondary);
}

.knowledge-status--error {
  color: var(--fluent-color-text-primary);
}

/* markdown 正文（渲染产物为消毒 HTML，深度选择器穿透 v-html）。 */
.knowledge-doc {
  max-width: 52rem;
  margin: 0 auto;
  font-size: var(--fluent-font-size-body);
  line-height: 1.7;
  word-wrap: break-word;
  -webkit-user-select: text;
  user-select: text;
}

.knowledge-doc :deep(h1),
.knowledge-doc :deep(h2),
.knowledge-doc :deep(h3),
.knowledge-doc :deep(h4) {
  margin: 1.4em 0 0.6em;
  line-height: 1.3;
}

.knowledge-doc :deep(h1) {
  font-size: 1.6em;
}

.knowledge-doc :deep(h2) {
  font-size: 1.35em;
}

.knowledge-doc :deep(h3) {
  font-size: 1.15em;
}

.knowledge-doc :deep(p),
.knowledge-doc :deep(ul),
.knowledge-doc :deep(ol),
.knowledge-doc :deep(blockquote),
.knowledge-doc :deep(pre),
.knowledge-doc :deep(table) {
  margin: 0.6em 0;
}

.knowledge-doc :deep(ul),
.knowledge-doc :deep(ol) {
  padding-left: 1.5em;
}

.knowledge-doc :deep(li) {
  margin: 0.2em 0;
}

.knowledge-doc :deep(a) {
  color: var(--fluent-color-fill-accent);
}

.knowledge-doc :deep(blockquote) {
  padding: 0.2em 1em;
  border-left: 3px solid var(--fluent-color-stroke-divider);
  color: var(--fluent-color-text-secondary);
}

.knowledge-doc :deep(code) {
  padding: 0.1em 0.35em;
  border-radius: 4px;
  background: color-mix(in srgb, var(--fluent-color-text-primary) 8%, transparent);
  font-family: ui-monospace, Consolas, monospace;
  font-size: 0.9em;
}

.knowledge-doc :deep(pre) {
  padding: var(--fluent-space-sm);
  border-radius: var(--fluent-radius-md);
  background: color-mix(in srgb, var(--fluent-color-text-primary) 8%, transparent);
  overflow-x: auto;
}

.knowledge-doc :deep(pre code) {
  padding: 0;
  background: transparent;
}

.knowledge-doc :deep(table) {
  border-collapse: collapse;
}

.knowledge-doc :deep(th),
.knowledge-doc :deep(td) {
  padding: 0.3em 0.7em;
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
}

.knowledge-doc :deep(img) {
  max-width: 100%;
}

.knowledge-doc :deep(hr) {
  margin: 1.5em 0;
  border: 0;
  border-top: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
}

/* chip 槽位：行内块，随正文行高排布。 */
.knowledge-doc :deep([data-afm-chip]) {
  display: inline;
}
</style>
