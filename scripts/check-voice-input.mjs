// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * ADR 0018 — voice input suite gate.
 * Asserts the full wiring across layers: Rust voice module (ai-addon feature),
 * settings schema v13 + migration, typed IPC client, state machine, search
 * palette integration (mic button / Esc cancel / auto-start), shortcut chord,
 * settings UI voice section + self-test, and 5-locale i18n key parity.
 */
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");

// ---------------------------------------------------------------------------
// 1. Rust voice module + AI add-on feature wiring
// ---------------------------------------------------------------------------

for (const rel of [
  "src-tauri/src/voice/mod.rs",
  "src-tauri/src/voice/capture.rs",
  "src-tauri/src/voice/engine.rs",
]) {
  assert.ok(existsSync(join(root, rel)), `voice module file exists: ${rel}`);
}

const cargoToml = read("src-tauri/Cargo.toml");
assert.ok(/ai-addon\s*=\s*\["dep:cpal"\]/.test(cargoToml), "ai-addon feature pulls optional cpal");
assert.ok(/cpal\s*=\s*\{\s*version\s*=\s*"0\.15",\s*optional\s*=\s*true\s*\}/.test(cargoToml), "cpal is optional");

const libRs = read("src-tauri/src/lib.rs");
assert.ok(/#\[cfg\(feature = "ai-addon"\)\]\s*\r?\nmod voice;/.test(libRs), "voice module behind ai-addon feature");
for (const cmd of ["voice_start", "voice_stop", "voice_cancel", "voice_selftest"]) {
  assert.ok(libRs.includes(`voice::${cmd},`), `lib.rs registers ${cmd}`);
}

const voiceMod = read("src-tauri/src/voice/mod.rs");
for (const event of ["voice://state", "voice://transcript", "voice://error"]) {
  assert.ok(voiceMod.includes(`"${event}"`), `voice event constant: ${event}`);
}
// Stable error codes consumed by the frontend i18n mapping.
for (const code of ["busy", "no-device", "permission", "not-configured", "empty", "no-audio", "no-speech", "network", "unauthorized", "server"]) {
  assert.ok(voiceMod.includes(`"${code}"`), `voice error code: ${code}`);
}

// Windows microphone consent pre-check: registry ConsentStore read before opening the device.
const voiceCapture = read("src-tauri/src/voice/capture.rs");
assert.ok(voiceCapture.includes("microphone_consent_denied"), "consent check entry point");
assert.ok(voiceCapture.includes("CapabilityAccessManager"), "consent store registry path queried");

const voiceEngine = read("src-tauri/src/voice/engine.rs");
assert.ok(voiceEngine.includes("pub fn normalize_base_url"), "engine normalizes base URL");
assert.ok(voiceEngine.includes("pub fn encode_wav"), "engine encodes WAV (hand-written header)");
assert.ok(voiceEngine.includes("pub fn build_multipart"), "engine builds multipart body");
assert.ok(voiceEngine.includes("pub fn transcribe"), "engine posts transcriptions");
assert.ok(voiceEngine.includes("pub fn resample_to_16k_mono"), "engine resamples to 16k mono");
assert.ok(voiceEngine.includes("power.sttBaseUrl"), "engine reads power.sttBaseUrl");
assert.ok(voiceEngine.includes("power.sttApiKey"), "engine reads power.sttApiKey");

// ---------------------------------------------------------------------------
// 2. Settings schema v13: definitions, defaults, migration
// ---------------------------------------------------------------------------

const typesRs = read("src-tauri/src/settings/types.rs");
assert.ok(/SETTINGS_SCHEMA_VERSION:\s*u32\s*=\s*15/.test(typesRs), "schema version bumped to 15");

const defaultsRs = read("src-tauri/src/settings/defaults.rs");
for (const key of [
  "power.voiceSearchAutoStart",
  "power.sttBaseUrl",
  "power.sttModel",
  "power.sttApiKey",
]) {
  assert.ok(defaultsRs.includes(`"${key}"`), `defaults define ${key}`);
}

const migrateRs = read("src-tauri/src/settings/migrate.rs");
assert.ok(
  /\(12, 13\)/.test(migrateRs),
  "migration step (12, 13) exists for voice keys",
);
assert.ok(migrateRs.includes("power.voiceSearchAutoStart"), "migration seeds power.voiceSearchAutoStart");

// ---------------------------------------------------------------------------
// 3. Frontend typed client + state machine
// ---------------------------------------------------------------------------

const voiceClient = read("src/voice/client.ts");
assert.ok(voiceClient.includes('invoke<void>("voice_start")'), "client invokes voice_start");
assert.ok(voiceClient.includes('invoke<void>("voice_stop")'), "client invokes voice_stop");
assert.ok(voiceClient.includes('invoke<void>("voice_cancel")'), "client invokes voice_cancel");
assert.ok(voiceClient.includes("voice_selftest"), "client invokes voice_selftest");
assert.ok(voiceClient.includes('export const VOICE_STATE_EVENT = "voice://state"'), "state event constant");
assert.ok(voiceClient.includes('export const VOICE_TRANSCRIPT_EVENT = "voice://transcript"'), "transcript event constant");
assert.ok(voiceClient.includes('export const VOICE_ERROR_EVENT = "voice://error"'), "error event constant");

const voiceInput = read("src/state/voiceInput.ts");
assert.ok(voiceInput.includes("export const voicePhase"), "state machine exports voicePhase");
assert.ok(voiceInput.includes("export const paletteVoiceActive"), "state machine exports paletteVoiceActive");
assert.ok(voiceInput.includes("export function onPaletteTranscript"), "palette transcript subscription");
assert.ok(voiceInput.includes("export function initVoiceInput"), "initVoiceInput (idempotent)");
assert.ok(voiceInput.includes("togglePaletteVoice"), "toggle action");
assert.ok(voiceInput.includes("cancelVoice"), "cancel action");
assert.ok(voiceInput.includes("export function voiceErrorMessageKey"), "error code → i18n key mapping");
assert.ok(voiceInput.includes("AI_ADDON_ENABLED"), "state machine gated by AI add-on flavor");
// palette errors toast; selftest errors stay inline (no toast).
assert.ok(
  voiceInput.includes("toastError(i18n.global.t(voiceErrorMessageKey(normalized.code)))"),
  "palette errors toast",
);
assert.ok(/payload\.source === "selftest"\)\s*\{\s*selfTestError\.value/.test(voiceInput), "selftest errors inline");

// ---------------------------------------------------------------------------
// 4. Search palette integration (CommandPanel) + global shortcut
// ---------------------------------------------------------------------------

const commandPanel = read("src/components/command/CommandPanel.vue");
assert.ok(commandPanel.includes("cmd-voice-btn"), "mic button rendered");
assert.ok(commandPanel.includes('name="mic"'), "mic button uses mic glyph");
assert.ok(commandPanel.includes("togglePaletteVoice()"), "mic button toggles voice");
assert.ok(commandPanel.includes("maybeAutoStartVoice"), "auto-start on panel open");
assert.ok(commandPanel.includes('values["power.voiceSearchAutoStart"] === true'), "auto-start reads power.voiceSearchAutoStart");
assert.ok(commandPanel.includes("appendPaletteTranscript"), "transcript appended into query");
assert.ok(commandPanel.includes("onPaletteTranscript(appendPaletteTranscript)"), "transcript listener registered");
assert.ok(commandPanel.includes("initVoiceInput()"), "panel initializes voice input");
assert.ok(commandPanel.includes("inputPlaceholder"), "recording placeholder swap");
assert.ok(commandPanel.includes("cancelVoice()"), "Esc / panel close cancels recording");
assert.ok(commandPanel.includes("AI_ADDON_ENABLED"), "panel voice UI gated by AI add-on flavor");
// Esc during recording cancels voice instead of closing the panel.
assert.ok(
  /event\.key === "Escape"[\s\S]*?paletteVoiceActive\.value && voicePhase\.value === "recording"[\s\S]*?cancelVoice\(\)/.test(
    commandPanel,
  ),
  "Esc cancels recording before closing panel",
);
// Panel close tears down any active voice session.
assert.ok(
  /else\s*\{[\s\S]*?paletteVoiceActive\.value[\s\S]*?cancelVoice\(\)[\s\S]*?closeAllOverlay\(\)/.test(
    commandPanel,
  ),
  "panel close cancels active voice session",
);

const registry = read("src/shortcuts/registry.ts");
assert.ok(
  /id: "commandPanel\.voiceToggle"[\s\S]*?labelKey: "shortcuts\.items\.voiceToggle"[\s\S]*?chord: \{ ctrl: true, key: "m" \}/.test(
    registry,
  ),
  "voiceToggle shortcut: Ctrl+M with i18n label",
);

const appShell = read("src/components/shell/AppShell.vue");
assert.ok(
  appShell.includes('registerShortcutHandler("commandPanel.voiceToggle"'),
  "AppShell registers voiceToggle handler",
);
assert.ok(
  /commandPanel\.voiceToggle"[\s\S]*?commandPanelOpen\.value[\s\S]*?togglePaletteVoice\(\)/.test(appShell),
  "voiceToggle handler gated on commandPanelOpen",
);
assert.ok(appShell.includes('"commandPanel.voiceToggle"'), "voiceToggle unregistered on teardown");
// The Search Palette toggle chord (Ctrl+Shift+P / backtick / remap) finalizes an
// active recording instead of closing the panel: stop → transcribe → append query.
assert.ok(
  /commandPanel\.toggle", \(\) => \{[\s\S]*?paletteVoiceActive\.value[\s\S]*?voicePhase\.value === "recording"[\s\S]*?stopPaletteVoice\(\)/.test(
    appShell,
  ),
  "palette toggle chord stops recording instead of closing the panel",
);

// ---------------------------------------------------------------------------
// 5. Settings UI: hidden keys, voice section, self-test
// ---------------------------------------------------------------------------

const settingsUi = read("src/state/settingsUi.ts");
for (const key of [
  "power.voiceSearchAutoStart",
  "power.sttBaseUrl",
  "power.sttModel",
  "power.sttApiKey",
]) {
  assert.ok(settingsUi.includes(`"${key}"`), `settingsUi hides ${key}`);
}

const searchSettings = read("src/components/settings/SearchSettingsPanel.vue");
assert.ok(searchSettings.includes('data-voice-settings'), "voice settings section present");
assert.ok(searchSettings.includes("voiceTitle"), "voice section title");
assert.ok(searchSettings.includes("startSelfTest"), "self-test start action");
assert.ok(searchSettings.includes("stopSelfTest"), "self-test stop action");
assert.ok(voiceInput.includes("resetSelfTest"), "self-test reset action");
assert.ok(searchSettings.includes("selfTestPhase"), "self-test inline phase feedback");
assert.ok(searchSettings.includes("permissionHint"), "permission hint shown on permission errors");
assert.ok(searchSettings.includes("AI_ADDON_ENABLED"), "voice settings gated by AI add-on flavor");

// ---------------------------------------------------------------------------
// 6. i18n: 5-locale parity for the voice key set
// ---------------------------------------------------------------------------

const locales = ["en", "zh-CN", "ja", "de", "fr"];
const voiceI18nKeys = [
  "voice.palette.listening",
  "voice.palette.transcribing",
  "voice.error.busy",
  "voice.error.noDevice",
  "voice.error.permission",
  "voice.error.notConfigured",
  "voice.error.empty",
  "voice.error.noAudio",
  "voice.error.noSpeech",
  "voice.error.network",
  "voice.error.unauthorized",
  "voice.error.server",
  "voice.selftest.start",
  "voice.selftest.stop",
  "voice.selftest.recording",
  "voice.selftest.transcribing",
  "voice.selftest.result",
  "voice.selftest.permissionHint",
  "voice.selftest.configSaved",
  "shortcuts.items.voiceToggle",
];
for (const locale of locales) {
  const doc = JSON.parse(read(`src/i18n/locales/${locale}.json`));
  const has = ( dotted ) => {
    let node = doc;
    for (const part of dotted.split(".")) {
      if (typeof node !== "object" || node === null || !(part in node)) return false;
      node = node[part];
    }
    return typeof node === "string";
  };
  for (const key of voiceI18nKeys) {
    assert.ok(has(key), `${locale} has ${key}`);
  }
}

console.log("voice input suite checks OK (ADR 0018)");
