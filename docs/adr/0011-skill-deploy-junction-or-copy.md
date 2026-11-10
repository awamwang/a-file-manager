# Skill 部署默认 Junction，可选复制

Library Skill 部署默认使用目录 Junction（链接中央仓库，不复制）。AI 配置中提供全局「Skill 部署方式」偏好，可改为复制；之后所有 Skill 部署遵循该偏好。目标路径若已存在且非 AFM 部署产物则拒绝（TARGET_CONFLICT）；已是 AFM 产物则允许覆盖更新。

Junction 保证中央更新对已部署 Agent 立即可见且省空间；复制满足需要独立副本、或 Junction 不可用场景。不做单次弹窗选择，避免同一偏好在多次部署间漂移。
