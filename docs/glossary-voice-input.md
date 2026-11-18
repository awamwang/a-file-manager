# 语音输入套件（Voice Input Suite）

给 AFM 全应用提供「说话变文字」的通用能力：对内暴露统一语音服务，对外预制标准 STT 协议适配；各界面场景经接入点调用，不直接触达具体 STT API。多上下文索引见 [GLOSSARY-MAP.md](../GLOSSARY-MAP.md)。

## Language

**语音输入套件（Voice Input Suite）**：
应用级「说话转文字」能力套件：统一语音服务 + 可配置引擎 + 各界面接入点。
_Avoid_: 语音插件、ASR 模块、识别器

**语音服务（Voice Service）**：
套件对内的统一服务协议（开始 / 停止 / 订阅识别结果与错误）；接入点只与它对话，引擎与网络协议被隔离在其后。
_Avoid_: STT API、引擎接口

**接入点（Integration Point）**：
调用语音服务的一个界面场景；当前唯一接入点是搜索面板。
_Avoid_: 入口（易与结果主动作混淆）、插件点

**引擎（Engine）**：
语音服务背后实际执行识别的实现；同一服务协议下可配置切换。
_Avoid_: 搜索源（那是搜索面板的概念）、后端

**云端引擎（Cloud Engine）**：
经 HTTP 调用外部 STT 服务的引擎；服务地址、模型与密钥均可配置。
_Avoid_: 服务器、在线识别

**OpenAI 兼容转写协议（OpenAI-compatible Transcription Protocol）**：
`POST {baseUrl}/v1/audio/transcriptions` 形态的批量语音转写协议；OpenAI、Groq、SiliconFlow 等均实现。套件唯一预制的对外协议。
_Avoid_: Whisper 协议（Whisper 只是模型之一）、流式协议

**语音搜索（Voice Search）**：
搜索面板的配置与行为：开启后，调起搜索面板即自动开始录音；麦克风权限在开启该项时申请。
_Avoid_: 语音助手、听写模式

**语音自检（Voice Self-Test）**：
设置内的端到端试录：录一段话走完整识别链路后回显识别文本（或错误与指引），用于验证麦克风与引擎配置；开启语音搜索时可弹起，另有常驻测试按钮。
_Avoid_: 试听、诊断、权限检查
