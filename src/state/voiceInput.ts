// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * 语音输入状态机（ADR 0018）：idle → recording → transcribing → idle。
 * Rust `voice://*` 事件驱动；单飞会话，同一时刻最多一路录音。
 * 接入点按 source 分发：palette 转写文本经 `onPaletteTranscript` 订阅回填搜索框，
 * 错误走 toast；selftest 状态/结果/错误以独立 ref 供设置面板内联回显，不弹 toast。
 */

import { computed, ref } from "vue";
import { AI_ADDON_ENABLED } from "../config/flavor.ts";
import { i18n } from "../i18n/index.ts";
import { toastError } from "./toast.ts";
import {
  onVoiceErrorEvent,
  onVoiceStateEvent,
  onVoiceTranscriptEvent,
  voiceCancel,
  voiceSelfTest,
  voiceStart,
  voiceStop,
  type VoiceErrorEvent,
  type VoiceSource,
  type VoiceStateEvent,
  type VoiceTranscriptEvent,
} from "../voice/client.ts";

export type VoicePhase = "idle" | "recording" | "transcribing";

/** 当前（或最近一次）会话的阶段与来源。 */
export const voicePhase = ref<VoicePhase>("idle");
export const voiceSource = ref<VoiceSource | null>(null);

/** 搜索面板可据此切麦克风按钮态与 placeholder。 */
export const paletteVoiceActive = computed(
  () => voiceSource.value === "palette" && voicePhase.value !== "idle",
);

// ---------------------------------------------------------------------------
// 语音自检（设置面板内联回显，不弹 toast）
// ---------------------------------------------------------------------------

export const selfTestPhase = ref<VoicePhase>("idle");
export const selfTestResult = ref<{ text: string; elapsedMs: number } | null>(null);
export const selfTestError = ref<{ code: string; message: string } | null>(null);

// ---------------------------------------------------------------------------
// palette 转写分发（Phase 4 由 CommandPanel 订阅回填 query）
// ---------------------------------------------------------------------------

type PaletteTranscriptListener = (event: VoiceTranscriptEvent) => void;
const paletteTranscriptListeners = new Set<PaletteTranscriptListener>();

export function onPaletteTranscript(listener: PaletteTranscriptListener): () => void {
  paletteTranscriptListeners.add(listener);
  return () => {
    paletteTranscriptListeners.delete(listener);
  };
}

// ---------------------------------------------------------------------------
// 事件接线
// ---------------------------------------------------------------------------

let initialized = false;

function applyStateEvent(payload: VoiceStateEvent): void {
  if (payload.source !== "palette" && payload.source !== "selftest") return;
  const phase: VoicePhase =
    payload.state === "recording" || payload.state === "transcribing"
      ? payload.state
      : "idle";
  voicePhase.value = phase;
  voiceSource.value = payload.source;
  if (payload.source === "selftest") {
    selfTestPhase.value = phase;
  }
}

function applyTranscriptEvent(payload: VoiceTranscriptEvent): void {
  if (payload.source === "selftest") {
    selfTestResult.value = { text: payload.text, elapsedMs: payload.elapsedMs };
    selfTestError.value = null;
    return;
  }
  if (payload.source !== "palette") return;
  for (const listener of paletteTranscriptListeners) {
    listener(payload);
  }
}

function applyErrorEvent(payload: VoiceErrorEvent): void {
  const normalized = normalizeVoiceError(payload);
  if (payload.source === "selftest") {
    selfTestError.value = normalized;
    return;
  }
  if (payload.source !== "palette") return;
  toastError(i18n.global.t(voiceErrorMessageKey(normalized.code)));
}

/**
 * 初始化语音输入：订阅 `voice://*` 事件。幂等，AI 包 flavor 下由接入点首次使用时调用；
 * Core 包（VITE_AFM_AI_ADDON=0）下为空操作，命令本身也未注册。
 */
export function initVoiceInput(): void {
  if (initialized || !AI_ADDON_ENABLED) return;
  initialized = true;
  void onVoiceStateEvent(applyStateEvent).catch(() => undefined);
  void onVoiceTranscriptEvent(applyTranscriptEvent).catch(() => undefined);
  void onVoiceErrorEvent(applyErrorEvent).catch(() => undefined);
}

// ---------------------------------------------------------------------------
// 动作：搜索面板
// ---------------------------------------------------------------------------

/** 命令拒绝负载 / 事件错误负载 → 统一 { code, message }。 */
function normalizeVoiceError(
  error: VoiceErrorEvent | unknown,
): { code: string; message: string } {
  if (typeof error === "object" && error !== null) {
    const record = error as Record<string, unknown>;
    const code = typeof record.code === "string" ? record.code : "server";
    const message = typeof record.message === "string" ? record.message : "";
    return { code, message };
  }
  return { code: "server", message: String(error) };
}

/** 稳定错误码 → i18n 键（5 locale 同键集）。 */
export function voiceErrorMessageKey(code: string): string {
  switch (code) {
    case "busy":
      return "voice.error.busy";
    case "no-device":
      return "voice.error.noDevice";
    case "permission":
      return "voice.error.permission";
    case "not-configured":
      return "voice.error.notConfigured";
    case "empty":
      return "voice.error.empty";
    case "no-audio":
      return "voice.error.noAudio";
    case "no-speech":
      return "voice.error.noSpeech";
    case "network":
      return "voice.error.network";
    case "unauthorized":
      return "voice.error.unauthorized";
    case "server":
      return "voice.error.server";
    default:
      return "voice.error.server";
  }
}

function toastVoiceCommandError(error: unknown): void {
  const normalized = normalizeVoiceError(error);
  toastError(i18n.global.t(voiceErrorMessageKey(normalized.code)));
}

/** 开始录音（palette 来源）；同步失败（未配置/无设备/占线）toast 提示。 */
export async function startPaletteVoice(): Promise<void> {
  initVoiceInput();
  try {
    await voiceStart();
  } catch (error) {
    toastVoiceCommandError(error);
  }
}

/** 停止录音并转写；结果经 `voice://transcript` 事件回到订阅方。 */
export async function stopPaletteVoice(): Promise<void> {
  try {
    await voiceStop();
  } catch (error) {
    toastVoiceCommandError(error);
  }
}

/** 麦克风按钮开关：录音中 → 停止转写；空闲 → 开始；识别中不动作（链路已在收尾）。 */
export async function togglePaletteVoice(): Promise<void> {
  if (voiceSource.value !== "palette" || voicePhase.value === "idle") {
    await startPaletteVoice();
    return;
  }
  if (voicePhase.value === "recording") {
    await stopPaletteVoice();
  }
}

// ---------------------------------------------------------------------------
// 动作：语音自检（设置面板）
// ---------------------------------------------------------------------------

/** 清空自检回显（再次点击测试前重置）。 */
export function resetSelfTest(): void {
  selfTestPhase.value = "idle";
  selfTestResult.value = null;
  selfTestError.value = null;
}

/** 开始自检试录；同步失败写入 `selfTestError` 内联回显。 */
export async function startSelfTest(): Promise<void> {
  initVoiceInput();
  resetSelfTest();
  try {
    await voiceSelfTest();
  } catch (error) {
    selfTestError.value = normalizeVoiceError(error);
  }
}

/** 自检录音中点击 → 停止并转写；识别中点击不动作（上传线程已脱管，结果即将返回）。 */
export async function stopSelfTest(): Promise<void> {
  if (selfTestPhase.value !== "recording") return;
  try {
    await voiceStop();
  } catch (error) {
    selfTestError.value = normalizeVoiceError(error);
  }
}

/** Esc 关面板等场景：取消录音/识别，不出文本。 */
export async function cancelVoice(): Promise<void> {
  try {
    await voiceCancel();
  } catch {
    /* 会话不存在时 Rust 侧静默成功；此处忽略意外错误 */
  }
}

/** 测试辅助：重置内存态。 */
export function resetVoiceInputForTests(): void {
  voicePhase.value = "idle";
  voiceSource.value = null;
  selfTestPhase.value = "idle";
  selfTestResult.value = null;
  selfTestError.value = null;
  paletteTranscriptListeners.clear();
  initialized = false;
}
