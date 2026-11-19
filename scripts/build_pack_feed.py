#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""AFM 集成方构建（ADR 0022）：从依赖仓源码 in-tree 构建 sidecar 产物 + 生成 feed 元清单。

这是「AFM 作为 integrator 自构建、自发布」的构建期工具：把一个能力后端的源码冻结成
**自包含单文件产物（PyInstaller onefile，runtime:self）**，计算 SHA-256，生成 AFM 自有
发布通道的 store 元清单（schema 与 ``store.rs::StoreManifest`` 对齐），并把产物 + 清单落进
一个 feed 目录（本地 http 服务即可喂给 AFM 的 provision 接缝）。

用法：
    python build_pack_feed.py build \\
        --pack-id awam.tool-runtime \\
        --name awam-tool-manager \\
        --src "G:/Projects/19AI/skills/awam-tool-manager" \\
        --entry src/server.py \\
        --version 0.1.0 \\
        --out "G:/.../afm-feed" \\
        --base-feed-url http://127.0.0.1:8078

    python build_pack_feed.py emit-feed-config \\
        --out "G:/.../afm-feed" --app-config-dir "<AFM app_config_dir>"

约束（ADR 0022）：产物冻结不可跨平台，须在目标 OS 上构建；本工具单 OS。
"""
from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


def sha256_of(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def pyinstaller_onefile(src: Path, entry: str, name: str, work: Path, python: str) -> Path:
    """用 PyInstaller 把 src/<entry> 冻结成单文件 exe，返回产物路径。"""
    entry_path = (src / entry).resolve()
    if not entry_path.exists():
        raise SystemExit(f"entry not found: {entry_path}")
    src_dir = entry_path.parent
    dist = work / "dist"
    cmd = [
        python, "-m", "PyInstaller",
        "--noconfirm", "--clean", "--onefile",
        "--name", name,
        "--distpath", str(dist),
        "--workpath", str(work / "build"),
        "--specpath", str(work),
        "--paths", str(src_dir),
        # 动态导入的第三方运行时需显式收集，否则冻结后 import 期缺模块。
        "--collect-all", "mcp",
        "--collect-all", "uvicorn",
        "--collect-submodules", "starlette",
        "--collect-submodules", "anyio",
        str(entry_path),
    ]
    # 把声明数据目录 data/（含 tools/*.json）打进冻结产物，供 server.py 在 frozen
    # 下从 _MEIPASS/data 读取——否则托管实例 list_tools 为空（与源码 config-mode 不一致）。
    data_dir = src / "data"
    if data_dir.exists():
        sep = ";" if sys.platform.startswith("win") else ":"
        cmd[-1:-1] = ["--add-data", f"{data_dir}{sep}data"]
    print("[build] running:", " ".join(cmd), flush=True)
    subprocess.run(cmd, check=True)
    exe = dist / (name + (".exe" if sys.platform.startswith("win") else ""))
    if not exe.exists():
        raise SystemExit(f"PyInstaller did not produce {exe}")
    return exe


def build(args: argparse.Namespace) -> int:
    src = Path(args.src)
    out = Path(args.out)
    artifacts = out / "artifacts"
    artifacts.mkdir(parents=True, exist_ok=True)

    work = Path(tempfile.mkdtemp(prefix="afm-pack-build-"))
    try:
        exe = pyinstaller_onefile(src, args.entry, args.name, work, args.python)
        dest = artifacts / exe.name
        shutil.copy2(exe, dest)
    finally:
        shutil.rmtree(work, ignore_errors=True)

    digest = sha256_of(dest)
    asset_url = f"{args.base_feed_url.rstrip('/')}/artifacts/{exe.name}"
    entry = {
        "packId": args.pack_id,
        "artifactVersion": args.version,
        "assetUrl": asset_url,
        "sha256": digest,
        "fileName": exe.name,
        "baseUrl": args.base_probe_url,
    }
    # 可选：对象存储镜像回落地址（GitHub 主源不可达时同 SHA 下载）。未配则不写、不回落。
    if getattr(args, "fallback_base_url", ""):
        entry["assetFallbackUrl"] = f"{args.fallback_base_url.rstrip('/')}/artifacts/{exe.name}"
    manifest = {"schema": 1, "entries": [entry], "packs": []}
    manifest_path = out / "manifest.json"
    # 合并：entries 按 packId 去重覆盖本条；packs（可安装列表）整段保留。
    if manifest_path.exists():
        try:
            existing = json.loads(manifest_path.read_text(encoding="utf-8"))
            others = [e for e in existing.get("entries", []) if e.get("packId") != args.pack_id]
            manifest["entries"] = others + [entry]
            manifest["packs"] = existing.get("packs", [])
        except Exception:
            pass
    manifest_path.write_text(json.dumps(manifest, indent=2), encoding="utf-8")

    print(f"[build] artifact : {dest}  ({dest.stat().st_size} bytes)")
    print(f"[build] sha256   : {digest}")
    print(f"[build] manifest : {manifest_path}")
    return 0


def zip_pack_dir(src_dir: Path, dest_zip: Path) -> None:
    """把一个 pack 目录（含 pack.json）打成 zip，相对路径入包根。"""
    import zipfile

    dest_zip.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(dest_zip, "w", zipfile.ZIP_DEFLATED) as zf:
        for p in sorted(src_dir.rglob("*")):
            if p.is_file():
                zf.write(p, p.relative_to(src_dir).as_posix())


def pack_dir(args: argparse.Namespace) -> int:
    """把成形 pack 目录打成安装包 zip + 计算 SHA + 写入 feed 元清单的 packs 段（可安装 Store）。"""
    src = Path(args.src)
    if not (src / "pack.json").exists():
        raise SystemExit(f"pack.json not found in {src}")
    out = Path(args.out)
    zp = out / "packs" / (args.pack_id + ".zip")
    zip_pack_dir(src, zp)
    digest = sha256_of(zp)
    install_url = f"{args.base_feed_url.rstrip('/')}/packs/{args.pack_id}.zip"
    entry = {
        "packId": args.pack_id,
        "name": args.name,
        "version": args.version,
        "installUrl": install_url,
        "sha256": digest,
    }
    if getattr(args, "fallback_base_url", ""):
        entry["installFallbackUrl"] = f"{args.fallback_base_url.rstrip('/')}/packs/{args.pack_id}.zip"
    manifest_path = out / "manifest.json"
    manifest: dict = {"schema": 1, "entries": [], "packs": []}
    if manifest_path.exists():
        try:
            existing = json.loads(manifest_path.read_text(encoding="utf-8"))
            manifest["entries"] = existing.get("entries", [])
            others = [e for e in existing.get("packs", []) if e.get("packId") != args.pack_id]
            manifest["packs"] = others + [entry]
        except Exception:
            manifest["packs"] = [entry]
    else:
        manifest["packs"] = [entry]
    manifest_path.write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    print(f"[pack] zip      : {zp}  ({zp.stat().st_size} bytes)")
    print(f"[pack] sha256   : {digest}")
    print(f"[pack] installUrl: {install_url}")
    print(f"[pack] manifest : {manifest_path}")
    return 0


def emit_feed_config(args: argparse.Namespace) -> int:
    """生成 AFM 运行时 feed 配置（managed-dependency-feed.json），指向本地 feed。"""
    out = Path(args.out)
    app_cfg = Path(args.app_config_dir)
    app_cfg.mkdir(parents=True, exist_ok=True)
    cfg = {
        "manifestUrl": f"{args.base_feed_url.rstrip('/')}/manifest.json",
        "allowHosts": args.allow_host,
    }
    # 可选：对象存储镜像回落清单地址。未配则不写（仅单源 feed 行为不变）。
    # 注意：回落主机也须出现在 --allow-host 中，否则运行时会按 allow-list 跳过该源。
    if getattr(args, "fallback_base_feed_url", ""):
        cfg["fallbackManifestUrl"] = f"{args.fallback_base_feed_url.rstrip('/')}/manifest.json"
    dest = app_cfg / "managed-dependency-feed.json"
    dest.write_text(json.dumps(cfg, indent=2), encoding="utf-8")
    print(f"[feed] wrote {dest}: {json.dumps(cfg)}")
    return 0


def main() -> int:
    p = argparse.ArgumentParser(description="AFM in-tree pack/feed builder (ADR 0022)")
    sub = p.add_subparsers(dest="cmd", required=True)

    b = sub.add_parser("build", help="freeze a backend to a single-file artifact + manifest entry")
    b.add_argument("--pack-id", required=True)
    b.add_argument("--name", required=True, help="artifact base name (no extension)")
    b.add_argument("--src", required=True, help="backend repo root")
    b.add_argument("--entry", default="src/server.py", help="entry relative to --src")
    b.add_argument("--version", required=True)
    b.add_argument("--out", required=True, help="feed output dir")
    b.add_argument("--base-feed-url", default="http://127.0.0.1:8078")
    b.add_argument("--base-probe-url", default="http://127.0.0.1:18778",
                   help="AFM probe/attach base url written to the manifest entry")
    b.add_argument("--python", default=sys.executable, help="interpreter with PyInstaller + deps")
    b.add_argument("--fallback-base-url", default="",
                   help="object-storage mirror base for assetFallbackUrl (empty = no fallback)")
    b.set_defaults(fn=build)

    e = sub.add_parser("emit-feed-config", help="write AFM managed-dependency-feed.json")
    e.add_argument("--out", required=True)
    e.add_argument("--app-config-dir", required=True)
    e.add_argument("--base-feed-url", default="http://127.0.0.1:8078")
    e.add_argument("--fallback-base-feed-url", default="",
                   help="object-storage mirror base for fallbackManifestUrl (empty = no fallback)")
    e.add_argument("--allow-host", action="append", default=["127.0.0.1"])
    e.set_defaults(fn=emit_feed_config)

    pd = sub.add_parser("pack-dir", help="zip a finished pack dir + record it as installable (packs)")
    pd.add_argument("--pack-id", required=True)
    pd.add_argument("--name", required=True)
    pd.add_argument("--version", required=True)
    pd.add_argument("--src", required=True, help="pack directory containing pack.json")
    pd.add_argument("--out", required=True, help="feed output dir")
    pd.add_argument("--base-feed-url", default="http://127.0.0.1:8078")
    pd.add_argument("--fallback-base-url", default="",
                    help="object-storage mirror base for installFallbackUrl (empty = no fallback)")
    pd.set_defaults(fn=pack_dir)

    args = p.parse_args()
    return args.fn(args)


if __name__ == "__main__":
    raise SystemExit(main())
