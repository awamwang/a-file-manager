#!/bin/bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# macOS 无 Developer ID 发布路径的一键安装脚本。
#
# Gatekeeper 会拦截任何带 com.apple.quarantine 的未公证 .app；macOS 15+
# 又移除了 Finder 右键 →「打开」的单次放行入口。这个脚本在 App 落盘后
# 主动清掉隔离属性，因此安装完成后双击即可使用，不需要 Developer ID。
#
# 脚本本身没有可执行签名，请用终端执行（不要双击）：
#   bash /path/to/安装-A-File-Manager.sh
# 从终端用 bash 读取脚本文本不会触发 Gatekeeper 的脚本签名校验。
#
# Usage:
#   bash scripts/install-macos.sh [--app "/path/to/A File Manager.app"] \
#                                 [--apps-dir /Applications] [--no-open]

set -euo pipefail

APP_NAME="A File Manager.app"
BIN_NAME="a-file-manager"
APPS_DIR="/Applications"
SOURCE_APP=""
NO_OPEN=0

usage() {
  sed -n '2,16p' "$0" | sed 's/^# \{0,1\}//'
}

die() {
  printf '错误: %s\n' "$*" >&2
  exit 1
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --app)
      [ "$#" -ge 2 ] || die "--app 需要一个路径参数"
      SOURCE_APP="$2"
      shift 2
      ;;
    --apps-dir)
      [ "$#" -ge 2 ] || die "--apps-dir 需要一个目录参数"
      APPS_DIR="$2"
      shift 2
      ;;
    --no-open)
      NO_OPEN=1
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      die "未知参数: $1"
      ;;
  esac
done

if [ -z "${SOURCE_APP}" ]; then
  SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
  SOURCE_APP="${SCRIPT_DIR}/${APP_NAME}"
fi

if [ ! -d "${SOURCE_APP}" ]; then
  die "找不到 App: ${SOURCE_APP}。请把脚本与 A File Manager.app 放在同一目录（推荐从 DMG 根目录运行），或用 --app 指定 App 路径。"
fi
[ -x "${SOURCE_APP}/Contents/MacOS/${BIN_NAME}" ] || die "包内可执行文件缺失或不可执行: ${SOURCE_APP}"

if ! command -v codesign >/dev/null 2>&1; then
  die "需要 macOS 的 codesign 工具"
fi

# 只接受 ad-hoc / Developer ID 签名有效的包；损坏的包不要装。
if ! codesign --verify --deep --strict "${SOURCE_APP}" >/dev/null 2>&1; then
  die "App 签名校验失败，请从官方 Release 重新下载: ${SOURCE_APP}"
fi

mkdir -p "${APPS_DIR}" || die "无法创建安装目录: ${APPS_DIR}"
[ -w "${APPS_DIR}" ] || die "没有写入 ${APPS_DIR} 的权限；请在终端执行 sudo bash \"$0\""

DEST="${APPS_DIR}/${APP_NAME}"

# 旧版本移入废纸篓而不是直接删除：安装失败时仍可恢复。
if [ -e "${DEST}" ]; then
  STAMP="$(date +%Y%m%d-%H%M%S)"
  TRASH_DIR="${HOME}/.Trash"
  mkdir -p "${TRASH_DIR}"
  TRASH_DEST="${TRASH_DIR}/A File Manager ${STAMP}.app"
  mv "${DEST}" "${TRASH_DEST}" || die "无法移走旧版本: ${DEST}"
  printf '旧版本已移入废纸篓: %s\n' "${TRASH_DEST}"
fi

if ! command -v ditto >/dev/null 2>&1; then
  die "需要 macOS 的 ditto 工具"
fi
ditto "${SOURCE_APP}" "${DEST}" || die "复制失败: ${DEST}"

# 关键步骤：ditto 会从隔离的源继承 quarantine，落盘后必须清掉。
# provenance 是 macOS 15 的受系统保护属性，能不能删取决于系统版本；
# 真正触发 Gatekeeper 的是 quarantine，删不掉 provenance 不影响启动。
xattr -dr com.apple.quarantine "${DEST}" 2>/dev/null || true
xattr -dr com.apple.provenance "${DEST}" 2>/dev/null || true

if [ -n "$(xattr -p com.apple.quarantine "${DEST}" 2>/dev/null || true)" ]; then
  die "隔离属性未能清除，请手动执行: xattr -dr com.apple.quarantine \"${DEST}\""
fi
codesign --verify --deep --strict "${DEST}" >/dev/null 2>&1 || die "复制后签名校验失败: ${DEST}"

printf '安装完成: %s\n' "${DEST}"
printf '已清除下载隔离属性，之后双击即可打开，无需系统设置或右键放行。\n'

if [ "${NO_OPEN}" -eq 0 ]; then
  open "${DEST}"
  printf '已启动 A File Manager。\n'
fi
