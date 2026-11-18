// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 知识内容源（ADR 0016 / glossary「知识内容源」「密钥行」）：
 * - 经 awam-knowledge HTTP 服务（Rust `knowledge_service_search`）做 BM25 内容检索；
 * - 命中块 → 文件行（Enter 打开 .md、Ctrl+Enter 资源管理器定位）；
 * - 命中块提取的 `[vault:…]` / `[REDACTED:…]` 引用 → 密钥行（vault 紫 + 锁图标）：
 *   Enter 复制（Rust 直写剪贴板，明文不过 Vue）、Ctrl+Enter 短显 5s 自动回掩码；
 * - 解锁窗口未开（unlock_required）→ Fluent 确认对话框转发 unlock 后重试一次；
 * - 服务离线 / 鉴权失败出状态行，不阻塞其它源。
 */

import { invoke } from "@tauri-apps/api/core";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import {
  openLocalEntry,
  type NavigatorEntry,
} from "../../state/navigatorBrowse.ts";
import { requestShellIcons } from "../../state/shellIcons.ts";
import { toastError, toastInfo } from "../../state/toast.ts";
import { openConfirmDialog } from "../../state/confirmDialog.ts";
import { revealSecretTransient } from "../../state/vaultChannel.ts";
import type {
  SearchActivateOutcome,
  SearchResultItem,
  SearchSource,
  SearchSourceResult,
} from "../types.ts";

/** 带内类型：与服务契约（camelCase）一致。 */
type KnowledgeRef = {
  kind: string;
  key: string;
  display: string;
};

type KnowledgeHit = {
  path: string;
  title: string;
  line: number;
  lines: string;
  score: number;
  snippet: string;
  refs: KnowledgeRef[];
};

type KnowledgeSearchResponse = {
  hits: KnowledgeHit[];
  refs: KnowledgeRef[];
  total: number;
};

type VaultReveal = {
  name: string;
  label: string;
  sensitivity?: string;
  masked?: string;
  value?: string;
};

type VaultWindow = {
  active: boolean;
  expiresAt?: string;
};

type VaultStatus = {
  ok: boolean;
  window?: VaultWindow | null;
  mapExists?: boolean;
};

/** 输入防抖：HTTP 检索每键只发一次。 */
const DEBOUNCE_MS = 180;
/** 内容命中上限（密钥行不计入，来自服务端去重后的 refs）。 */
const HIT_LIMIT = 8;
/** 解锁窗口时长（秒）：确认对话框文案与此一致。 */
const UNLOCK_TTL_SECONDS = 600;

let epoch = 0;

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/** Tauri 命令错误（Rust `KnowledgeError` 序列化）→ code 字符串。 */
function vaultErrorCode(error: unknown): string {
  if (typeof error === "object" && error !== null && "code" in error) {
    return String((error as { code: unknown }).code);
  }
  return "";
}

/**
 * 密钥取用（复制 / 短显）统一链路：unlock_required 时经确认对话框解锁后重试一次。
 */
async function takeSecret(
  t: (key: string) => string,
  entry: string,
  action: "copy" | "reveal",
  allowUnlockPrompt = true,
): Promise<SearchActivateOutcome> {
  try {
    if (action === "copy") {
      await invoke("knowledge_service_copy", { entry });
      return { toastKey: "searchPalette.vault.copied", kind: "info" };
    }
    const reveal = await invoke<VaultReveal>("knowledge_service_reveal", {
      entry,
    });
    revealSecretTransient(entry, reveal.value ?? "");
    return;
  } catch (error) {
    const code = vaultErrorCode(error);
    if (code === "unlock_required" && allowUnlockPrompt) {
      const { confirmed } = await openConfirmDialog({
        title: t("searchPalette.vault.unlockTitle"),
        message: t("searchPalette.vault.unlockMessage"),
        confirmLabel: t("searchPalette.vault.unlockConfirm"),
      });
      if (!confirmed) return;
      try {
        await invoke("knowledge_service_unlock", {
          ttl: UNLOCK_TTL_SECONDS,
          confirm: true,
        });
      } catch {
        return { toastKey: "searchPalette.vault.unlockFailed", kind: "error" };
      }
      return takeSecret(t, entry, action, false);
    }
    if (code === "unauthorized") {
      return { toastKey: "searchPalette.vault.unauthorized", kind: "error" };
    }
    if (code === "rate_limited") {
      return { toastKey: "searchPalette.vault.rateLimited", kind: "error" };
    }
    return { toastKey: "searchPalette.vault.failed", kind: "error" };
  }
}

function normalizeHitPath(path: string): string {
  return path.replace(/\//g, "\\");
}

/** 「锁定密钥窗口」命令动作工厂（commands 注册表复用）：确认对话框转发服务端 lock。 */
export function lockVaultWindowAction(
  t: (key: string) => string,
): () => Promise<void> {
  return async () => {
    let status: VaultStatus;
    try {
      status = await invoke<VaultStatus>("knowledge_service_vault_status");
    } catch {
      toastError(t("searchPalette.vault.lockCheckFailed"));
      return;
    }
    if (!status.window?.active) {
      toastInfo(t("searchPalette.vault.lockNotActive"));
      return;
    }
    const { confirmed } = await openConfirmDialog({
      title: t("searchPalette.vault.lockTitle"),
      message: t("searchPalette.vault.lockMessage"),
      confirmLabel: t("searchPalette.vault.lockConfirm"),
    });
    if (!confirmed) return;
    try {
      await invoke("knowledge_service_lock");
      toastInfo(t("searchPalette.vault.locked"));
    } catch {
      toastError(t("searchPalette.vault.lockFailed"));
    }
  };
}

export function createKnowledgeSource(
  t: (key: string) => string,
): SearchSource {
  return {
    id: "knowledge",
    labelKey: "searchPalette.scopes.knowledge",
    enabled: () => true,
    unavailableKey: "searchPalette.status.knowledgeUnavailable",
    async search(query: string): Promise<SearchSourceResult> {
      const seq = ++epoch;
      const q = query.trim();
      if (!q) return { status: "ready", items: [] };
      await sleep(DEBOUNCE_MS);
      if (seq !== epoch) return { status: "ready", items: [] };
      let result: KnowledgeSearchResponse;
      try {
        result = await invoke<KnowledgeSearchResponse>(
          "knowledge_service_search",
          { query: q, topk: HIT_LIMIT },
        );
      } catch {
        // 服务离线 / 鉴权失败 / 超时：出状态行，不阻塞其它源。
        return { status: "unavailable", items: [] };
      }
      if (seq !== epoch) return { status: "ready", items: [] };
      const items: SearchResultItem[] = [];
      // 密钥行优先（可操作项），再排内容命中；组上限截断时保住密钥行。
      for (const ref of result.refs) {
        if (ref.kind !== "vault" && ref.kind !== "redacted") continue;
        items.push({
          key: `knowledge:secret:${ref.key}`,
          sourceId: "knowledge",
          type: "secret",
          title: ref.display || ref.key,
          subtitle: ref.key,
          icon: "lock",
          secretEntry: ref.key,
          activate: ({ ctrl }) =>
            takeSecret(t, ref.key, ctrl ? "reveal" : "copy"),
        });
      }
      const hits = result.hits.slice(0, HIT_LIMIT);
      requestShellIcons(hits.map((hit) => normalizeHitPath(hit.path)));
      for (const hit of hits) {
        const path = normalizeHitPath(hit.path);
        const entry: NavigatorEntry = {
          name: hit.title,
          path,
          kind: "file",
          size: 0,
        };
        items.push({
          key: `knowledge:note:${path}:${hit.line}`,
          sourceId: "knowledge",
          type: "file",
          title: hit.title,
          subtitle: hit.snippet,
          iconPath: path,
          activate: async ({ ctrl }): Promise<SearchActivateOutcome> => {
            if (ctrl) {
              void revealItemInDir(path);
              return;
            }
            try {
              await openLocalEntry(entry);
            } catch {
              return { toastKey: "searchPalette.openFailed", kind: "error" };
            }
          },
        });
      }
      return { status: "ready", items };
    },
  };
}
