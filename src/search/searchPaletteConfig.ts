// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 搜索面板 Core 配置（`<appConfigDir>/command-palette.json`，Rust `palette_get_config`）。
 * 文件缺失 / JSON 损坏 / 动作非法一律回落 `navigate`，不阻断面板。
 * 见 docs/command-palette-config.md。
 */

import { invoke } from "@tauri-apps/api/core";

export type PaletteToolAction =
  | { enter: "navigate" }
  | {
      enter: "callCapability";
      capabilityId: string;
      arguments?: Record<string, unknown>;
    };

export type PaletteConfig = {
  schemaVersion: number;
  toolActions: Record<string, PaletteToolAction>;
};

export const EMPTY_PALETTE_CONFIG: PaletteConfig = {
  schemaVersion: 1,
  toolActions: {},
};

function normalizeArguments(
  value: unknown,
): Record<string, unknown> | undefined {
  if (value && typeof value === "object" && !Array.isArray(value)) {
    return value as Record<string, unknown>;
  }
  return undefined;
}

/** 非法动作（未知 enter、callCapability 缺 capabilityId）→ null（回落 navigate）。 */
function normalizeAction(raw: unknown): PaletteToolAction | null {
  if (!raw || typeof raw !== "object") return null;
  const record = raw as Record<string, unknown>;
  if (record.enter === "callCapability") {
    const capabilityId =
      typeof record.capabilityId === "string" ? record.capabilityId.trim() : "";
    if (!capabilityId) return null;
    return {
      enter: "callCapability",
      capabilityId,
      arguments: normalizeArguments(record.arguments),
    };
  }
  // navigate 及一切非法值统一回落 navigate。
  return { enter: "navigate" };
}

function normalizeConfig(raw: unknown): PaletteConfig {
  if (!raw || typeof raw !== "object") return EMPTY_PALETTE_CONFIG;
  const record = raw as Record<string, unknown>;
  const toolActions: Record<string, PaletteToolAction> = {};
  const rawActions = record.toolActions;
  if (rawActions && typeof rawActions === "object" && !Array.isArray(rawActions)) {
    for (const [toolId, actionRaw] of Object.entries(rawActions)) {
      const action = normalizeAction(actionRaw);
      if (action) toolActions[toolId] = action;
    }
  }
  return {
    schemaVersion:
      typeof record.schemaVersion === "number" ? record.schemaVersion : 1,
    toolActions,
  };
}

let cache: Promise<PaletteConfig> | null = null;

/** 读取配置（会话内缓存一次）；IPC 失败回落空配置。 */
export function loadSearchPaletteConfig(): Promise<PaletteConfig> {
  cache ??= invoke<unknown>("palette_get_config")
    .then(normalizeConfig)
    .catch(() => EMPTY_PALETTE_CONFIG);
  return cache;
}

/** 取某工具的 Enter 动作；未配置 → navigate。 */
export function paletteActionFor(
  config: PaletteConfig,
  toolId: string,
): PaletteToolAction {
  return config.toolActions[toolId] ?? { enter: "navigate" };
}
