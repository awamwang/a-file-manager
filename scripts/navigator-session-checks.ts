// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * Navigator 会话（grill #1 / Q7）：钉住现有 navigatorBrowse 导出行为。
 * Run via: node --import ./scripts/node-ts-resolve.mjs --experimental-strip-types scripts/navigator-session-checks.ts
 */
import assert from "node:assert/strict";
import {
  __clearInvokeHandlers,
  __setInvokeHandler,
} from "./mocks/tauri-core.mjs";
import {
  activeWorkGroupId,
  applyLiveOpenState,
  closeNavigatorTab,
  currentPath,
  listError,
  listing,
  liveOpenState,
  navigateTo,
  newNavigatorTab,
  openOrFocusNavigatorTab,
  restoreWorkGroupOpenState,
  stripTabs,
  switchNavigatorTab,
} from "../src/state/navigatorBrowse.ts";
import {
  openStateFromPaths,
  type WorkGroupOpenState,
} from "../src/state/openState/model.ts";
import { GUIDE_PATH } from "../src/state/guide/index.ts";

function stubListOk(): void {
  __setInvokeHandler("navigator_list", async (args: { path?: string }) => {
    const path = String(args?.path ?? "").trim() || "C:\\";
    return {
      path,
      entries: [],
      dirCount: 0,
      fileCount: 0,
    };
  });
}

function resetSession(): void {
  __clearInvokeHandlers();
  stubListOk();
  applyLiveOpenState(null);
  activeWorkGroupId.value = "";
  currentPath.value = "";
  listing.value = null;
  listError.value = null;
}

resetSession();

// —— 标签：开 / 切 / 关 ——
applyLiveOpenState(
  openStateFromPaths(["C:\\Temp\\afm-a", "C:\\Temp\\afm-b"]),
);
assert.equal(stripTabs.value.length, 2);
assert.equal(liveOpenState.value.currentTabGroupIndex, 0);

await newNavigatorTab(GUIDE_PATH);
assert.equal(stripTabs.value.length, 3);
assert.equal(liveOpenState.value.currentTabGroupIndex, 2);
assert.equal(
  liveOpenState.value.tabGroups[2]?.[0]?.type,
  "guide",
);
assert.equal(currentPath.value, GUIDE_PATH);

await switchNavigatorTab(0);
assert.equal(liveOpenState.value.currentTabGroupIndex, 0);
// reveal 异步失败也不应回滚 strip（AIA-225）
assert.equal(stripTabs.value.length, 3);

await closeNavigatorTab(2);
assert.equal(stripTabs.value.length, 2);
assert.ok(
  liveOpenState.value.tabGroups.every(
    (group) => group[0]?.type !== "guide",
  ),
);

await openOrFocusNavigatorTab("C:\\Temp\\afm-b");
assert.equal(liveOpenState.value.currentTabGroupIndex, 1);
assert.equal(stripTabs.value[1]?.path.toLowerCase(), "c:\\temp\\afm-b");

// —— WG 静默切换 strip（restoreWorkGroupOpenState）——
const wgState: WorkGroupOpenState = openStateFromPaths([
  "D:\\WorkGroups\\demo",
  "D:\\WorkGroups\\tools",
]);
await restoreWorkGroupOpenState("wg-demo", wgState);
assert.equal(activeWorkGroupId.value, "wg-demo");
assert.equal(stripTabs.value.length, 2);
assert.equal(
  stripTabs.value[0]?.path.toLowerCase(),
  "d:\\workgroups\\demo",
);
assert.equal(liveOpenState.value.currentTabGroupIndex, 0);

// —— navigateTo / listing 失败：路径与错误进会话，不留下陈旧 listing ——
__setInvokeHandler("navigator_list", async () => {
  throw new Error("ENOENT: missing fixture path");
});
await navigateTo("C:\\Temp\\afm-missing-xyz");
assert.equal(listing.value, null);
assert.ok(listError.value?.includes("ENOENT"));
assert.equal(currentPath.value, "C:\\Temp\\afm-missing-xyz");
assert.equal(
  stripTabs.value[liveOpenState.value.currentTabGroupIndex]?.path.toLowerCase(),
  "c:\\temp\\afm-missing-xyz",
);

// —— navigateTo Guide：无 invoke，清空 listing ——
stubListOk();
await navigateTo(GUIDE_PATH);
assert.equal(currentPath.value, GUIDE_PATH);
assert.equal(listing.value, null);
assert.equal(listError.value, null);

// FIL-25：已在引导页时再次 navigateTo 不得克隆 Open State（路径栏误点根屑）。
{
  const before = liveOpenState.value;
  await navigateTo(GUIDE_PATH);
  assert.equal(currentPath.value, GUIDE_PATH);
  assert.equal(liveOpenState.value, before);
}

console.log("navigator-session-checks: ok");
