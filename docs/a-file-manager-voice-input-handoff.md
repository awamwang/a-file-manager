# 语音输入套件（Voice Input Suite）交接文档

对应决策：`docs/adr/0018-voice-input-suite-openai-compatible-transcription.md`；
实施计划：`docs/a-file-manager-voice-input-implementation-plan.md`；
术语表：`docs/glossary-voice-input.md`。
本文档面向后续维护者，汇总已交付内容、架构地图、行为契约与验证清单。

## 1. 交付概览

| 阶段 | 内容 | 提交 |
| --- | --- | --- |
| Phase 1 | Rust 语音服务：cpal 采集（独立线程持有 Stream）→ 线性重采样 16k mono → 手写 WAV 头 → ureq 手工 multipart 上传；`voice_start/stop/cancel/selftest` 四命令 + `voice://*` 三事件；全部挂 `ai-addon` feature（cpal optional） | 见 git log |
| Phase 2 | 前端 typed IPC 客户端（`src/voice/client.ts`）+ 状态机（`src/state/voiceInput.ts`）：source 分发（palette/selftest）、palette 错误 toast、selftest 内联回显 | 见 git log |
| Phase 3 | 设置：schema v12→v13 迁移 + `power.voiceSearchAutoStart / power.sttBaseUrl / power.sttModel / power.sttApiKey` 四键；SearchSettingsPanel 语音配置 + 常驻自检；i18n 5 locale 全键 | 见 git log |
| Phase 4 | 搜索面板接入：麦克风按钮（录音中红色脉冲）、Ctrl+M 快捷键、Esc 取消、转写文本追加进 query、`power.voiceSearchAutoStart` 开面板自动录音、关面板断会话 | 见 git log |
| Phase 5 | 测试门禁（`test:voice-input`）与本文档 | 见 git log |

## 2. 架构地图

```
src-tauri/src/voice/
  mod.rs         会话状态机 + 四命令 + VoiceError（稳定错误码）+ 事件常量 voice://state|transcript|error
  capture.rs     cpal 采集：独立线程持有 Stream（Stream 不进静态会话），通道控制 start/stop
  engine.rs      read_config（每次请求前现读 settings.json 的 power.stt* 三键）
                 normalize_base_url / resample_to_16k_mono / encode_wav（手写 44 字节头）
                 build_multipart（ureq 3 无 multipart，手工构造）/ transcribe（POST {base}/audio/transcriptions）
src-tauri/src/settings/           schema v13：defaults.rs 四键 + migrate.rs (12,13) 增量步 + types.rs 版本
src/voice/client.ts               typed invoke 封装 + 事件常量 + Voice*Event 负载类型
src/state/voiceInput.ts           前端状态机：voicePhase/voiceSource、paletteVoiceActive、
                                 onPaletteTranscript 订阅表、selftest 独立 ref 回显、
                                 voiceErrorMessageKey（错误码 → i18n 键）
src/components/command/CommandPanel.vue
                                 接入点：麦克风按钮 / maybeAutoStartVoice / appendPaletteTranscript
                                 / Esc 取消 / 关面板 cancelVoice / placeholder 切换
src/components/settings/SearchSettingsPanel.vue
                                 语音配置区（data-voice-settings）+ 常驻自检按钮 + 权限提示
src/shortcuts/registry.ts         commandPanel.voiceToggle（Ctrl+M，allowInEditable）
src/components/shell/AppShell.vue 全局 handler：commandPanelOpen 门控 + AI_ADDON_ENABLED 门控
scripts/check-voice-input.mjs     跨层接线门禁（test:voice-input）
```

## 3. 行为契约（改动前先读）

1. **单飞会话**：同一时刻最多一路录音；`voice_start` 在会话进行中同步拒绝（`busy`）。
   selftest 与 palette 互斥，共用同一条 Rust 会话链路。
2. **阶段不可逆收尾**：`transcribing` 阶段会话已被上传线程消费（脱管），stop/cancel 均为静默
   no-op——前端 `togglePaletteVoice` / `stopSelfTest` 在该阶段不动作，等待结果事件自然落地。
3. **错误分发**：命令同步失败走 invoke 拒绝（前端 toast / selftest 内联回显）；上传等异步失败走
   `voice://error` 事件。错误码集固定为
   `busy / no-device / permission / not-configured / empty / no-audio / no-speech / network / unauthorized / server`，
   前端 `voiceErrorMessageKey` 一一映射 i18n 键；新增错误码必须同步 5 locale。
   场景区分：`permission` = 开流前注册表预检 ConsentStore Deny（未授权，附设置路径）；
   `no-device` = 无输入设备；`no-audio` = 开流成功但 1.5s 零采样死流（看门狗原子 take）；
   `empty` = 误触过短（<0.3s）；`no-speech` = 录到音频但转写结果为空（多为麦克风静音）。
4. **转写追加不替换**：palette 转写文本**追加**进 query（非空补一个空格），不自动触发搜索；
   selftest 结果只内联回显。
5. **面板生命周期**：`power.voiceSearchAutoStart` 为真 → 面板打开自动起录；面板关闭（含 Esc、
   backdrop、runItem）→ 录音中 `cancelVoice` 断会话。录音中 Esc 先取消语音、不关面板；
   识别中 Esc 直接关面板（上传已脱管）。
6. **自动截止**：录音 60s 自动 finalize 进转写；<0.3s 返回 `empty`。
7. **Core 包静默**：cpal 与 voice 命令全部挂 `ai-addon` feature；`VITE_AFM_AI_ADDON=0` 时
   `AI_ADDON_ENABLED=false`，麦克风按钮、语音设置区、快捷键 handler 全部静默不渲染/不动作，
   `initVoiceInput` 空操作。
8. **配置即存**：四键存 settings.json power 段；引擎每次请求前现读（改配置无需重启）；
   UI 在 SearchSettingsPanel「语音输入」区（常规 Settings 页不出现，键在
   `SETTINGS_UI_HIDDEN_KEYS`，按 awamKnowledge 先例归 "advanced" 组）。
9. **Windows 无运行时权限弹窗**：`permission` / `no-device` 错误在自检区显示
   `voice.selftest.permissionHint` 引导用户查系统隐私设置；macOS 移植时需补
   `NSMicrophoneUsageDescription`。
10. **命名不变量**：快捷键 id `commandPanel.voiceToggle`、事件名 `voice://*`、错误码集、
    设置键 `power.stt*` 不变；i18n 顶层 `voice.*` 命名空间独立于 `settings.search.voice*`。

## 4. 测试门禁

- `npm run test:voice-input`：跨层接线断言——Rust feature/命令/事件/错误码、schema v13 与迁移、
  typed client、状态机分发、面板接入（mic 按钮 / 自动录音 / Esc / 转写追加）、快捷键注册、
  设置 UI 隐藏键、5 locale 语音键存在性。
- 相关回归：`test:shortcuts`（registry 完整性 + 冲突检测）、`test:fluent-icons`（mic glyph）、
  `test:settings-ui-regroup`、`test:i18n`、`npm run test:rust`（`cargo test voice` 11 用例）。

## 5. 真机验证清单（`npm run dev:ai`）

- [ ] 设置 → 搜索 → 「语音输入」：填 baseUrl / model / apiKey 保存；开启「自动录音」触发自检试录。
- [ ] Ctrl+Shift+P 打开面板：麦克风按钮出现；点击（或 Ctrl+M）开始录音，按钮红色脉冲、
      placeholder 切「正在聆听」；再按停止转写，placeholder 切「正在识别」，结果追加进搜索框。
- [ ] 录音中 Esc 只取消语音不关面板；再按 Esc 才关面板。
- [ ] 「自动录音」开启时打开面板自动起录；关闭面板后录音停止（无悬空会话，再次起录不报 busy）。
- [ ] 断网 / 错误 apiKey / 空白录音分别得到 network / unauthorized / empty toast（palette）或
      内联回显（自检）。
- [ ] `npm run dev:core`：麦克风按钮不出现，设置无语音区，Ctrl+M 无动作。
