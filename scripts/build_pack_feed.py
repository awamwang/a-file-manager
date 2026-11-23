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

Mac 打包（每架构在对应 target 上各跑一次，产物写进同一 feed 目录/同一 manifest）：
    # Apple Silicon runner (aarch64)
    python build_pack_feed.py build --pack-id awam.tool-runtime \\
        --name awam-tool-manager --src <src> --entry src/server.py \\
        --version 0.1.0 --out <shared-feed> --base-feed-url <base> \\
        --platform macos-aarch64
    # Intel runner (x86_64) — 同一 out 目录，manifest 自动按 (packId, platform) 合并
    python build_pack_feed.py build ... --platform macos-x86_64
    # Windows 构建同样进来（windows-x86_64），三份产物共存于一份 feed。

`platform` 省略时按构建宿主自动派生（sys.platform + machine → Rust 风格的
`<os>-<arch>`）。客户端按 `platform::artifact_platform_key()` 精确选条目，
macOS 绝不会误下 Windows `.exe`。详见 docs/feed-operations-runbook.md。

CI 多平台聚合（发布链路）：每个平台 runner 产一个独立 feed slice（各自
manifest.json + artifacts/），再由聚合 job 合成唯一 manifest 上传 Release：
    python build_pack_feed.py merge-feed \\
        --slice <win-slice> --slice <mac-arm64-slice> --slice <mac-x64-slice> \\
        --out <final-feed>

同名产物若摘要不同会直接报错：Mac x86_64 的 --name 须加 -x64 后缀，
避免与 Apple Silicon 的同名无扩展名产物在扁平 Release 资产里互相覆盖。

约束（ADR 0022）：产物冻结不可跨平台，须在目标 OS 上构建；本工具单 OS。
"""
from __future__ import annotations

import argparse
import hashlib
import json
import platform as host_platform
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


def merge_documents(base: dict, incoming: dict) -> dict:
    """合并两份 feed 文档：entries 按 (packId, platform) 去重，packs 按 packId 去重。

    incoming 覆盖同键旧条目（重建场景），其它平台条目原样保留。旧 feed 无
    platform 字段时以 "" 参与比较，保持向后兼容。抽取为公共函数供 build /
    pack_dir / merge-feed 共用，避免三处去重语义漂移。
    """
    entries: dict[tuple[str, str], dict] = {}
    for e in base.get("entries", []):
        entries[(e.get("packId"), e.get("platform") or "")] = e
    packs: dict[str, dict] = {}
    for p in base.get("packs", []):
        packs[p.get("packId")] = p
    for e in incoming.get("entries", []):
        entries[(e.get("packId"), e.get("platform") or "")] = e
    for p in incoming.get("packs", []):
        packs[p.get("packId")] = p
    return {"schema": 1, "entries": list(entries.values()), "packs": list(packs.values())}


def artifact_platform_key(os_name: str | None = None, arch: str | None = None) -> str:
    """规范化宿主平台键 `<os>-<arch>`，与 Rust `platform::artifact_platform_key()` 对齐。

    Python 与 Rust 的架构命名不同：Python `arm64` ↔ Rust `aarch64`、
    Python `AMD64` ↔ Rust `x86_64`。这里统一到 Rust consts 的拼写，确保
    构建脚本写进 feed 的 `platform` 与客户端匹配逻辑是同一套键。
    """
    os_name = (os_name or sys.platform).lower()
    if os_name.startswith("win") or os_name == "windows":
        os_key = "windows"
    elif os_name.startswith("darwin") or os_name == "macos":
        os_key = "macos"
    elif os_name.startswith("linux"):
        os_key = "linux"
    else:
        os_key = os_name

    raw_arch = (arch or host_platform.machine()).lower()
    arch_map = {
        "arm64": "aarch64",
        "aarch64": "aarch64",
        "x86_64": "x86_64",
        "amd64": "x86_64",
        "x64": "x86_64",
    }
    arch_key = arch_map.get(raw_arch, raw_arch)
    return f"{os_key}-{arch_key}"


def sha256_of(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def asset_url(base: str, name: str, layout: str) -> str:
    """产物/安装包下载 URL。subdir=本地 feed 布局（<base>/artifacts/ 或 <base>/packs/）；
    flat=GitHub Release 扁平资产（<base>/<name>，对应 .../releases/download/<tag>/<name>）。"""
    b = base.rstrip("/")
    if layout == "flat":
        return f"{b}/{name}"
    return f"{b}/artifacts/{name}"


def pack_url(base: str, name: str, layout: str) -> str:
    b = base.rstrip("/")
    if layout == "flat":
        return f"{b}/{name}"
    return f"{b}/packs/{name}"


def pyinstaller_onefile(src: Path, entry: str, name: str, work: Path, python: str,
                        collect: list[str], paths: list[str], add_data: list[str]) -> Path:
    """用 PyInstaller 把 src/<entry> 冻结成单文件 exe，返回产物路径。

    collect: 额外 --collect-all 包名（包式后端如 awam_resource_gateway 需带）。
    paths:   额外 --paths（包式入口需把包根目录加进来，如 <src>/src）。
    add_data: 额外 --add-data（sep 自动按平台）。
    """
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
    ]
    sep = ";" if sys.platform.startswith("win") else ":"
    for extra_path in paths:
        cmd += ["--paths", str((src / extra_path).resolve() if not Path(extra_path).is_absolute() else extra_path)]
    for pkg in collect:
        cmd += ["--collect-all", pkg]
    alt_sep = ":" if sep == ";" else ";"
    for ad in add_data:
        # 形如 "src/data;data"。CI 命令行在 Win/Mac 之间共用，两种分隔符都接受；
        # 优先平台原生分隔符（Windows 盘符 C: 不能被冒号误切）。
        split_on = sep if sep in ad else alt_sep
        if split_on not in ad:
            raise SystemExit(f"--add-data must be 'local{sep}dest': {ad}")
        local, _, dest = ad.partition(split_on)
        local_path = Path(local)
        if not local_path.is_absolute():
            local_path = (src / local).resolve()
        cmd += ["--add-data", f"{local_path}{sep}{dest}"]
    # 声明数据目录 data/（含 tools/*.json 等）打进冻结产物，供 frozen 下从 _MEIPASS/data 读取。
    data_dir = src / "data"
    if data_dir.exists() and not add_data:
        # 必须传绝对路径：PyInstaller 的 --add-data 相对路径按 --specpath（临时目录）解析，
        # 而 src 常是相对的 upstream/<repo>，相对写法会让 PyInstaller 去 <temp>/upstream/<repo>/data
        # 找不到 data 目录而崩（CI 首次真跑即此因，见 build 失败 "Unable to find ...data"）。
        cmd += ["--add-data", f"{data_dir.resolve()}{sep}data"]
    cmd.append(str(entry_path))
    print("[build] running:", " ".join(cmd), flush=True)
    subprocess.run(cmd, check=True)
    exe = dist / (name + (".exe" if sys.platform.startswith("win") else ""))
    if not exe.exists():
        raise SystemExit(f"PyInstaller did not produce {exe}")
    return exe


def adhoc_sign_macos(path: Path) -> None:
    """给 macOS onefile 做 ad-hoc 签名（零凭证回退，与 AFM.app 同一策略）。

    不做 Developer ID、不做公证：这里只提供完整性印章，让系统把它识别为
    已签名（而非“身份不明的开发者”），用户仍按 runbook 的临时放行方式打开。
    Gatekeeper 的 quarantine 由下载端决定；AFM 自身下载落盘的 sidecar 不携带
    quarantine，因此正常自动拉起不受影响。
    """
    if sys.platform != "darwin":
        return
    subprocess.run(
        ["codesign", "--force", "--sign", "-", "--timestamp=none", str(path)],
        check=True,
    )
    verify = subprocess.run(
        ["codesign", "-dv", "--verbose=1", str(path)],
        capture_output=True,
        text=True,
    )
    details = (verify.stdout or "") + "\n" + (verify.stderr or "")
    if "Signature=adhoc" not in details:
        raise SystemExit(f"ad-hoc signature not applied to {path}: {details}")


def build(args: argparse.Namespace) -> int:
    src = Path(args.src)
    out = Path(args.out)
    artifacts = out / "artifacts"
    artifacts.mkdir(parents=True, exist_ok=True)
    # 平台键须在目标 OS 上构建时按宿主派生（ADR 0022：PyInstaller 不跨平台冻结）。
    # 允许 --platform 显式覆盖（交叉发布已构建好的产物场景），否则自动探测。
    platform_key = getattr(args, "platform", "") or artifact_platform_key()

    work = Path(tempfile.mkdtemp(prefix="afm-pack-build-"))
    try:
        exe = pyinstaller_onefile(
            src, args.entry, args.name, work, args.python,
            getattr(args, "collect", []) or [],
            getattr(args, "paths", []) or [],
            getattr(args, "add_data", []) or [],
        )
        dest = artifacts / exe.name
        shutil.copy2(exe, dest)
        # Mac 产物与 AFM 主程序一致：ad-hoc 签名即可，暂不做 Developer ID / 公证。
        if platform_key.startswith("macos-"):
            adhoc_sign_macos(dest)
    finally:
        shutil.rmtree(work, ignore_errors=True)

    digest = sha256_of(dest)
    layout = getattr(args, "url_layout", "subdir")
    asset_url_ = asset_url(args.base_feed_url, exe.name, layout)
    entry = {
        "packId": args.pack_id,
        "artifactVersion": args.version,
        "assetUrl": asset_url_,
        "sha256": digest,
        "fileName": exe.name,
        "baseUrl": args.base_probe_url,
        "platform": platform_key,
    }
    # 可选：对象存储镜像回落地址（GitHub 主源不可达时同 SHA 下载）。未配则不写、不回落。
    fb = getattr(args, "fallback_base_url", "")
    if fb:
        entry["assetFallbackUrl"] = asset_url(fb, exe.name, layout)
    manifest: dict = {"schema": 1, "entries": [entry], "packs": []}
    manifest_path = out / "manifest.json"
    # 合并：entries 按 (packId, platform) 去重，让同一 out 目录在多次构建后
    # 同时容纳 windows-x86_64 / macos-aarch64 / macos-x86_64 三份产物。
    if manifest_path.exists():
        try:
            manifest = merge_documents(
                json.loads(manifest_path.read_text(encoding="utf-8")), manifest
            )
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
    layout = getattr(args, "url_layout", "subdir")
    zip_name = f"{args.pack_id}.zip"
    install_url = pack_url(args.base_feed_url, zip_name, layout)
    entry = {
        "packId": args.pack_id,
        "name": args.name,
        "version": args.version,
        "installUrl": install_url,
        "sha256": digest,
    }
    fb = getattr(args, "fallback_base_url", "")
    if fb:
        entry["installFallbackUrl"] = pack_url(fb, zip_name, layout)
    manifest_path = out / "manifest.json"
    manifest: dict = {"schema": 1, "entries": [], "packs": [entry]}
    if manifest_path.exists():
        try:
            manifest = merge_documents(
                json.loads(manifest_path.read_text(encoding="utf-8")), manifest
            )
        except Exception:
            pass
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


def merge_feed(args: argparse.Namespace) -> int:
    """把多个平台构建出的 feed slice 合成唯一 feed（manifest + artifacts + packs）。

    PyInstaller 不可跨平台冻结，因此 Windows / macOS-arm64 / macOS-x64 各在
    目标 runner 上 build 出一个 slice；本命令在聚合 job 中把它们合成发布用的
    单份 feed。同名文件若摘要不同则直接报错，绝不静默覆盖（防止平台产物串味）。
    """
    out = Path(args.out)
    (out / "artifacts").mkdir(parents=True, exist_ok=True)
    (out / "packs").mkdir(parents=True, exist_ok=True)
    merged: dict = {"schema": 1, "entries": [], "packs": []}
    for raw in args.slice:
        s = Path(raw)
        mp = s / "manifest.json"
        if not mp.is_file():
            raise SystemExit(f"merge-feed: missing {mp}")
        merged = merge_documents(merged, json.loads(mp.read_text(encoding="utf-8")))
        for sub in ("artifacts", "packs"):
            src = s / sub
            if not src.is_dir():
                continue
            for f in sorted(src.iterdir()):
                if not f.is_file():
                    continue
                dest = out / sub / f.name
                digest = sha256_of(f)
                if dest.exists():
                    if sha256_of(dest) != digest:
                        raise SystemExit(
                            f"merge-feed: conflicting {sub}/{f.name} from {s} "
                            "(platform artifacts must have distinct names)"
                        )
                    continue
                shutil.copy2(f, dest)
    manifest_path = out / "manifest.json"
    manifest_path.write_text(json.dumps(merged, indent=2), encoding="utf-8")
    print(f"[merge] slices   : {len(args.slice)}")
    print(f"[merge] entries  : {len(merged['entries'])}")
    print(f"[merge] packs    : {len(merged['packs'])}")
    print(f"[merge] manifest : {manifest_path}")
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
    b.add_argument("--url-layout", choices=["subdir", "flat"], default="subdir",
                   help="subdir=<base>/artifacts/.. (本地 http feed)；flat=<base>/.. (GitHub Release 资产)")
    b.add_argument("--base-probe-url", default="http://127.0.0.1:18778",
                   help="AFM probe/attach base url written to the manifest entry")
    b.add_argument("--python", default=sys.executable, help="interpreter with PyInstaller + deps")
    b.add_argument("--fallback-base-url", default="",
                   help="object-storage mirror base for assetFallbackUrl (empty = no fallback)")
    b.add_argument("--collect", action="append", default=[],
                   help="extra PyInstaller --collect-all package (e.g. awam_resource_gateway)")
    b.add_argument("--paths", action="append", default=[],
                   help="extra --paths (relative to --src or absolute); for package-style entries")
    b.add_argument("--add-data", dest="add_data", action="append", default=[],
                   help="extra PyInstaller --add-data 'local{sep}dest'")
    b.add_argument("--platform", default="",
                   help="feed entry platform key (e.g. macos-aarch64 / macos-x86_64 / "
                        "windows-x86_64); default: derive from the build host")
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
    pd.add_argument("--url-layout", choices=["subdir", "flat"], default="subdir",
                    help="subdir=<base>/packs/.. (本地 http feed)；flat=<base>/.. (GitHub Release 资产)")
    pd.add_argument("--fallback-base-url", default="",
                    help="object-storage mirror base for installFallbackUrl (empty = no fallback)")
    pd.set_defaults(fn=pack_dir)

    mg = sub.add_parser("merge-feed", help="merge per-platform feed slices into one release feed")
    mg.add_argument("--slice", dest="slice", action="append", required=True,
                    help="a build host's feed dir (repeatable); holds manifest.json + artifacts/ + packs/")
    mg.add_argument("--out", required=True, help="merged feed output dir")
    mg.set_defaults(fn=merge_feed)

    args = p.parse_args()
    return args.fn(args)


if __name__ == "__main__":
    raise SystemExit(main())
