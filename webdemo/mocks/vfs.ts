/* Stateful virtual filesystem + work-group + project backend for the browser
   demo (webdemo). Mirrors the Rust `invoke` command surface the real AFM
   frontend calls, so the genuine Vue UI runs fully in a plain browser.
   Seed data is coherent: every work-group member and project member points to
   a real, populated directory, and navigator / work-groups / projects all show
   the same linked set of folders. */

type Kind = "dir" | "file" | "drive";
interface VNode {
  name: string;
  kind: Kind;
  size?: number;
  modifiedMs?: number;
  children: Record<string, VNode>;
}

const norm = (p: string) => p.replace(/\//g, "\\").replace(/\\+$/, "");
const baseName = (p: string) => {
  const n = norm(p);
  const i = n.lastIndexOf("\\");
  return i >= 0 ? n.slice(i + 1) : n;
};
const parentPath = (p: string): string | null => {
  const n = norm(p);
  const i = n.lastIndexOf("\\");
  if (i < 0) return null;
  const up = n.slice(0, i);
  return /^[A-Za-z]:$/.test(up) ? up + "\\" : up;
};
const join = (dir: string, name: string) => norm(dir) + "\\" + name;

const file = (name: string, size = 2048): VNode => ({ name, kind: "file", size, modifiedMs: Date.now() });
const dir = (name: string, children: Record<string, VNode> = {}): VNode => ({ name, kind: "dir", modifiedMs: Date.now(), children });

/** Build a nested directory node from a spec: value number => file, object => dir. */
function mk(name: string, spec?: Record<string, unknown>): VNode {
  const node = dir(name);
  for (const [k, v] of Object.entries(spec || {})) {
    if (typeof v === "number") node.children[k] = file(k, v || 1024);
    else node.children[k] = mk(k, v as Record<string, unknown>);
  }
  return node;
}

/* ---- seed tree (drives + populated folders) ---- */
const C = mk("C:", {
  Users: {
    demo: {
      Desktop: { "截图.png": 820_000, "待办.txt": 512, "快捷方式.url": 180 },
      Documents: {
        "季度报告.docx": 128_000, "预算表.xlsx": 96_000, "会议纪要.md": 4_200,
        收件箱: { "发票.pdf": 240_000, "合同扫描件.pdf": 1_200_000, "说明.md": 3_200 },
        归档: { "2025": { "年终总结.docx": 88_000, "复盘.md": 6_400 }, "2024": { "年报.pdf": 520_000 } },
      },
      Downloads: { "installer.exe": 42_000_000, "photo.jpg": 1_800_000, "release-notes.txt": 2_400, "setup.zip": 8_600_000 },
      Pictures: { "wallpaper.png": 3_200_000, "IMG_2048.jpg": 4_200_000, "IMG_2049.jpg": 3_100_000, 旅行: { "海边.jpg": 5_000_000, "山顶.jpg": 4_600_000 } },
      Music: { "track-01.mp3": 5_000_000, "track-02.mp3": 4_300_000 },
      Videos: { "demo-clip.mp4": 128_000_000 },
    },
  },
  Windows: {
    System32: { "shell32.dll": 7_400_000, "explorer.exe": 5_100_000, "cmd.exe": 300_000, config: { "system.ini": 1_400 } },
    "win.ini": 220,
  },
});

const G = mk("G:", {
  Projects: {
    "a-file-manager": {
      src: {
        views: { "NavigatorView.vue": 21_000, "WorkGroupsView.vue": 24_500, "ProjectsView.vue": 31_200, "ExtensionsView.vue": 18_000, "SettingsView.vue": 44_000, "HelpView.vue": 6_000 },
        components: {
          shell: { "AppShell.vue": 38_000, "SideNav.vue": 9_000, "Breadcrumb.vue": 4_200 },
          navigator: { "FolderTree.vue": 9_800, "PreviewPane.vue": 12_000, "DetailsPane.vue": 8_600, "NavigatorContextMenu.vue": 16_500 },
        },
        state: { "navigatorBrowse.ts": 46_800, "fileClipboard.ts": 11_200, "favorites.ts": 5_400 },
        styles: { "fluent-tokens.css": 6_400, "base.css": 3_800 },
        i18n: { "zh-CN.json": 88_000, "en.json": 84_000, "index.ts": 3_100 },
        router: { "index.ts": 4_800 },
        "main.ts": 3_100, "App.vue": 2_600,
      },
      "src-tauri": {
        src: { "main.rs": 2_100, file_ops: { "mod.rs": 31_000, "undo.rs": 6_800 }, ai: { "agents.rs": 24_000 } },
        "Cargo.toml": 1_800, "tauri.conf.json": 2_400,
      },
      docs: {
        "glossary-core-concepts.md": 8_200, "file-ops-scenario-baseline.md": 22_000, "explorer-baseline.md": 15_000,
        adr: { "0022-extension-architecture.md": 9_800, "0027-remote-clipboard.md": 6_400, "0001-record-adr.md": 2_400 },
      },
      scripts: { "check-i18n.mjs": 4_000, "check-fluent-p1-color.mjs": 5_000, "check-file-ops.mjs": 6_200, "build-flavor.mjs": 3_400 },
      members: { "shared-ui": { "tokens.md": 2_200 } },
      "CONTEXT.md": 1_200, "tasks.json": 900, "package.json": 17_000, "vite.config.ts": 900, "index.html": 1_200, "README.md": 11_400, "LICENSE": 35_000,
    },
    "ths-linker": {
      ths_linker: { find_offset: { "finder.py": 12_000, "guided.py": 8_600, "probes.py": 5_400 }, "__init__.py": 120, "watch.py": 6_100, "offsets.json": 3_800 },
      skills: { "smart-review.md": 9_000 },
      tests: { "test_core_logic.py": 14_000 },
      "CONTEXT.md": 900, "tasks.json": 700, "pyproject.toml": 1_900, "README.md": 7_200,
    },
    "vibe-astock": {
      duanxian: { "review_graph.py": 19_000, "llm_preflight.py": 4_800 },
      webui: { src: { "AgentReview.tsx": 11_000, "api.ts": 2_400 }, "App.vue": 5_200 },
      "CONTEXT.md": 1_000, "tasks.json": 800, "config.py": 6_000, "test_core_logic.py": 9_800, "README.md": 4_200,
    },
  },
  Repos: {
    backend: { internal: { "server.go": 9_200, "store.go": 5_400 }, "main.go": 9_200, "go.mod": 480, "api.http": 1_200 },
    frontend: { src: { "App.vue": 5_400, "main.ts": 800 }, "index.html": 3_100, "vite.config.ts": 900, "package.json": 1_400 },
  },
  "知识": {
    adr: { "0001-record-architecture-decisions.md": 2_400 },
    glossary: { "术语表.md": 5_200, "架构笔记.md": 15_800 },
    "reading-list.md": 3_100, "ideas.md": 2_600,
  },
});

const tree: Record<string, VNode> = { "C:": C, "G:": G };

/* ---- lookups ---- */
function findNode(path: string): VNode | undefined {
  const n = norm(path);
  const parts = n.split("\\");
  const rootKey = parts[0].endsWith(":") ? parts[0] : parts[0] + ":";
  let cur = tree[rootKey];
  if (!cur) return undefined;
  for (let i = 1; i < parts.length; i++) {
    cur = cur.children[parts[i]];
    if (!cur) return undefined;
  }
  return cur;
}

/* ---- clipboard + recycle + undo (snapshot stacks) ---- */
let clip: { paths: string[]; mode: "copy" | "cut" | "link" | null } = { paths: [], mode: null };
const recycle: { path: string; name: string; parent: string; kind: Kind; snapshot: VNode; at: number }[] = [];
const undoStack: string[] = [];
function snap() { return JSON.stringify(tree); }
function commit() { undoStack.push(snap()); if (undoStack.length > 50) undoStack.shift(); }
function restoreSnap(s: string) {
  const d = JSON.parse(s);
  Object.keys(tree).forEach((k) => delete tree[k]);
  Object.assign(tree, d);
}

/* ---- shared paths (single source of truth for linkage) ---- */
const AFM = "G:\\Projects\\a-file-manager";
const THS = "G:\\Projects\\ths-linker";
const VIBE = "G:\\Projects\\vibe-astock";
const KNOW = "G:\\知识";
const DOCS = "C:\\Users\\demo\\Documents";
const DOWN = "C:\\Users\\demo\\Downloads";
const PICS = "C:\\Users\\demo\\Pictures";

/* ---- work groups (members = real, populated directories) ---- */
const wgDefaultId = "wg-default";
const workGroups = {
  workGroups: [
    { id: wgDefaultId, name: "默认工作组", isDefault: true, memberPaths: [DOCS, DOWN, PICS], openState: null },
    { id: "wg-afm", name: "AFM 开发", isDefault: false, memberPaths: [AFM, KNOW, "G:\\Repos\\frontend"], openState: null },
    { id: "wg-ths", name: "智存 · 投研联动", isDefault: false, memberPaths: [THS, VIBE, "G:\\Repos\\backend"], openState: null },
  ],
  activeWorkGroupId: "wg-afm",
};
const wgDoc = () => JSON.parse(JSON.stringify(workGroups));

/* ---- projects (dir + members resolve to the same folders as work groups) ---- */
const projects = {
  projectsRoot: "G:\\Projects",
  activeSlug: "a-file-manager",
  workspaces: [
    { path: AFM, project: { slug: "a-file-manager", displayName: "A File Manager", description: "为 AI 智能体打造的现代文件管理器（Tauri 2 + Vue 3）", boundWorkGroupId: "wg-afm", createdAt: Date.now() - 8e8, updatedAt: Date.now() } },
    { path: THS, project: { slug: "ths-linker", displayName: "智存 · 同花顺联动", description: "同花顺联动的本地控制面（HTTP 8766 / WS 8765）", boundWorkGroupId: "wg-ths", createdAt: Date.now() - 6e8, updatedAt: Date.now() - 1e7 } },
  ],
};
const projDoc = () => JSON.parse(JSON.stringify(projects));

/* member targets, all real + populated */
const MEMBERS: Record<string, any[]> = {
  "a-file-manager": [
    { name: "frontend", junctionPath: AFM + "\\members\\frontend", targetPath: "G:\\Repos\\frontend", openPath: "G:\\Repos\\frontend" },
    { name: "shared-ui", junctionPath: AFM + "\\members\\shared-ui", targetPath: AFM + "\\members\\shared-ui", openPath: AFM + "\\members\\shared-ui" },
  ],
  "ths-linker": [
    { name: "ths_linker", junctionPath: THS + "\\members\\ths_linker", targetPath: THS + "\\ths_linker", openPath: THS + "\\ths_linker" },
    { name: "backend", junctionPath: THS + "\\members\\backend", targetPath: "G:\\Repos\\backend", openPath: "G:\\Repos\\backend" },
    { name: "vibe-astock", junctionPath: THS + "\\members\\vibe-astock", targetPath: VIBE, openPath: VIBE },
  ],
};

/* ---- settings ---- */
const settingsDoc = {
  version: 1,
  values: {
    "core.language": "zh-CN", "core.theme": "system", "core.listDensity": "standard",
    "core.showHidden": false, "core.navigatorViewMode": "details",
    "core.navigatorSortColumn": "name", "core.navigatorSortDirection": "asc",
    "core.navigatorGroupBy": "none", "core.navigatorGroupDirection": "asc",
    "core.closeBehavior": "ask", "core.startupPage": "navigator",
    "core.defaultLocation": "G:\\Projects", "core.projectsRoot": "G:\\Projects",
    "core.fileDoubleClickAction": "systemDefault", "core.newTabBehavior": "guide", "core.trackRecentVisits": true,
  } as Record<string, unknown>,
};

const ok = (paths: string[]): any => ({ ok: true, paths, error: null, errorCode: null });

/* ---------- command router ---------- */
export function handle(cmd: string, a: any = {}): any {
  switch (cmd) {
    /* settings */
    case "settings_get": return settingsDoc;
    case "settings_set": { Object.assign(settingsDoc.values, a.patch?.values || {}); return JSON.parse(JSON.stringify(settingsDoc)); }
    case "platform_capabilities": return {
      os: "windows", arch: "x86_64", primaryModifier: "ctrl", pathSeparator: "\\", caseInsensitivePaths: true,
      shellIcons: false, systemThumbnails: false, trashDelete: true, trashListRestore: true, shellClipboard: false,
      nativeOpenWithDialog: false, nativeContextMenu: false, osShellVerb: false, registryAutostart: false, nativeAutostart: false,
      singleInstance: false, globalSearch: false, globalSearchEngine: "none", quickLookPreview: false, peekPreview: false,
      oleDragOut: false, nativeDragOut: false, webviewDragOut: true, hiddenFileAttributes: false, driveLetters: true, volumeList: true,
      systemTray: false, webview2Required: false, microphonePermissionPrecheck: false, sidecarNativeArtifacts: false,
    };

    /* feature packs / extensions (safe empty) */
    case "extension_host_list_contributions": return { sideNav: [], routes: [], commands: [], sidecars: [], config: [], search: [], extensionsSections: [] };
    case "extension_host_list_packs": return [];
    case "managed_dependency_list": return [];
    case "managed_dependency_check_all_updates": return [];
    case "ai_addon_status": return null;

    /* favorites (linked to the same dirs) */
    case "favorites_get": return { favorites: [{ path: AFM, addedAt: Date.now(), isDir: true }, { path: KNOW, addedAt: Date.now(), isDir: true }, { path: VIBE, addedAt: Date.now(), isDir: true }] };
    case "favorites_os_hook_status": return { registered: false, note: "" };
    case "favorites_toggle": return { document: { favorites: [] }, added: false, path: a.path };

    /* work groups */
    case "work_groups_get": return wgDoc();
    case "work_groups_reorder": return wgDoc();
    case "work_groups_delete": { const arr = workGroups.workGroups.filter((w) => w.id !== a.id); workGroups.workGroups.length = 0; workGroups.workGroups.push(...arr); return wgDoc(); }
    case "work_groups_rename": { const w = workGroups.workGroups.find((x) => x.id === a.id); if (w) w.name = a.name; return wgDoc(); }
    case "work_groups_remove_member": { const w = workGroups.workGroups.find((x) => x.id === a.workGroupId); if (w) w.memberPaths = w.memberPaths.filter((m) => m !== a.path); return wgDoc(); }
    case "work_groups_silent_switch": { const t = workGroups.workGroups.find((w) => w.id === a.targetWorkGroupId); workGroups.activeWorkGroupId = a.targetWorkGroupId; return { document: wgDoc(), openStateToRestore: t?.openState || null, requireSavePrompt: false, relativeMemberPathsToOpen: [], notice: null }; }
    case "work_groups_create": { const id = "wg-" + Math.random().toString(36).slice(2, 6); const w = { id, name: a.name || "新建工作组", isDefault: false, memberPaths: a.memberPaths || [], openState: null }; workGroups.workGroups.push(w); workGroups.activeWorkGroupId = id; return { document: wgDoc(), openStateToRestore: null, requireSavePrompt: false, relativeMemberPathsToOpen: [], notice: null }; }
    case "work_groups_copy": return [wgDoc(), { id: "wg-c", name: (a.name || "副本") + " 副本", isDefault: false, memberPaths: [], openState: null }];
    case "work_groups_save_open_state": case "work_groups_save_members_from_open_state": return wgDoc();

    /* projects */
    case "projects_workspace_list": return projDoc();
    case "projects_workspace_member_list": return (MEMBERS[a.slug] || []).map((m) => ({ ...m }));
    case "projects_workspace_context_get": return a.slug === "a-file-manager"
      ? "# A File Manager · 项目上下文\n\nTauri 2 + Vue 3 + TypeScript，Windows 优先的现代桌面文件管理器。\n\n## 约定\n- UI 共享样式：src/styles/fluent-tokens.css\n- 行为基线：docs/file-ops-scenario-baseline.md\n- 门禁：scripts/check-*.mjs（与源码字符串强耦合）\n"
      : "# 智存 · 同花顺联动\n\n本地控制面 HTTP 8766 / WS 8765，需官方同花顺客户端 + 授权。\n前端 webui（Vue3+Vite，dev 8767），数据来自本地控制面。\n";
    case "projects_workspace_activate": return { workspace: projects.workspaces.find((w) => w.project.slug === a.slug) || projects.workspaces[0], index: { activeProject: { slug: a.slug } }, silentSwitch: null };
    case "projects_workspace_bind_work_group": { const w = projects.workspaces.find((x) => x.project.slug === a.slug); if (w) w.project.boundWorkGroupId = a.workGroupId || null; return projDoc(); }
    case "projects_workspace_rename": case "projects_workspace_member_add": case "projects_workspace_member_remove": case "projects_workspace_context_set": case "projects_workspace_delete": case "projects_workspace_copy": case "projects_workspace_create": case "projects_workspace_promote_from_work_group": return projDoc();

    /* navigator read */
    case "navigator_default_path": return "G:\\Projects";
    case "navigator_list_drives": return [
      { letter: "C:", path: "C:\\", label: "本地磁盘 (C:)", kind: "fixed" },
      { letter: "G:", path: "G:\\", label: "项目盘 (G:)", kind: "fixed" },
    ];
    case "navigator_user_folders": return { home: "C:\\Users\\demo", desktop: "C:\\Users\\demo\\Desktop", documents: DOCS, downloads: DOWN, pictures: PICS, music: "C:\\Users\\demo\\Music", videos: "C:\\Users\\demo\\Videos" };
    case "navigator_shell_places": return { onedrive: null, libraries: [], network: [{ name: "局域网共享", path: "\\\\NAS\\share" }] };
    case "navigator_network_shares": return [{ name: "局域网共享", path: "\\\\NAS\\share" }];
    case "navigator_expand_path": return norm(a.path ?? a);
    case "navigator_parent": return parentPath(typeof a === "string" ? a : a.path);
    case "navigator_disk_space": return { path: a.path || "C:\\", freeBytes: 128_000_000_000, totalBytes: 512_000_000_000 };

    case "navigator_list": {
      const path = norm(a.path);
      const node = findNode(path);
      if (!node || node.kind === "file") return { path, entries: [], dirCount: 0, fileCount: 0, error: node ? "不是目录" : "路径不存在" };
      const entries = Object.entries(node.children).map(([name, c]) => ({
        name, path: join(path, name), kind: c.kind === "file" ? "file" : "dir",
        size: c.kind === "file" ? c.size || 0 : 0, modifiedMs: c.modifiedMs || Date.now(),
        providerKind: null, isLink: null, linkTarget: null,
      }));
      return { path, entries, dirCount: entries.filter((e) => e.kind === "dir").length, fileCount: entries.filter((e) => e.kind === "file").length, error: null };
    }
    case "navigator_stat": {
      const path = norm(a.path); const node = findNode(path);
      return { name: baseName(path), path, kind: node?.kind === "file" ? "file" : "dir", size: node?.size || 0, modifiedMs: node?.modifiedMs || Date.now(), createdMs: node?.modifiedMs || Date.now(), readonly: false, hidden: false, totalSize: null, fileCount: null, dirCount: null, totalSizeTruncated: false, link: null };
    }
    case "navigator_search": {
      const q = String(a.query || "").toLowerCase(); const root = norm(a.path); const out: any[] = [];
      const walk = (node: VNode, p: string, depth: number) => { if (depth > 5) return; Object.entries(node.children).forEach(([name, c]) => { const cp = join(p, name); if (name.toLowerCase().includes(q)) out.push({ name, path: cp, kind: c.kind === "file" ? "file" : "dir", size: c.size || 0, modifiedMs: c.modifiedMs || Date.now() }); if (c.kind !== "file") walk(c, cp, depth + 1); }); };
      const rn = findNode(root); if (rn) walk(rn, root, 0);
      return { path: root, query: a.query, entries: out.slice(0, 200), truncated: out.length > 200 };
    }
    case "shell_icon_get_many": return [];
    case "shell_show_context_menu": return { success: false, cancelled: true, error: null };

    /* clipboard */
    case "file_ops_clipboard_set": clip = { paths: (a.paths || []).map(norm), mode: a.mode }; return null;
    case "file_ops_clipboard_clear": clip = { paths: [], mode: null }; return null;
    case "file_ops_clipboard_get": return { paths: clip.paths, mode: clip.mode };

    /* file write ops */
    case "file_ops_name_exists": { const pn = findNode(norm(a.destDir)); return !!pn?.children?.[a.name]; }
    case "file_ops_name_conflict_info": { const pn = findNode(norm(a.destDir)); const ex = pn?.children?.[a.name]; return { exists: !!ex, typeMismatch: !!ex && (ex.kind === "file") !== (findNode(a.source)?.kind === "file") }; }

    case "file_ops_create_dir": { commit(); const pn = findNode(norm(a.parent)); if (!pn || pn.kind === "file") return { ok: false, paths: [], error: "父目录不存在" }; let name = a.name, k = 1; while (pn.children[name]) name = `${a.name} ${++k}`; pn.children[name] = dir(name); return ok([join(a.parent, name)]); }
    case "file_ops_create_file": { commit(); const pn = findNode(norm(a.parent)); if (!pn || pn.kind === "file") return { ok: false, paths: [], error: "父目录不存在" }; let name = a.name, k = 1; while (pn.children[name]) { const d = name.lastIndexOf("."); name = d > 0 ? `${a.name.slice(0, d)} ${++k}${a.name.slice(d)}` : `${a.name} (${++k})`; } pn.children[name] = file(name, 0); return ok([join(a.parent, name)]); }
    case "file_ops_rename": { commit(); const pp = parentPath(a.path); const pn = findNode(pp || ""); if (!pn) return { ok: false, paths: [], error: "父目录不存在" }; const node = pn.children[baseName(a.path)]; if (!node) return { ok: false, paths: [], error: "项不存在" }; if (pn.children[a.newName] && !a.replace) return { ok: false, paths: [], error: "重名" }; delete pn.children[baseName(a.path)]; node.name = a.newName; node.modifiedMs = Date.now(); pn.children[a.newName] = node; return ok([join(pp, a.newName)]); }

    case "file_ops_copy": case "file_ops_move": case "file_ops_create_shortcuts": {
      const isMove = cmd === "file_ops_move";
      const isLink = cmd === "file_ops_create_shortcuts";
      commit();
      const dest = findNode(norm(a.destDir)); if (!dest || dest.kind === "file") return { ok: false, paths: [], error: "目标不存在" };
      const made: string[] = [];
      (a.sources || []).forEach((src: string) => {
        const sp = norm(src); const sn = findNode(sp); if (!sn) return;
        let name = isLink ? baseName(sp) + " - Shortcut.lnk" : baseName(sp);
        if (dest.children[name] && a.conflict === "skip") return;
        if (dest.children[name] && a.conflict !== "replace") { const d = name.lastIndexOf("."); name = d > 0 ? `${name.slice(0, d)} - 副本${name.slice(d)}` : name + " - 副本"; }
        if (a.conflict === "replace") delete dest.children[name];
        const clone = isLink ? file(name, 1024) : JSON.parse(JSON.stringify(sn)); clone.name = name; clone.modifiedMs = Date.now();
        dest.children[name] = clone; made.push(join(a.destDir, name));
        if (isMove) { const pn = findNode(parentPath(sp) || ""); if (pn) delete pn.children[baseName(sp)]; }
      });
      return ok(made);
    }
    case "file_ops_delete": case "file_ops_delete_permanent": {
      const perm = cmd === "file_ops_delete_permanent"; commit();
      const out: string[] = [];
      (a.paths || []).forEach((p: string) => { const pp = parentPath(norm(p)); const pn = findNode(pp || ""); const node = pn?.children?.[baseName(p)]; if (pn && node) { delete pn.children[baseName(p)]; if (!perm) recycle.unshift({ path: norm(p), name: node.name, parent: pp!, kind: node.kind, snapshot: node, at: Date.now() }); out.push(norm(p)); } });
      return ok(out);
    }
    case "file_ops_can_recycle": return [];
    case "file_ops_fingerprints": return (a.paths || []).map(() => null);
    case "file_undo_pop": case "file_ops_undo": { const s = undoStack.pop(); if (!s) return { ok: false, paths: [], error: "没有可撤销的操作" }; restoreSnap(s); return { ok: true, paths: [], error: null }; }

    default: return null;
  }
}

export { recycle };
