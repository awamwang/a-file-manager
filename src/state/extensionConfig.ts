// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 扩展配置读写（发现机制的共享实现）：服务配置页、各功能页的「本服务配置」区块、
 * 以及提供程序卡片的内联单项编辑都走这里，避免三份重复逻辑。
 *
 * 值统一存进 settings 文档（pack 声明的 settingKey 经后端 schema overlay 落盘）；
 * 保存后对「值发生变化且处于托管模式」的 pack 自动 stop→ensure 重启，使新配置生效。
 */
import { invoke } from "@tauri-apps/api/core";
import {
  configExtras,
  type FeaturePackConfigContribution,
} from "./featurePack/contributions";
import {
  hydrateSettings,
  rememberSettingsDocument,
  type SettingsDocument,
} from "./settingsHydrate";

export type ConfigRow = FeaturePackConfigContribution & { settingKey: string };

type ManagedRecord = { packId: string; mode: string };

export type ConfigGroup = { packId: string; packName: string; items: ConfigRow[] };

/** 已启用 pack 声明的配置项，按 pack 分组（保持 contributions 的稳定排序）。 */
export function configGroups(): ConfigGroup[] {
  const map = new Map<string, ConfigGroup>();
  for (const item of configExtras.value) {
    if (!item.settingKey) continue;
    const row = item as ConfigRow;
    const packId = row.packId ?? "";
    if (!map.has(packId)) {
      map.set(packId, { packId, packName: row.packName ?? packId, items: [] });
    }
    map.get(packId)!.items.push(row);
  }
  return [...map.values()];
}

/** 指定 pack 的配置项（未知/未启用 → 空数组）。 */
export function configRowsFor(packId: string): ConfigRow[] {
  return configGroups().find((g) => g.packId === packId)?.items ?? [];
}

/** 按 settingKey 找配置声明（内联单项编辑用）。 */
export function findConfigRow(settingKey: string): ConfigRow | null {
  for (const group of configGroups()) {
    const hit = group.items.find((row) => row.settingKey === settingKey);
    if (hit) return hit;
  }
  return null;
}

/** 按 pack + 声明 id 找配置声明。 */
export function findConfigRowByPack(packId: string, id: string): ConfigRow | null {
  return configRowsFor(packId).find((row) => row.id === id) ?? null;
}

export function isManagedMode(mode: string | undefined): boolean {
  return mode === "managed";
}

/** 读取托管模式映射（packId → mode）。 */
export async function loadManagedModes(): Promise<Record<string, string>> {
  try {
    const records = await invoke<ManagedRecord[]>("managed_dependency_list");
    const map: Record<string, string> = {};
    for (const record of records) map[record.packId] = record.mode;
    return map;
  } catch {
    return {};
  }
}

export function coerceForInput(row: ConfigRow, raw: unknown): unknown {
  switch (row.kind) {
    case "boolean":
      return raw === true;
    case "number":
      if (typeof raw === "number") return raw;
      if (raw != null && String(raw).trim() !== "") {
        const n = Number(raw);
        return Number.isFinite(n) ? n : 0;
      }
      return 0;
    default:
      return typeof raw === "string" ? raw : raw == null ? "" : String(raw);
  }
}

export function normalizeForSave(row: ConfigRow, val: unknown): unknown {
  switch (row.kind) {
    case "boolean":
      return val === true;
    case "number": {
      const n = Number(val);
      return Number.isFinite(n) ? n : 0;
    }
    default: {
      let text = val == null ? "" : String(val).trim();
      // 连接地址类（settingKey 以 Url 结尾）去掉尾斜杠，与迁移前一致。
      if (/url$/i.test(row.settingKey)) text = text.replace(/\/+$/, "");
      return text;
    }
  }
}

/** 从 settings 文档读出这些行的当前值（缺失按类型默认）。 */
export async function loadConfigValues(
  rows: ConfigRow[],
): Promise<Record<string, unknown>> {
  let doc: SettingsDocument | null = null;
  try {
    doc = await hydrateSettings();
  } catch {
    doc = null;
  }
  const values: Record<string, unknown> = {};
  for (const row of rows) {
    values[row.settingKey] = coerceForInput(row, doc?.values[row.settingKey]);
  }
  return values;
}

/** 值是否为空（"没存过" → 服务端会用自己的默认值）。 */
export function isEmptyValue(row: ConfigRow, value: unknown): boolean {
  if (row.kind === "boolean") return false;
  if (row.kind === "number") return value == null || value === "";
  return String(value ?? "").trim() === "";
}

/** 相对原始值，哪些 settingKey 真的变了。 */
export function diffChangedKeys(
  rows: ConfigRow[],
  values: Record<string, unknown>,
  original: Record<string, unknown>,
): string[] {
  const changed: string[] = [];
  for (const row of rows) {
    const next = normalizeForSave(row, values[row.settingKey]);
    const prev = normalizeForSave(row, original[row.settingKey]);
    if (JSON.stringify(next) !== JSON.stringify(prev)) changed.push(row.settingKey);
  }
  return changed;
}

/** 重启受影响的托管服务：stop 仅作用于 AFM 自启 pid；附着/未拉起时 stop 会报错，忽略后 ensure 重新附着或拉起。 */
export async function restartManagedPack(packId: string): Promise<void> {
  try {
    await invoke("managed_dependency_stop", { id: packId });
  } catch {
    /* 非自启实例，无需停 */
  }
  await invoke("managed_dependency_ensure", { id: packId });
}

/**
 * 保存这些行的值，并对「值有变化且处于托管模式」的 pack 重启使配置生效。
 * 返回已重启的 packId；抛错由调用方提示。
 */
export async function saveConfig(
  rows: ConfigRow[],
  values: Record<string, unknown>,
  original: Record<string, unknown>,
  managedModes: Record<string, string>,
): Promise<string[]> {
  const patch: Record<string, unknown> = {};
  for (const row of rows) {
    patch[row.settingKey] = normalizeForSave(row, values[row.settingKey]);
  }
  const doc = await invoke<SettingsDocument>("settings_set", { patch });
  rememberSettingsDocument(doc);

  const changed = new Set(diffChangedKeys(rows, values, original));
  const packs = new Set(
    rows
      .filter((row) => changed.has(row.settingKey))
      .map((row) => row.packId ?? "")
      .filter((packId) => packId && isManagedMode(managedModes[packId])),
  );
  const restarted: string[] = [];
  for (const packId of packs) {
    try {
      await restartManagedPack(packId);
      restarted.push(packId);
    } catch {
      // 重启失败不阻断保存结果；调用方按 success 提示，服务健康页会显示真实状态。
    }
  }
  return restarted;
}