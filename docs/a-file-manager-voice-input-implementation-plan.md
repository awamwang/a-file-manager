<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# 语音输入套件实施计划（Voice Input Suite）

状态：设计已确认，尚未开始实现。本文是实现入口；术语见 [语音输入术语](./glossary-voice-input.md)，整体决策见 [ADR 0018](./adr/0018-voice-input-suite-openai-compatible-transcription.md)。

## 1. 目标

给 AFM 接入全应用可用的「说话变文字」能力：统一语音服务（Rust 采集 + OpenAI 兼容转写协议）+ 可配置云端引擎；第一个且当前唯一的接入点是搜索面板——麦克风按钮、面板内快捷键（默认 Ctrl+M，可改键）、以及「语音搜索」配置（开启后调起面板即自动录音）。识别文本追加进面板 query，跟随面板既有「输入即搜」。随 AI 包 flavor 分发。

## 2. 不变量

- Core flavor 不新增任何运行时依赖与可见 UI；语音能力仅存在于 AI flavor（cargo feature `ai-addon` + `VITE_AFM_AI_ADDON`）。
- 面板「输入即搜」链路不因语音改动：语音回填只是修改 query 值，不新增搜索触发分支；不引入「识别后自动搜索」开关。
- 不做系统级全局热键；快捷键 `voice.toggle` 为应用内注册、面板打开时生效。
- Rust 命令名 `voice_start / voice_stop / voice_cancel / voice_selftest`、事件名 `voice://state / voice://transcript / voice://error`、设置键 `power.voiceSearchAutoStart / power.sttBaseUrl / power.sttModel / power.sttApiKey` 保持不变。
- 现有 `test:search-palette`、`test:shortcuts`、`test:i18n`、`test:deps-surface`、`test:settings-switch`、`npm run test:rust` 必须继续通过。
- v1 只预制 OpenAI 兼容批量转写协议；流式协议、端侧引擎、更多接入点仅留接口不实现。

## 3. 架构

```
CommandPanel.vue（唯一接入点）
   ├─ voiceInput.ts 状态机 idle→recording→transcribing→idle/error
   │     └─ src/voice/client.ts（typed invoke / listen）
   │            ↘ Tauri IPC ↙
   │     src-tauri/src/voice/
   │        ├─ mod.rs      voice_start/stop/cancel/selftest + 事件发射
   │        ├─ capture.rs  cpal 采集（16kHz mono i16，60s 自动截止）
   │        └─ engine.rs   hound 编 WAV → ureq 手工 multipart
   │                       → POST {baseUrl}/v1/audio/transcriptions
   ├─ SearchSettingsPanel.vue 语音输入组：语音搜索开关 + baseUrl/model/key
   │     └─ 语音自检（开启时弹起试录 + 常驻「测试」按钮）
   └─ shortcuts/registry.ts   voice.toggle（默认 Ctrl+M，面板作用域）
```

结果链路：`voice://transcript {text, source}` → 前端按 `source`（`palette` / `selftest`）分发：面板追加进 query（非空补一个空格）即触发既有检索；自检回显文本与耗时。

## 4. Rust 语音域契约（`src-tauri/src/voice/`）

- **单飞会话**：同一时刻最多一路录音；重复 `voice_start` 返回错误码 `busy`。
- **采集**（`capture.rs`）：cpal 默认输入设备，目标 16 000 Hz / 单声道 / i16（设备不支持时取默认配置并在编码时重采样规格注明）；样本累积于 `Arc<Mutex<Vec<i16>>>`；`MAX_RECORDING_SECS = 60` 到时自动走 stop 路径。采集失败错误码：`no-device` / `permission`。
- **引擎**（`engine.rs`）：
  - 设置读取：`power.sttBaseUrl / power.sttModel / power.sttApiKey`，任一为空返回 `not-configured`。
  - multipart 手工构造（ureq 3 无内置 multipart）：boundary + `model` 字段 + `file`（`audio.wav`，`audio/wav`）+ WAV 字节；构造函数做成纯函数以便单测（`build_multipart(boundary, model, wav) -> Vec<u8>`）。
  - 超时：转写请求 30s；错误映射照 `map_ureq_error` 惯例返回可 i18n 的字符串码：`network` / `unauthorized` / `server`。
  - 有效采样不足 0.3s 返回 `empty`。
- **事件**：`voice://state`（`recording|transcribing|idle`）、`voice://transcript`（`{ text, source, elapsedMs }`）、`voice://error`（`{ code, source }`）。
- **依赖**：`cpal`、`hound` 均为 optional，挂到 `ai-addon` feature；Core flavor 依赖面不变（过 `test:deps-surface`）。
- **macOS**（后续移植）：打包声明 `NSMicrophoneUsageDescription`；Windows 无运行时弹窗，权限语义为「检测 + 引导隐私设置」。

## 5. 设置（settings.json power 段 + 搜索设置面板）

| 键 | 类型/默认 | 说明 |
|---|---|---|
| `power.voiceSearchAutoStart` | bool，`false` | 语音搜索：开启后调起搜索面板即自动录音 |
| `power.sttBaseUrl` | string，`""` | OpenAI 兼容转写服务地址（如 `https://api.siliconflow.cn/v1`） |
| `power.sttModel` | string，`""` | 模型名（如 `Fun-AudioLLM/SenseVoiceSmall`、`whisper-1`） |
| `power.sttApiKey` | string，`""` | API key，明文存 settings.json（与 ai-config.json apiKey、knowledge token 先例一致） |

- Schema 进 `settings/defaults.rs`（power tier），前端经既有 `settings_get/set` IPC 读写，无新 IPC。
- UI 进 `SearchSettingsPanel.vue` 新「语音输入」组（照 knowledge URL/Token 编辑模式）：三个字段 + 语音搜索开关 + 常驻「测试」按钮；`settingsUi.ts` 归入搜索分组。
- **语音自检**：开启「语音搜索」开关时自动弹起一次试录（走 `voice_selftest`，可跳过）；「测试」按钮随时可用。试录 = 录音（手动停止）→ 完整链路识别 → 回显文本与耗时；失败按错误码显示指引（`not-configured` 引导填写配置、`permission` 引导 Windows 隐私设置、`network`/`unauthorized`/`server` 显示对应文案）。
- 未配置（baseUrl/key/model 有空）时：自动录音静默跳过（不开 toast 打扰），麦克风按钮点击给出 `not-configured` toast 引导进设置。

## 6. 搜索面板接入（`CommandPanel.vue` + `src/state/voiceInput.ts`）

- **状态机**：`idle → recording → transcribing → idle`；`error` 态经 toast（error 8000ms 惯例）后回 `idle`。
- **触发**：① 面板打开且「语音搜索」开启且已配置 → 自动 `voice_start`；② 麦克风按钮（输入框右侧，AI flavor 才渲染）；③ `voice.toggle` 快捷键（registry 注册，默认 **Ctrl+M**，可改键；handler 以 `commandPanelOpen` 为门，面板未开不响应）。录音中任一触发即 `voice_stop`。
- **录音中呈现**：按钮红色脉冲，placeholder 切「正在聆听…」；转写中切「识别中…」。
- **Esc**：录音/转写中先 `voice_cancel`（不出文本、不关面板）；空闲态保持既有关闭行为。
- **回填**：`voice://transcript(source=palette)` → `query = query ? query + " " + text : text`；检索由既有 `watch(query)` 自动发生，无新增分支。
- **flavor 门控**：`src/config/flavor.ts`（`VITE_AFM_AI_ADDON`）为 false 时，麦克风按钮、自动录音、快捷键 handler、设置组全部不出现。

## 7. i18n（5 locale 全补，过 `test:i18n`）

- `settings.fields.power.voiceSearchAutoStart|sttBaseUrl|sttModel|sttApiKey.title/.description`
- `shortcuts.items.voice.toggle`（照 registry `labelKey` 惯例）
- `voice.palette.listening` / `voice.palette.transcribing`
- `voice.error.notConfigured / noDevice / permission / empty / network / unauthorized / server / busy`
- 自检文案：`voice.selftest.*`（按钮、引导、结果标签）

## 8. 分阶段任务

### Phase 1 — Rust 语音域骨架

- `Cargo.toml`：`cpal`、`hound` optional 挂 `ai-addon` feature。
- 新增 `src-tauri/src/voice/{mod,capture,engine}.rs`：四命令 + 三事件；`#[cfg(feature = "ai-addon")]` 门控并注册进 `lib.rs`。
- 单测：`build_multipart` 纯函数（boundary/字段/文件字节序）、WAV 编码、错误码映射。
- 验收：`npm run test:rust` 通过；Core flavor 构建产物不含 cpal/hound。

### Phase 2 — 前端状态与客户端

- 新增 `src/voice/client.ts`（typed invoke + listen）与 `src/state/voiceInput.ts`（状态机、事件接线、`source` 分发、错误 toast）。
- 验收：`tauri:dev` 下手动 invoke 可走通 录音→停止→回显。

### Phase 3 — 设置与语音自检

- `settings/defaults.rs` 四键（power tier）；`SearchSettingsPanel.vue` 语音输入组（三字段 + 开关 + 测试按钮）；`settingsUi.ts` 分组；开启开关自动弹起自检。
- 5 locale 文案全补。
- 验收：配置读写生效；自检回显文本/耗时；各错误码显示对应指引。

### Phase 4 — 面板接入

- `CommandPanel.vue`：麦克风按钮、录音/转写呈现、Esc 取消、回填追加。
- `shortcuts/registry.ts` 注册 `voice.toggle`（默认 Ctrl+M）+ AppShell handler（面板门控）；flavor 门控（Core 无语音 UI）。
- 验收：完整链路（按钮/快捷键/自动录音三条入口）走通；回填即时检索；Core flavor 无按钮无快捷键副作用。

### Phase 5 — 门禁与收尾

- 新增 `scripts/check-voice-input.mjs` + `package.json` `test:voice-input` 并进 `test` 聚合。覆盖：状态机转换与单飞、回填追加（空格规则）、自动录音门控（配置 × flavor × 面板开）、设置键存在于 defaults、i18n 键 ×5、快捷键注册与 labelKey、Core flavor 门控静态断言。
- 交接文档 `docs/a-file-manager-voice-input-handoff.md`（照搜索面板 handoff 格式）。
- 验收：`npm run test` 全绿（含新增门禁）；`test:deps-surface` 通过。

## 9. 真机验证清单（`npm run tauri:dev`，AI flavor）

- [ ] Ctrl+Shift+P 开面板 → Ctrl+M 录音 → 再按停止 → 文本追加进输入框并即时出结果。
- [ ] 「语音搜索」开启后开面板自动录音；关闭后不自动录。
- [ ] Esc 录音中只取消不出文本，面板保持打开；空闲态 Esc 照旧关面板。
- [ ] 60s 自动截止并照常出文本。
- [ ] 设置里「测试」按钮回显识别文本与耗时；断网 / 错误 key 分别出 `network` / `unauthorized` 文案。
- [ ] 未配置时点麦克风按钮出 `notConfigured` toast 引导；自动录音静默跳过。
- [ ] `/settings/shortcuts` 中 `voice.toggle` 可见且可改键，tooltip/aria 正确。
- [ ] Core flavor 构建：无麦克风按钮、无语音设置组、快捷键不响应。

## 10. 留接口不实现（v2 候选）

- 端侧引擎：Windows WinRT `Windows.Media.SpeechRecognizer`，插入同一语音服务接口，不动接入点。
- 系统级全局热键（Tauri global-shortcut）：任意桌面场景调起面板并自动录音。
- 流式 WS 协议（实时出字）、`language` 参数、面板外其他接入点（套件组件化后逐个挂）。
