// SPDX-License-Identifier: GPL-3.0-or-later
/** 工具管理运行时 IPC 封装：网关状态、附着/启动与 list/call capability。 */

import { invoke } from "@tauri-apps/api/core";

export type GatewayStatus = {
  reachable: boolean;
  detail: string;
  version: string | null;
};

export type EnsureGatewayResult = {
  pid: number;
  spawned: boolean;
  reachable: boolean;
  detail: string;
};

export type RuntimeTool = {
  id: string;
  name: string;
  kind: string;
  description: string;
  status: string;
};

export type JsonSchema = {
  type?: string | string[];
  properties?: Record<string, JsonSchema>;
  required?: string[];
  enum?: unknown[];
  items?: JsonSchema;
  default?: unknown;
  description?: string;
  title?: string;
};

export type RuntimeCapability = {
  capabilityId: string;
  name: string;
  description: string;
  risk: string;
  inputSchema: JsonSchema | null;
};

export type CallCapabilityArgs = {
  toolId: string;
  capabilityId: string;
  arguments?: Record<string, unknown>;
  execute?: boolean;
  allowDestructive?: boolean;
};

export function invokeErrorMessage(err: unknown): string {
  if (typeof err === "string") return err;
  if (err instanceof Error) return err.message;
  return String(err);
}

export function isToolRuntimeDisabled(message: string): boolean {
  return message.includes("tool_runtime_disabled");
}

export function isDestructiveBlocked(message: string): boolean {
  if (message.includes("permission_denied")) return false;
  if (message.includes("destructive_blocked")) return true;
  const json = extractJsonObject(message);
  if (json?.errorType === "permission_denied") return false;
  if (json?.errorType === "destructive_blocked") return true;
  return message.includes("destructive") || message.includes("破坏性");
}

export function asRecord(value: unknown): Record<string, unknown> | null {
  if (value && typeof value === "object" && !Array.isArray(value)) {
    return value as Record<string, unknown>;
  }
  return null;
}

export function formatPayload(value: unknown): string {
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

export function parseToolList(payload: unknown): RuntimeTool[] {
  const tools = asRecord(payload)?.tools;
  if (!Array.isArray(tools)) return [];
  const rows: RuntimeTool[] = [];
  for (const item of tools) {
    const row = asRecord(item);
    if (!row) continue;
    const id = asString(row.id);
    if (!id) continue;
    rows.push({
      id,
      name: asString(row.name, id),
      kind: asString(row.kind, "unknown"),
      description: asString(row.description),
      status: asString(row.status),
    });
  }
  return rows;
}

export function parseCapabilityList(payload: unknown): RuntimeCapability[] {
  const capabilities = asRecord(payload)?.capabilities;
  if (!Array.isArray(capabilities)) return [];
  const rows: RuntimeCapability[] = [];
  for (const item of capabilities) {
    const row = asRecord(item);
    if (!row) continue;
    const capabilityId = asString(row.capabilityId) || asString(row.name);
    if (!capabilityId) continue;
    rows.push({
      capabilityId,
      name: asString(row.name, capabilityId),
      description: asString(row.description),
      risk: asString(row.risk, "write"),
      inputSchema: asJsonSchema(row.inputSchema),
    });
  }
  return rows;
}

export function gatewayStatus(): Promise<GatewayStatus> {
  return invoke<GatewayStatus>("tool_runtime_gateway_status");
}

export function ensureGateway(): Promise<EnsureGatewayResult> {
  return invoke<EnsureGatewayResult>("tool_runtime_ensure_gateway");
}

export function listTools(kind?: string): Promise<unknown> {
  return invoke<unknown>("tool_runtime_list_tools", kind ? { kind } : {});
}

export function listCapabilities(
  toolId: string,
  refresh?: boolean,
): Promise<unknown> {
  return invoke<unknown>("tool_runtime_list_capabilities", { toolId, refresh });
}

export function callCapability(args: CallCapabilityArgs): Promise<unknown> {
  return invoke<unknown>("tool_runtime_call_capability", {
    toolId: args.toolId,
    capabilityId: args.capabilityId,
    arguments: args.arguments ?? {},
    execute: args.execute,
    allowDestructive: args.allowDestructive,
  });
}

function asString(value: unknown, fallback = ""): string {
  return typeof value === "string" ? value : fallback;
}

function asJsonSchema(value: unknown): JsonSchema | null {
  return asRecord(value) as JsonSchema | null;
}

function extractJsonObject(text: string): Record<string, unknown> | null {
  const start = text.indexOf("{");
  const end = text.lastIndexOf("}");
  if (start < 0 || end <= start) return null;
  try {
    return asRecord(JSON.parse(text.slice(start, end + 1)));
  } catch {
    return null;
  }
}
