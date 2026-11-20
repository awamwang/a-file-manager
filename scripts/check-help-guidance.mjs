// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Independent QA gate for the in-app Help & Guidance system (ADR 0021).
 *
 * Run: node scripts/check-help-guidance.mjs
 *
 * Checks are deliberately invariant-based rather than file-existence based:
 * anchor coverage, replay progress-neutrality, single-key reset, five-locale
 * registry coverage, guide.* namespace preservation, AI flavor gating,
 * missing-anchor recovery, requireClick enforcement, and replay anchor
 * reachability (the last one currently exposes a live defect).
 */
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (rel) => readFileSync(join(root, rel), "utf8");
const LOCALES = ["en", "zh-CN", "ja", "de", "fr"];

let passed = 0;
const failures = [];

function check(name, fn) {
  try {
    const detail = fn();
    passed += 1;
    console.log(`PASS  ${name}${detail ? ` — ${detail}` : ""}`);
  } catch (error) {
    failures.push({ name, message: error.message });
    console.log(`FAIL  ${name} — ${error.message}`);
  }
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function walk(dir, suffix, out = []) {
  for (const name of readdirSync(dir)) {
    const full = join(dir, name);
    if (statSync(full).isDirectory()) walk(full, suffix, out);
    else if (name.endsWith(suffix)) out.push(full);
  }
  return out;
}

function flatten(obj, prefix = "", out = {}) {
  for (const [key, value] of Object.entries(obj)) {
    const path = prefix ? `${prefix}.${key}` : key;
    if (value && typeof value === "object" && !Array.isArray(value)) {
      flatten(value, path, out);
    } else {
      out[path] = value;
    }
  }
  return out;
}

const registry = read("src/help/registry.ts");
const progress = read("src/help/progress.ts");
const tourState = read("src/help/tourState.ts");
const guidedTour = read("src/help/components/GuidedTour.vue");
const helpCenter = read("src/help/components/HelpCenter.vue");
const hintBubble = read("src/help/components/HintBubble.vue");
const appShell = read("src/components/shell/AppShell.vue");
const navigatorView = read("src/views/NavigatorView.vue");
const router = read("src/router/index.ts");

/* ------------------------------------------------------------------ 1 ---- */
check("锚点全覆盖：registry 的每个 anchor 都有 data-tour-id", () => {
  const registryAnchors = [
    ...new Set([...registry.matchAll(/anchor:\s*"([^"]+)"/g)].map((m) => m[1])),
  ];
  const vueFiles = walk(join(root, "src"), ".vue");
  const domAnchors = new Set();
  for (const file of vueFiles) {
    const source = readFileSync(file, "utf8");
    for (const match of source.matchAll(/data-tour-id="([^"]+)"/g)) {
      // Ignore the runtime query template, e.g. data-tour-id="${CSS.escape(anchor)}".
      if (!match[1].includes("${")) domAnchors.add(match[1]);
    }
  }
  const missing = registryAnchors.filter((id) => !domAnchors.has(id));
  assert(registryAnchors.length > 0, "未能从 registry.ts 提取到任何 anchor");
  assert(missing.length === 0, `缺失锚点: ${missing.join(", ")}`);
  const unused = [...domAnchors].filter((id) => !registryAnchors.includes(id)).sort();
  return `${registryAnchors.length} 个注册锚点全命中；预留未引用 ${unused.length} 个 (${unused.join(", ")})`;
});

/* ------------------------------------------------------------------ 2 ---- */
check("进度语义：重播不写进度（false），主动试一试写进度（true）", () => {
  const replayBody = helpCenter.match(/function replay\([\s\S]*?\n\}/)?.[0] ?? "";
  assert(replayBody.includes("startTour("), "HelpCenter.replay 未调用 startTour");
  assert(
    /startTour\([^)]*,\s*false\s*\)/.test(replayBody),
    "HelpCenter.replay 未传 recordProgress=false",
  );
  assert(
    !/markSeen\(|markDone\(/.test(replayBody),
    "HelpCenter.replay 直接写了 seen/done 进度",
  );
  assert(
    /recordProgress\s*&&\s*props\.tourId\)\s*markDone\(/.test(guidedTour),
    "GuidedTour.finish 未用 recordProgress 门控 markDone",
  );
  assert(
    /startTour\([^)]*,\s*true\s*\)/.test(hintBubble),
    "HintBubble 的主动试一试未以 recordProgress=true 启动教学",
  );
  return "HelpCenter.replay=false；finish 门控=true；主动试一试=true";
});

/* ------------------------------------------------------------------ 3 ---- */
check("重置只删单 key：无 clear()，仅 removeItem(STORAGE_KEY)", () => {
  const resetBody = progress.match(/export function resetAllProgress\([\s\S]*?\n\}/)?.[0] ?? "";
  assert(resetBody.includes("removeItem(STORAGE_KEY)"), "resetAllProgress 未 removeItem(STORAGE_KEY)");
  assert(!/\.clear\s*\(/.test(resetBody), "resetAllProgress 调用了 clear()");
  assert(
    (resetBody.match(/removeItem\s*\(/g) ?? []).length === 1,
    "resetAllProgress 调用了多次 removeItem",
  );
  assert(
    (progress.match(/const STORAGE_KEY\s*=/g) ?? []).length === 1,
    "progress.ts 使用了多个存储 key",
  );
  assert(helpCenter.includes("resetConfirming") && helpCenter.includes("confirmReset"), "重置缺少二次确认");
  assert(/@click="confirmReset"/.test(helpCenter), "确认按钮未绑定 confirmReset");
  return "单 key、单次 removeItem、无 clear()、有二次确认";
});

/* ------------------------------------------------------------------ 4 ---- */
check("进度层行为实测：重播标记不改存储；重置不碰其他 key", () => {
  const harness = `
    const store = new Map([
      ["afm.settings.v13", JSON.stringify({theme:"dark"})],
      ["afm.other", "keep-me"],
    ]);
    globalThis.window = { localStorage: {
      getItem: k => store.has(k) ? store.get(k) : null,
      setItem: (k,v) => { store.set(k, String(v)); },
      removeItem: k => { store.delete(k); },
      clear: () => { throw new Error("clear() was called"); },
    }};
    const p = await import("./src/help/progress.ts");
    const t = await import("./src/help/tourState.ts");
    p.markSeen("navigator"); p.markDone("navigator");
    if (!p.isSeen("navigator") || !p.isDone("navigator")) throw new Error("markSeen/markDone failed");
    const before = store.get("afm.help.progress.v1");
    t.startTour("navigator", [{anchor:"x",titleKey:"t",bodyKey:"b"}], false);
    if (t.activeTour.value.recordProgress !== false) throw new Error("replay flag not false");
    if (store.get("afm.help.progress.v1") !== before) throw new Error("replay mutated progress");
    t.stopTour();
    p.resetAllProgress();
    if (store.has("afm.help.progress.v1")) throw new Error("reset left progress key");
    if (store.get("afm.settings.v13") !== JSON.stringify({theme:"dark"})) throw new Error("reset touched settings");
    if (store.get("afm.other") !== "keep-me") throw new Error("reset touched other key");
    console.log("behavior-ok");
  `;
  const output = execFileSync(
    process.execPath,
    ["--experimental-strip-types", "--input-type=module", "-e", harness],
    { cwd: root, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] },
  );
  assert(output.includes("behavior-ok"), "进度行为测试未通过");
  return "markSeen/markDone/replay(false)/reset 全部实测通过";
});

/* ------------------------------------------------------------------ 5 ---- */
check("五语齐全：注册表 key 在 en/zh-CN/ja/de/fr 全部存在", () => {
  const keys = [
    ...new Set(
      [...registry.matchAll(/\b(?:titleKey|hintKey|bodyKey):\s*"([^"]+)"/g)].map((m) => m[1]),
    ),
  ];
  assert(keys.length === 53, `注册表 key 期望 53，实得 ${keys.length}`);
  const catalogs = {};
  for (const id of LOCALES) {
    catalogs[id] = flatten(JSON.parse(read(`src/i18n/locales/${id}.json`)));
  }
  for (const key of keys) {
    for (const id of LOCALES) {
      assert(Object.hasOwn(catalogs[id], key), `${id} 缺少注册表 key: ${key}`);
    }
  }
  const counts = LOCALES.map((id) => Object.keys(catalogs[id]).length);
  assert(new Set(counts).size === 1, `五语 key 总数不一致: ${counts.join("/")}`);
  return `53 key × 5 语齐全；各语 ${counts[0]} keys`;
});

/* ------------------------------------------------------------------ 6 ---- */
check("Guide 未挪用：guide.* 未新增/改动，help 源码不含 guide.", () => {
  for (const id of LOCALES) {
    const rel = `src/i18n/locales/${id}.json`;
    const before = flatten(JSON.parse(execFileSync("git", ["show", `HEAD:${rel}`], { cwd: root, encoding: "utf8" })));
    const after = flatten(JSON.parse(read(rel)));
    const beforeGuide = Object.fromEntries(Object.entries(before).filter(([k]) => k.startsWith("guide.")));
    const afterGuide = Object.fromEntries(Object.entries(after).filter(([k]) => k.startsWith("guide.")));
    assert(
      JSON.stringify(beforeGuide) === JSON.stringify(afterGuide),
      `${id} 的 guide.* 命名空间被改动`,
    );
  }
  const helpSources = [
    ...walk(join(root, "src/help"), ".ts"),
    ...walk(join(root, "src/help"), ".vue"),
    read("src/views/HelpView.vue") ? join(root, "src/views/HelpView.vue") : "",
  ].filter(Boolean);
  for (const file of helpSources) {
    assert(
      !/\bguide\./.test(readFileSync(file, "utf8")),
      `帮助源码引用了 guide. 命名空间: ${relative(root, file)}`,
    );
  }
  return "五语 guide.* 与 HEAD 完全一致；help 源码零 guide. 引用";
});

/* ------------------------------------------------------------------ 7 ---- */
check("AI 门控：ai entry 有 featureFlag，ai tour 随 registry 过滤，路由按风味注册", () => {
  assert(/featureFlag:\s*"ai"/.test(registry), "ai entry 未声明 featureFlag: ai");
  assert(
    /helpRegistry\.some\(\(entry\)\s*=>\s*entry\.tourId\s*===\s*tour\.id\)/.test(registry),
    "tourRegistry 未按 helpRegistry 过滤（ai tour 可能泄漏到 core）",
  );
  assert(
    /if\s*\(AI_ADDON_ENABLED\)\s*\{[\s\S]*?path:\s*"\/ai"/.test(router),
    "路由未按 AI_ADDON_ENABLED 门控 /ai",
  );
  return "entry/tour/route 三层门控齐备";
});

/* ------------------------------------------------------------------ 8 ---- */
check("缺锚点不卡死：缺失与零尺寸都 nextTick(advance)，并监听 scroll/resize", () => {
  const missingBranch = guidedTour.match(/if\s*\(\s*!element\s*\)\s*\{([\s\S]*?)\n\s*\}/)?.[0] ?? "";
  const zeroBranch = guidedTour.match(/box\.width\s*<=\s*0\s*&&\s*box\.height\s*<=\s*0\s*\)\s*\{([\s\S]*?)\n\s*\}/)?.[0] ?? "";
  assert(/nextTick\(advance\)/.test(missingBranch), "锚点缺失未 nextTick(advance)");
  assert(/nextTick\(advance\)/.test(zeroBranch), "零尺寸锚点未 nextTick(advance)");
  assert(/addEventListener\("scroll"/.test(guidedTour), "未监听 scroll 重算 rect");
  assert(/addEventListener\("resize"/.test(guidedTour), "未监听 resize 重算 rect");
  return "缺失/零尺寸均跳步，scroll/resize 重算";
});

/* ------------------------------------------------------------------ 9 ---- */
check("requireClick 不可绕过：气泡『下一步』禁用且仅真实锚点点击推进", () => {
  assert(
    /:disabled="current\.requireClick === true"/.test(guidedTour),
    "requireClick 步的『下一步』未禁用",
  );
  assert(
    /if\s*\(target\.closest\("\.tour-bubble"\)\)\s*return/.test(guidedTour),
    "点击气泡未被排除",
  );
  assert(
    /anchor\.contains\(target\)/.test(guidedTour),
    "未校验点击目标是否位于真实锚点内",
  );
  assert(/if\s*\(!step \|\| step\.requireClick\)\s*return/.test(guidedTour), "键盘仍可绕过 requireClick");
  return "下一步禁用 + 锚点包含校验 + 键盘不可绕过";
});

/* ----------------------------------------------------------------- 10 ---- */
check("重播锚点可达：replay 启动教学前须确保目标锚点所在路由已挂载", () => {
  const replayBody = helpCenter.match(/function replay\([\s\S]*?\n\}/)?.[0] ?? "";
  const navigates = /useRouter|router\.push|router\.replace|\$router/.test(replayBody);
  assert(
    navigates,
    "HelpCenter.replay 只 startTour、不导航到 entry.route；在 /help 上所有教学锚点都不在 DOM，"
      + "GuidedTour 会逐步跳过并立即 finish，用户看不到任何教学内容（源码 Bug，交工程师修）",
  );
  return "replay 会先挂载目标视图再开教学";
});

/* ----------------------------------------------------------------- 11 ---- */
check("引导点击不吞锚点原动作：onPointerDown 零 stopPropagation/preventDefault（代码+行为）", () => {
  const body = guidedTour.match(/function onPointerDown[\s\S]*?\n\}/)?.[0] ?? "";
  assert(body.length > 0, "未找到 onPointerDown");
  assert(
    !/stopPropagation\s*\(/.test(body),
    "onPointerDown 仍调用 stopPropagation，会吞掉锚点自身的业务动作",
  );
  assert(
    !/preventDefault\s*\(/.test(body),
    "onPointerDown 调用 preventDefault，可能阻断锚点默认行为",
  );
  assert(
    !/stopPropagation\s*\(|stopImmediatePropagation\s*\(/.test(body),
    "onPointerDown 仍有 stopPropagation/stopImmediatePropagation 残留",
  );
  assert(
    !/stopPropagation\s*\(|stopImmediatePropagation\s*\(/.test(guidedTour),
    "GuidedTour.vue 仍有 stopPropagation/stopImmediatePropagation 残留",
  );

  // Execute the real shipped function body with a stopPropagation spy.
  const fn = body.replace("event: PointerEvent", "event").replace("): void {", ") {");
  class Element {}
  let advanced = 0;
  let stopped = 0;
  let prevented = 0;
  const anchor = { contains: (target) => target.isChild === true };
  const current = { value: { anchor: "tour-navigator-help", requireClick: true } };
  const factory = new Function(
    "deps",
    `const {current,anchorElement,advance,Element}=deps;\n${fn}\nreturn onPointerDown;`,
  );
  const handler = factory({
    current,
    anchorElement: () => anchor,
    advance: () => {
      advanced += 1;
      current.value = { anchor: "tour-navigator-help", requireClick: false };
    },
    Element,
  });
  const target = Object.assign(new Element(), { closest: () => null, isChild: true });
  handler({
    target,
    stopPropagation: () => { stopped += 1; },
    preventDefault: () => { prevented += 1; },
  });
  assert(advanced === 1, `真实锚点点击未推进 (advanced=${advanced})`);
  assert(
    stopped === 0 && prevented === 0,
    `锚点业务动作被吞: stopPropagation=${stopped}, preventDefault=${prevented}`,
  );
  return "源码零拦截调用；行为探针推进 1 次、stopPropagation=0、preventDefault=0";
});

/* ----------------------------------------------------------------- 12 ---- */
check("引导点击无回归：气泡/无关元素不推进，重复派发不重复推进", () => {
  const body = guidedTour.match(/function onPointerDown[\s\S]*?\n\}/)?.[0] ?? "";
  const fn = body.replace("event: PointerEvent", "event").replace("): void {", ") {");
  class Element {}
  let advanced = 0;
  const anchor = { contains: (target) => target.isChild === true };
  const current = { value: { anchor: "x", requireClick: true } };
  const factory = new Function(
    "deps",
    `const {current,anchorElement,advance,Element}=deps;\n${fn}\nreturn onPointerDown;`,
  );
  const handler = factory({
    current,
    anchorElement: () => anchor,
    advance: () => {
      advanced += 1;
      current.value = { anchor: "x", requireClick: false };
    },
    Element,
  });
  const mk = (props) => Object.assign(new Element(), props);
  const evt = (target) => ({ target, stopPropagation() {}, preventDefault() {} });

  handler(evt(mk({ closest: (s) => (s === ".tour-bubble" ? {} : null), isChild: true })));
  assert(advanced === 0, "点气泡触发了推进（可绕过 requireClick）");
  handler(evt(mk({ closest: () => null, isChild: false })));
  assert(advanced === 0, "点无关元素触发了推进（误触）");
  handler(evt(mk({ closest: () => null, isChild: true })));
  assert(advanced === 1, "点真实锚点未推进一次");
  handler(evt(mk({ closest: () => null, isChild: true })));
  assert(advanced === 1, `同一事件重复派发造成重复推进 (advanced=${advanced})`);
  current.value = { anchor: "x", requireClick: false };
  handler(evt(mk({ closest: () => null, isChild: true })));
  assert(advanced === 1, "非 requireClick 步仍响应了 pointerdown");
  assert(
    /:disabled="current\.requireClick === true"/.test(guidedTour),
    "气泡『下一步』未按 requireClick 禁用",
  );
  return "气泡/无关元素零推进、真实点击仅一次、重复派发与非 requireClick 均无回归";
});

/* ----------------------------------------------------------------- 13 ---- */
check("功能页 ? 深链接线：HelpLink 传 tour、HelpView 解析 autoTour、HelpCenter 转发 replay", () => {
  const surfaces = [
    "src/components/command/CommandPanel.vue",
    "src/views/AiView.vue",
    "src/views/NavigatorView.vue",
    "src/views/ProjectsView.vue",
    "src/views/ProvidersView.vue",
    "src/views/WorkGroupsView.vue",
  ];
  for (const rel of surfaces) {
    const src = read(rel);
    const links = [...src.matchAll(/<HelpLink\b[^>]*\/?>/g)].map((m) => m[0]);
    assert(links.length > 0, `${rel} 没有 HelpLink`);
    for (const link of links) {
      assert(/\btour(?:=|[\s/>])/.test(link), `${rel} 的 HelpLink 未开启 tour 深链: ${link}`);
    }
  }
  const helpLink = read("src/help/components/HelpLink.vue");
  assert(
    /getHelpEntry\(props\.entryId\)\?\.tourEnabled !== false/.test(helpLink),
    "HelpLink 未按 entryId 读取 tourEnabled 开关",
  );
  assert(
    /props\.tour\s*&&\s*tourAllowed\.value\s*\?\s*\{\s*tour:\s*"1"\s*\}/.test(helpLink),
    "HelpLink 未在 tour=true 且 tourEnabled 未关闭时生成 ?tour=1",
  );
  const helpView = read("src/views/HelpView.vue");
  assert(/route\.query\["tour"\]\s*===\s*"1"/.test(helpView), "HelpView 未解析 ?tour=1");
  assert(/:auto-tour="autoTour"/.test(helpView), "HelpView 未把 autoTour 传给 HelpCenter");
  assert(
    /if\s*\(!props\.autoTour[\s\S]*?\n[\s\S]*?replay\(activeEntry\.value\)/.test(helpCenter),
    "HelpCenter 未把 autoTour 转发到 replay",
  );
  return `${surfaces.length} 个入口 tour 深链齐备且贯通到 replay`;
});

/* ----------------------------------------------------------------- 14 ---- */
check("超时/路由不符只放弃本次启动，绝不误停并发教学", () => {
  // Static: the abort branch must not call stopTour().
  const bridge = tourState.match(
    /export function startTourWhenAnchorReady[\s\S]*?\n\}/,
  )?.[0] ?? "";
  assert(bridge.length > 0, "未找到 startTourWhenAnchorReady");
  const abortBranch = bridge.match(
    /Date\.now\(\)\s*>\s*deadline[\s\S]*?return;/,
  )?.[0] ?? "";
  assert(abortBranch.length > 0, "未找到超时/路由不符的放弃分支");
  // Strip comments before asserting — the explanatory note intentionally
  // mentions "stopTour()" to document why it is NOT called.
  const abortCode = abortBranch.replace(/\/\/[^\n]*/g, "").replace(/\/\*[\s\S]*?\*\//g, "");
  assert(
    !/stopTour\s*\(/.test(abortCode),
    "放弃分支仍调用 stopTour()，会误停正在播放的并发教学",
  );

  // Behavior: a concurrent tour must survive an aborted pending launch.
  const harness = `
    globalThis.window = {
      location: { pathname: "/help" },
      setTimeout: (fn, ms) => setTimeout(fn, ms),
    };
    // Import before faking document: Vue's runtime-dom touches document at
    // module-eval time and would break on a partial stub.
    const t = await import("./src/help/tourState.ts");
    globalThis.document = { querySelector: () => null };
    globalThis.CSS = { escape: (s) => s };
    t.startTour("quickstart", [{anchor:"q",titleKey:"t",bodyKey:"b"}], true);
    let started = false;
    // Route does not match -> abort on first poll; onReady must never fire.
    t.startTourWhenAnchorReady("missing-anchor", "/projects", () => { started = true; });
    await new Promise((r) => setTimeout(r, 200));
    if (started) throw new Error("aborted launch still called onReady");
    if (!t.activeTour.value || t.activeTour.value.id !== "quickstart") {
      throw new Error("aborted launch killed the concurrent quickstart tour");
    }
    console.log("abort-ok");
  `;
  const output = execFileSync(
    process.execPath,
    ["--experimental-strip-types", "--input-type=module", "-e", harness],
    { cwd: root, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] },
  );
  assert(output.includes("abort-ok"), "并发教学存活行为测试未通过");
  return "放弃分支零 stopTour()；并发 quickstart 存活、onReady 未触发";
});

/* ----------------------------------------------------------------- 15 ---- */
check("启动路径不裁剪注册表：helpRegistry 只按风味/平台门控（AI 6 / core 5）", () => {
  const registryDecl = registry.match(/export const helpRegistry[\s\S]*?\n\);/)?.[0] ?? "";
  assert(registryDecl.length > 0, "未找到 helpRegistry 定义");
  assert(!/\broutes\b/.test(registryDecl), "helpRegistry 的 filter 仍按 routes 过滤");
  assert(
    !/defaultContext\s*\(/.test(registryDecl),
    "helpRegistry 的 filter 仍依赖 defaultContext()（模块加载时的启动路径）",
  );
  assert(
    /buildGateAllows\(entry\.applicability\)/.test(registryDecl),
    "helpRegistry 未按构建门控过滤",
  );

  // Evaluate the real shipped predicates and the real entry literals under
  // both flavors x platforms x startup paths. No import.meta.glob needed.
  const extractFn = (name) => {
    const source = registry.match(new RegExp(`function ${name}\\([\\s\\S]*?\\n\\}`))?.[0] ?? "";
    // Strip the TypeScript signature so the real body can run in new Function.
    const header = source.match(/^function\s+(\w+)\s*\(([\s\S]*?)\)\s*:\s*[^{]+\{/);
    if (!header) return source;
    const params = header[2]
      .split(",")
      .map((param) => param.split(":")[0].trim())
      .filter(Boolean);
    return `function ${header[1]}(${params.join(", ")}) {${source.slice(header[0].length)}`;
  };
  const pure = extractFn("flavorAndPlatformAllowed");
  const build = extractFn("buildGateAllows");
  const applicable = extractFn("isApplicable");
  assert(pure && build && applicable, "未能从 registry.ts 提取门控函数");
  const entriesLiteral = (
    registry.match(/const ALL_ENTRIES[^=]*=\s*\[[\s\S]*?\n\];/)?.[0] ?? ""
  ).replace(/^const ALL_ENTRIES[^=]*=/, "const ALL_ENTRIES =");
  assert(entriesLiteral.includes("id:"), "未能从 registry.ts 提取 ALL_ENTRIES");

  const simulate = (aiEnabled, mac, startupPath) =>
    new Function("AI_ADDON_ENABLED", "isMacPlatform", "STARTUP_PATH", `
      ${pure}
      ${build}
      ${applicable}
      ${entriesLiteral}
      const registered = ALL_ENTRIES.filter((entry) => buildGateAllows(entry.applicability));
      // What the old buggy code did at module load — the regression detector.
      const legacy = ALL_ENTRIES.filter((entry) => isApplicable(
        entry.applicability,
        { path: STARTUP_PATH, aiEnabled: AI_ADDON_ENABLED, mac: isMacPlatform() },
      ));
      const visibleAt = (path) => registered.filter((entry) => isApplicable(
        entry.applicability,
        { path, aiEnabled: AI_ADDON_ENABLED, mac: isMacPlatform() },
      ));
      return { registered, legacy, visibleAt };
    `)(aiEnabled, () => mac, startupPath);

  for (const mac of [false, true]) {
    for (const startupPath of ["/", "/navigator"]) {
      const ai = simulate(true, mac, startupPath);
      assert(ai.legacy.length < 6, `回归探针失效：启动路径 ${startupPath} 的旧逻辑未裁剪条目`);
      assert(
        ai.registered.length === 6,
        `AI 构建启动路径 ${startupPath}: 注册表只剩 ${ai.registered.length} 条，期望 6`,
      );
      assert(
        ai.visibleAt("/help").length === 6,
        `AI 构建帮助中心可见 ${ai.visibleAt("/help").length} 条，期望 6`,
      );
      const core = simulate(false, mac, startupPath);
      assert(
        core.registered.length === 5,
        `core 构建启动路径 ${startupPath}: 注册表只剩 ${core.registered.length} 条，期望 5`,
      );
      assert(!core.registered.some((entry) => entry.id === "ai"), "core 构建仍注册 ai entry");
      assert(
        core.visibleAt("/help").length === 5,
        `core 构建帮助中心可见 ${core.visibleAt("/help").length} 条，期望 5`,
      );
    }
  }
  return "AI 6 / core 5；启动路径 / 与 /navigator 均不裁剪；/help 全量可见";
});

/* ----------------------------------------------------------------- 16 ---- */
check("功能级开关：6 条 entry 双开关齐备，L2/L3/? 三端消费且 L1 不受影响", () => {
  const entriesLiteral = (
    registry.match(/const ALL_ENTRIES[^=]*=\s*\[[\s\S]*?\n\];/)?.[0] ?? ""
  );
  const ids = [...entriesLiteral.matchAll(/\bid:\s*"([^"]+)"/g)].map((match) => match[1]);
  assert(ids.length === 6, `entry 数期望 6，实得 ${ids.length}`);
  const flagCount = (name) =>
    (entriesLiteral.match(new RegExp(`\\b${name}:\\s*true`, "g")) ?? []).length;
  assert(flagCount("tourEnabled") === 6, `仅 ${flagCount("tourEnabled")}/6 条声明 tourEnabled`);
  assert(flagCount("hintEnabled") === 6, `仅 ${flagCount("hintEnabled")}/6 条声明 hintEnabled`);
  assert(/tourEnabled\?: boolean/.test(read("src/help/types.ts")), "HelpEntry 缺少 tourEnabled 类型");
  assert(/hintEnabled\?: boolean/.test(read("src/help/types.ts")), "HelpEntry 缺少 hintEnabled 类型");

  const hintBubble = read("src/help/components/HintBubble.vue");
  assert(
    /getHelpEntry\(props\.entryId\)/.test(hintBubble) && /hintEnabled !== false/.test(hintBubble),
    "HintBubble 未按 entryId 读取 hintEnabled",
  );
  assert(/if\s*\(!hintAllowed\.value\)\s*return/.test(hintBubble), "HintBubble 关闭时仍显示气泡");

  const replayBody = helpCenter.match(/function replay\([\s\S]*?\n\}/)?.[0] ?? "";
  assert(/if\s*\(!tourAllowed\(entry\)\)\s*return/.test(replayBody), "HelpCenter.replay 未检查 tourEnabled");
  assert(
    /if\s*\(!tourAllowed\(activeEntry\.value\)\)\s*return/.test(helpCenter),
    "HelpCenter.maybeAutoTour 未检查 tourEnabled",
  );
  assert(
    /<template v-if="tourAllowed\(entry\)">[\s\S]*?v-if="tourForEntry\(entry\)"/.test(helpCenter),
    "重播按钮未按 tourEnabled 隐藏",
  );
  assert(/t\("help\.tourDisabled"\)/.test(helpCenter), "关闭教学时缺少说明文案");

  // L1 stays unconditional: entry list + article block must not consult switches.
  assert(/applicableHelpEntries\("\/help"\)/.test(helpCenter), "帮助中心条目列表被开关影响");
  assert(
    /t\("help\.articleLabel"\)/.test(helpCenter) && /articleHtml/.test(helpCenter),
    "帮助文档区块被开关影响",
  );

  // Protected modules must stay free of feature-level switch coupling.
  for (const [name, source] of [
    ["GuidedTour", guidedTour],
    ["tourState", tourState],
    ["progress", progress],
  ]) {
    assert(
      !/tourEnabled|hintEnabled/.test(source),
      `${name} 被功能级开关污染`,
    );
  }
  return "6 entry 双开关；HintBubble/HelpCenter/HelpLink 三端消费；L1 与受保护模块不变";
});

/* ----------------------------------------------------------------- 17 ---- */
check("主动触发：气泡试一试按 tourId 播教学，Navigator 映射 quickstart，无自动定时器", () => {
  assert(/tourId\?: string/.test(hintBubble), "HintBubble 缺少可选 tourId prop");
  assert(
    /props\.tourId \|\| entry\.value\?\.tourId/.test(hintBubble),
    "HintBubble 未回退到 entry.tourId",
  );
  assert(/t\("help\.hintTryTour"\)/.test(hintBubble), "试一试按钮缺少 i18n 文案");
  assert(/@click="tryTour"/.test(hintBubble), "试一试按钮未绑定 tryTour");
  const tryBody = hintBubble.match(/function tryTour\([\s\S]*?\n\}/)?.[0] ?? "";
  assert(tryBody.length > 0, "未找到 tryTour 实现");
  assert(/dismiss\(\)/.test(tryBody), "tryTour 未关闭气泡");
  const dismissBody = hintBubble.match(/function dismiss\([\s\S]*?\n\}/)?.[0] ?? "";
  assert(
    /markSeen\(props\.entryId\)/.test(dismissBody),
    "关闭气泡未 markSeen（试一试必须同关闭一样记住已看过）",
  );
  assert(/tourRegistry\[resolvedTourId\.value\]/.test(tryBody), "tryTour 未从 tourRegistry 取步骤");
  assert(
    /startTour\([^)]*,\s*true\s*\)/.test(tryBody),
    "tryTour 未以 recordProgress=true 启动教学",
  );
  assert(/if \(!canTryTour\.value\) return/.test(tryBody), "tryTour 未按开关/有效 id 守卫");
  assert(/canTryTour[\s\S]*?tourAllowed/.test(hintBubble), "tourEnabled=false 时按钮未隐藏");
  assert(!/setTimeout/.test(hintBubble), "HintBubble 含 setTimeout 自动播放");
  assert(
    (hintBubble.match(/startTour\s*\(/g) ?? []).length === 1,
    "HintBubble 只允许 tryTour 内一处 startTour 调用",
  );
  const mounted = hintBubble.match(/onMounted\(\(\) => \{[\s\S]*?\n\}\)/)?.[0] ?? "";
  assert(mounted.length > 0, "未找到 HintBubble 的 onMounted 钩子");
  assert(!/startTour\s*\(/.test(mounted), "onMounted 不得自动播放教学");

  assert(
    /<HintBubble entry-id="navigator" tour-id="quickstart"/.test(navigatorView),
    "Navigator 气泡未把试一试映射到 quickstart 主线",
  );
  for (const rel of [
    "src/views/AiView.vue",
    "src/views/ProjectsView.vue",
    "src/views/ProvidersView.vue",
    "src/views/WorkGroupsView.vue",
  ]) {
    const source = read(rel);
    const bubbles = [...source.matchAll(/<HintBubble\b[^>]*\/?>/g)].map((m) => m[0]);
    assert(bubbles.length > 0, `${rel} 没有 HintBubble`);
    for (const bubble of bubbles) {
      assert(!/tour-id=/.test(bubble), `${rel} 不应覆盖 tourId: ${bubble}`);
    }
  }
  return "5 处气泡：Navigator→quickstart、其余回退 entry.tourId；点击才播、recordProgress=true";
});

/* ----------------------------------------------------------------- 18 ---- */
check("零自动播放：src 无 useFirstRunTour 残留，也无 setTimeout 包围的 startTour", () => {
  assert(!existsSync(join(root, "src/help/useFirstRunTour.ts")), "useFirstRunTour.ts 仍存在");
  const sources = [
    ...walk(join(root, "src"), ".ts"),
    ...walk(join(root, "src"), ".vue"),
  ];
  for (const file of sources) {
    const source = readFileSync(file, "utf8");
    assert(
      !/useFirstRunTour/.test(source),
      `src 残留 useFirstRunTour 引用: ${relative(root, file)}`,
    );
  }
  assert(!/useFirstRunTour/.test(appShell), "AppShell 仍 import/调用 useFirstRunTour");
  for (const file of sources) {
    const source = readFileSync(file, "utf8");
    for (const match of source.matchAll(/setTimeout\s*\(/g)) {
      const tail = source.slice(match.index, match.index + 900);
      assert(
        !/startTour\s*\(/.test(tail),
        `检测到 setTimeout 包围的 startTour 自动播放: ${relative(root, file)}`,
      );
    }
  }
  return `${sources.length} 个源文件零残留；零 setTimeout→startTour 自动播放`;
});

/* ------------------------------------------------------------------------- */
console.log(`\n${passed} passed, ${failures.length} failed.`);
if (failures.length) {
  console.log("\nFailures:");
  for (const failure of failures) console.log(`  - ${failure.name}: ${failure.message}`);
  process.exitCode = 1;
}
