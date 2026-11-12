# Feature Pack v1 仅本机安装

Feature Pack 第一版通过本机选择 zip/目录安装，由 ExtensionHost（`/extensions`）校验清单与权限后启用；不做远程目录/商店。清单声明兼容的工具管理网关版本范围，不匹配则拒绝盲调。连接偏好存 Pack 私有配置；token 沿用网关机制，不进 Core settings / AI 中央配置。Capability 参数表单 v1 仅覆盖 string/number/boolean/enum，其余走原始 JSON。

**Status**: accepted
