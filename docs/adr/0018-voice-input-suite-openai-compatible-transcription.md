# 语音输入套件：OpenAI 兼容转写协议（v1 只做云端批量）

AFM 需要全应用可用的「说话变文字」能力，第一个接入点是搜索面板（麦克风按钮 + 面板内快捷键 + 「语音搜索」自动录音配置）。桌面端 STT 没有统一标准：流式 WebSocket 全是厂商私有协议，逐一适配成本高、锁定深。

决定 v1 只预制 **OpenAI 兼容批量转写协议**（`POST {baseUrl}/v1/audio/transcriptions`，multipart，返回 `{text}`）：一个适配器覆盖 OpenAI / Groq / SiliconFlow 等所有兼容服务商。链路全在 Rust（cpal 采集 → 手写 44 字节头编 WAV → ureq 手工构造 multipart 上传 → 文本事件回前端），前端零音频知识。不做端侧引擎——Windows WinRT `Windows.Media.SpeechRecognizer` 留作后续引擎候选，将来走同一语音服务接口插入，不动接入点。套件随 **AI 包 flavor**（cargo feature `ai-addon` + `VITE_AFM_AI_ADDON`）分发：AI 包规划转为正式主包（名称即普通正式版）、核心包转为精简版，语音输入属于主包能力。

## Consequences

- 配置走 settings.json 的 power 段设置键 + 搜索设置面板编辑（照 knowledge 服务 URL/Token 先例），含语音自检：开启语音搜索时弹起试录，另有常驻测试按钮。
- 识别文本**追加**进搜索面板 query，跟随面板既有「输入即搜」；不引入「识别后自动搜索」开关。
- v1 仅应用内快捷键（快捷键注册表注册、面板作用域生效）；系统级全局热键、流式协议、端侧引擎、更多接入点均留接口不实现。
- 新增 Rust 依赖 cpal（WAV 头与 multipart 手写，不引 hound/reqwest）；需通过 `test:deps-surface` 依赖面检查。
