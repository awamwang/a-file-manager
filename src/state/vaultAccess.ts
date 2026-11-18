// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 密钥取用共享链路（ADR 0016 / ADR 0017）：
 * - `takeVaultSecret`：复制（Rust 直写剪贴板，明文不过 Vue）/ 短显（vaultChannel 5s 自动回掩码）；
 * - `lockVaultWindowAction`：「锁定密钥窗口」命令动作（确认对话框转发服务端 lock）；
 * - `unlock_required` → Fluent 确认对话框转发 unlock 后重试一次。
 * 搜索密钥行已随 ADR 0017 移除；本模块供知识页密钥 chip 与命令注册表复用。
 * 明文边界：明文只经 Rust 返回值进 vaultChannel 非响应式 Map，不进 Vue 响应式状态、不进日志。
 */

import { invoke } from "@tauri-apps/api/core";
import { openConfirmDialog } from "./confirmDialog.ts";
import { revealSecretTransient } from "./vaultChannel.ts";
import { toastError, toastInfo } from "./toast.ts";
import type { SearchActivateOutcome } from "../search/types.ts";

/** 解锁窗口时长（秒）：确认对话框文案与此一致。 */
export const UNLOCK_TTL_SECONDS = 600;

export type VaultSecretAction = "copy" | "reveal";

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

/** Tauri 命令错误（Rust `KnowledgeError` 序列化）→ code 字符串。 */
export function vaultErrorCode(error: unknown): string {
  if (typeof error === "object" && error !== null && "code" in error) {
    return String((error as { code: unknown }).code);
  }
  return "";
}

/**
 * 密钥取用（复制 / 短显）统一链路：unlock_required 时经确认对话框解锁后重试一次。
 */
export async function takeVaultSecret(
  t: (key: string) => string,
  entry: string,
  action: VaultSecretAction,
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
      return takeVaultSecret(t, entry, action, false);
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
