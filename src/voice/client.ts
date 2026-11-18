// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 语音输入套件 typed IPC 客户端（ADR 0018）。
 * 搜索面板与设置「语音自检」经此访问 voice_* 命令与 `voice://*` 事件；
 * 负载字段与 Rust 侧 serde camelCase 序列化一一对应。
 */

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type VoiceState = "recording" | "transcribing" | "idle";
export type VoiceSource = "palette" | "selftest";

export type VoiceStateEvent = { state: VoiceState; source: VoiceSource };
export type VoiceTranscriptEvent = {
  text: string;
  source: VoiceSource;
  elapsedMs: number;
};
export type VoiceErrorEvent = { code: string; source: VoiceSource; message: string };
/** 命令同步失败时 invoke 拒绝的负载（Rust `VoiceError` 序列化）。 */
export type VoiceError = { code: string; message: string; status: number };

export const VOICE_STATE_EVENT = "voice://state";
export const VOICE_TRANSCRIPT_EVENT = "voice://transcript";
export const VOICE_ERROR_EVENT = "voice://error";

export function voiceStart(): Promise<void> {
  return invoke<void>("voice_start");
}

export function voiceSelfTest(): Promise<void> {
  return invoke<void>("voice_selftest");
}

export function voiceStop(): Promise<void> {
  return invoke<void>("voice_stop");
}

export function voiceCancel(): Promise<void> {
  return invoke<void>("voice_cancel");
}

export function onVoiceStateEvent(
  handler: (payload: VoiceStateEvent) => void,
): Promise<UnlistenFn> {
  return listen<VoiceStateEvent>(VOICE_STATE_EVENT, (event) => handler(event.payload));
}

export function onVoiceTranscriptEvent(
  handler: (payload: VoiceTranscriptEvent) => void,
): Promise<UnlistenFn> {
  return listen<VoiceTranscriptEvent>(VOICE_TRANSCRIPT_EVENT, (event) =>
    handler(event.payload),
  );
}

export function onVoiceErrorEvent(
  handler: (payload: VoiceErrorEvent) => void,
): Promise<UnlistenFn> {
  return listen<VoiceErrorEvent>(VOICE_ERROR_EVENT, (event) => handler(event.payload));
}
