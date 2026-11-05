<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, onBeforeMount, onMounted, onUnmounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import CommandPanel from "../command/CommandPanel.vue";
import ToastHost from "./ToastHost.vue";
import FluentIcon from "../icons/FluentIcon.vue";
import { toastError, toastInfo, toastSuccess } from "../../state/toast";
import { SIDE_NAV_ITEMS, isSideNavActive, sideNavIcon } from "../../shell/sideNav";
import { buildAppCommands } from "../../commands/registry";
import { toggleCommandPanel } from "../../state/commandPanel";
import { requestGlobalSearchFocus } from "../../state/globalSearch";
import {
  dispatchShortcutEvent,
  registerShortcutHandler,
  unregisterShortcutHandler,
} from "../../shortcuts/index.ts";
import {
  bootstrapNavigatorOpenState,
  closeNavigatorTab,
  currentPath,
  flushOpenStatePersist,
  goUp,
  listError,
  listLoading,
  listing,
  liveOpenState,
  navigateTo,
  openOrFocusNavigatorTab,
  newNavigatorTab,
  reorderNavigatorTabs,
  stripTabs,
  switchNavigatorTab,
  switchNavigatorTabAdjacent,
} from "../../state/navigatorBrowse";
import {
  beginHistoryNavigation,
  buildNavigatorBreadcrumbs,
  canGoBack,
  canGoForward,
  endHistoryNavigation,
  popBack,
  popForward,
} from "../../state/navHistory";
import { favoritesDoc, loadFavorites } from "../../state/favorites";
import { quickAccessList, loadVisitStats } from "../../state/visitStats";
import {
  GUIDE_PATH,
  drivesList,
  isGuidePath,
  loadNavigatorPlaces,
  resolveHomePath,
} from "../../state/navigatorGuide";
import { dualPaneEnabled, toggleDualPane } from "../../state/dualPane";
import {
  loadNavigatorViewMode,
  NAVIGATOR_VIEW_MODES,
  navigatorViewMode,
  setNavigatorViewMode,
  viewModeLabelKey,
  type NavigatorViewMode,
} from "../../state/navigatorViewMode";
import {
  activeWorkGroup,
  dismissWorkGroupSoftNotice,
  loadWorkGroupsChrome,
  openActiveMembersIntoOpenState,
  saveActiveMembersFromTabsHard,
  silentSwitchWorkGroup,
  workGroupChromeBusy,
  workGroupChromeError,
  workGroupsDoc,
  workGroupSoftNotice,
} from "../../state/workGroupChrome";
import {
  hideMainWindowToTray,
  loadCloseBehavior,
  permitWindowClose,
  persistCloseBehavior,
  quitApplication,
  type CloseBehavior,
} from "../../state/closeBehavior";
import {
  clearNavigatorFilter,
  focusNavigatorFilter,
  navigatorFilterQuery,
} from "../../state/navigatorFilter";

const { t, locale } = useI18n();

type HealthPayload = {
  ok: boolean;
  name: string;
  version: string;
  identifier: string;
};

type SettingsDocument = {
  version: number;
  values: Record<string, unknown>;
};

const route = useRoute();
const router = useRouter();

const addressDraft = ref("");
const addressEditing = ref(false);
const activeTab = ref("navigator");
const health = ref<HealthPayload | null>(null);
const healthError = ref<string | null>(null);
const isMaximized = ref(false);
const maxHover = ref(false);
const placesMenuOpen = ref(false);
const workGroupMenuOpen = ref(false);
const viewModeMenuOpen = ref(false);
const crumbMenuIndex = ref<number | null>(null);
const crumbSiblings = ref<Array<{ name: string; path: string }>>([]);
const crumbMenuLoading = ref(false);
const crumbMenuError = ref<string | null>(null);
const crumbMenuActivePath = ref<string | null>(null);
const crumbMenuStyle = ref<Record<string, string>>({});
const closeConfirmOpen = ref(false);
const closeConfirmRemember = ref(true);
const closeConfirmBusy = ref(false);
const unlisteners: UnlistenFn[] = [];

const breadcrumbs = computed(() => {
  void locale.value;
  return buildNavigatorBreadcrumbs(currentPath.value, t("guide.rootLabel"));
});
const isNavigatorRoute = computed(
  () => route.path === "/navigator" || route.path === "/",
);
const showingGuide = computed(() => isGuidePath(currentPath.value));

function togglePlacesMenu() {
  workGroupMenuOpen.value = false;
  viewModeMenuOpen.value = false;
  placesMenuOpen.value = !placesMenuOpen.value;
}

function closePlacesMenu() {
  placesMenuOpen.value = false;
}

function toggleWorkGroupMenu() {
  placesMenuOpen.value = false;
  viewModeMenuOpen.value = false;
  workGroupMenuOpen.value = !workGroupMenuOpen.value;
  if (workGroupMenuOpen.value) {
    void loadWorkGroupsChrome();
  }
}

function closeWorkGroupMenu() {
  workGroupMenuOpen.value = false;
}

function toggleViewModeMenu() {
  placesMenuOpen.value = false;
  workGroupMenuOpen.value = false;
  viewModeMenuOpen.value = !viewModeMenuOpen.value;
}

function closeViewModeMenu() {
  viewModeMenuOpen.value = false;
}

function chooseViewMode(mode: NavigatorViewMode) {
  void setNavigatorViewMode(mode);
  closeViewModeMenu();
}

function closeCrumbMenu() {
  crumbMenuIndex.value = null;
  crumbSiblings.value = [];
  crumbMenuLoading.value = false;
  crumbMenuError.value = null;
  crumbMenuActivePath.value = null;
  crumbMenuStyle.value = {};
}

function positionCrumbMenu(anchor: HTMLElement) {
  const rect = anchor.getBoundingClientRect();
  const menuWidth = Math.min(288, window.innerWidth * 0.7);
  let left = rect.left;
  if (left + menuWidth > window.innerWidth - 8) {
    left = Math.max(8, window.innerWidth - menuWidth - 8);
  }
  crumbMenuStyle.value = {
    position: "fixed",
    top: `${Math.round(rect.bottom + 4)}px`,
    left: `${Math.round(left)}px`,
    zIndex: "120",
  };
}

function closeChromeMenus() {
  closePlacesMenu();
  closeWorkGroupMenu();
  closeViewModeMenu();
  closeCrumbMenu();
}

function pathKey(path: string): string {
  return path.replace(/\//g, "\\").replace(/\\+$/, "").toLowerCase();
}

async function toggleCrumbMenu(
  index: number,
  parentPath: string,
  event?: MouseEvent,
) {
  closePlacesMenu();
  closeWorkGroupMenu();
  if (crumbMenuIndex.value === index) {
    closeCrumbMenu();
    return;
  }
  const anchor =
    event?.currentTarget instanceof HTMLElement
      ? event.currentTarget
      : document.querySelector<HTMLElement>(`[data-crumb-chevron="${index}"]`);
  if (anchor) positionCrumbMenu(anchor);
  crumbMenuIndex.value = index;
  crumbSiblings.value = [];
  crumbMenuError.value = null;
  crumbMenuLoading.value = true;
  const crumb = breadcrumbs.value[index];
  const next = breadcrumbs.value[index + 1];
  crumbMenuActivePath.value = next?.path ?? null;

  // Root segment → Windows-style places: Home, drives, Recent, Favorites.
  if (crumb?.kind === "root") {
    try {
      await loadNavigatorPlaces();
      const home = await resolveHomePath();
      const items: Array<{ name: string; path: string }> = [
        { name: t("chrome.home"), path: home },
      ];
      for (const drive of drivesList.value) {
        items.push({
          name: t("guide.drive", { letter: drive.letter }),
          path: drive.path,
        });
      }
      for (const recent of quickAccessList.value) {
        items.push({
          name: `${t("guide.sections.recent")}: ${quickAccessLabel(recent.path)}`,
          path: recent.path,
        });
      }
      for (const fav of favoritesDoc.value.favorites) {
        items.push({
          name: `${t("favorites.label")}: ${favoriteLabel(fav.path)}`,
          path: fav.path,
        });
      }
      crumbSiblings.value = items;
    } catch (err) {
      crumbMenuError.value = err instanceof Error ? err.message : String(err);
    } finally {
      crumbMenuLoading.value = false;
    }
    return;
  }

  try {
    const listing = await invoke<{
      entries: Array<{ name: string; path: string; kind: string }>;
    }>("navigator_list", { path: parentPath });
    crumbSiblings.value = listing.entries
      .filter((entry) => entry.kind === "dir")
      .map((entry) => ({ name: entry.name, path: entry.path }));
  } catch (err) {
    crumbMenuError.value = err instanceof Error ? err.message : String(err);
  } finally {
    crumbMenuLoading.value = false;
  }
}

async function onCrumbSibling(path: string) {
  closeCrumbMenu();
  addressEditing.value = false;
  await ensureNavigatorRoute();
  await navigateTo(path);
}

async function openPlacesPath(path: string) {
  closePlacesMenu();
  await ensureNavigatorRoute();
  await openOrFocusNavigatorTab(path);
}

/** Home → Settings `core.defaultLocation`, else OS USERPROFILE via navigator_default_path. */
async function goHome() {
  closeChromeMenus();
  await ensureNavigatorRoute();
  try {
    const doc = await invoke<SettingsDocument>("settings_get");
    const configured = String(doc.values?.["core.defaultLocation"] ?? "").trim();
    if (configured) {
      await openOrFocusNavigatorTab(configured);
      return;
    }
  } catch {
    /* fall through to OS default */
  }
  const path = await invoke<string>("navigator_default_path");
  await openOrFocusNavigatorTab(path);
}

async function onSilentSwitchWorkGroup(targetId: string) {
  closeWorkGroupMenu();
  await ensureNavigatorRoute();
  const result = await silentSwitchWorkGroup(targetId);
  if (result?.requireSavePrompt || workGroupChromeError.value === "REQUIRE_SAVE_PROMPT") {
    workGroupChromeError.value = t("workGroups.errorSavePrompt");
  }
}

async function onSaveMembersFromTabs() {
  closeWorkGroupMenu();
  await saveActiveMembersFromTabsHard();
}

async function onOpenMembersIntoOpenState() {
  closeWorkGroupMenu();
  await ensureNavigatorRoute();
  const opened = await openActiveMembersIntoOpenState();
  if (opened === null) return;
  if (opened.length) {
    toastSuccess(t("workGroups.openMembersDone", { count: opened.length }));
  } else if (!(activeWorkGroup.value?.memberPaths.length)) {
    toastInfo(t("workGroups.openMembersEmpty"));
  } else {
    toastInfo(t("workGroups.openMembersNone"));
  }
}

async function openWorkGroupsPage() {
  closeWorkGroupMenu();
  await router.push("/work-groups");
}

async function ensureNavigatorThenGoUp() {
  if (route.path !== "/navigator") {
    await router.push("/navigator");
  }
  await goUp();
}

const panelCommands = computed(() => {
  void locale.value;
  return buildAppCommands({
    router,
    ensureNavigatorThenGoUp,
    refreshHealth: loadHealth,
    t: (key) => t(key),
  });
});

function focusAddressBar() {
  addressEditing.value = true;
  addressDraft.value = currentPath.value;
  requestAnimationFrame(() => {
    const input = document.getElementById("address-input");
    if (input instanceof HTMLInputElement) {
      input.focus();
      input.select();
    }
  });
}

async function onGoBack() {
  const target = popBack(currentPath.value);
  if (!target) return;
  await ensureNavigatorRoute();
  beginHistoryNavigation();
  try {
    await navigateTo(target);
  } finally {
    endHistoryNavigation();
  }
}

async function onGoForward() {
  const target = popForward(currentPath.value);
  if (!target) return;
  await ensureNavigatorRoute();
  beginHistoryNavigation();
  try {
    await navigateTo(target);
  } finally {
    endHistoryNavigation();
  }
}

async function onBreadcrumb(path: string) {
  addressEditing.value = false;
  await ensureNavigatorRoute();
  await navigateTo(path);
}

async function onRootCrumb() {
  addressEditing.value = false;
  await ensureNavigatorRoute();
  await navigateTo(GUIDE_PATH);
}

function favoriteLabel(path: string): string {
  const parts = path.replace(/\\/g, "/").replace(/\/+$/, "").split("/");
  return parts[parts.length - 1] || path;
}

function quickAccessLabel(path: string): string {
  return favoriteLabel(path);
}

function onGlobalKeydown(event: KeyboardEvent) {
  if (event.key === "Escape" && closeConfirmOpen.value) {
    cancelCloseConfirm();
    event.preventDefault();
    return;
  }
  if (
    event.key === "Escape" &&
    isNavigatorRoute.value &&
    navigatorFilterQuery.value.trim()
  ) {
    clearNavigatorFilter();
    event.preventDefault();
    return;
  }
  if (
    event.key === "Escape" &&
    (placesMenuOpen.value ||
      workGroupMenuOpen.value ||
      viewModeMenuOpen.value ||
      crumbMenuIndex.value !== null)
  ) {
    closeChromeMenus();
    event.preventDefault();
    return;
  }
  if (dispatchShortcutEvent(event)) {
    event.preventDefault();
  }
}

function onDocumentPointerDown(event: PointerEvent) {
  if (
    !placesMenuOpen.value &&
    !workGroupMenuOpen.value &&
    !viewModeMenuOpen.value &&
    crumbMenuIndex.value === null
  ) {
    return;
  }
  const target = event.target;
  if (!(target instanceof Node)) return;
  const placesRoot = document.getElementById("navigator-places-menu");
  const wgRoot = document.getElementById("navigator-work-group-menu");
  const viewRoot = document.getElementById("navigator-view-mode-menu");
  const crumbBar = document.getElementById("navigator-crumb-menu");
  const crumbFlyout = document.getElementById("navigator-crumb-siblings");
  if (placesMenuOpen.value && placesRoot && !placesRoot.contains(target)) {
    closePlacesMenu();
  }
  if (workGroupMenuOpen.value && wgRoot && !wgRoot.contains(target)) {
    closeWorkGroupMenu();
  }
  if (viewModeMenuOpen.value && viewRoot && !viewRoot.contains(target)) {
    closeViewModeMenu();
  }
  if (crumbMenuIndex.value !== null) {
    const inBar = crumbBar?.contains(target);
    const inFlyout = crumbFlyout?.contains(target);
    if (!inBar && !inFlyout) closeCrumbMenu();
  }
}

function registerShellShortcuts() {
  registerShortcutHandler("commandPanel.toggle", () => {
    toggleCommandPanel();
  });
  registerShortcutHandler("discovery.globalSearch", () => {
    // On Navigator: focus in-folder toolbar search. Elsewhere: Discovery global search.
    if (route.path === "/navigator" || route.path === "/") {
      focusNavigatorFilter();
      return;
    }
    requestGlobalSearchFocus();
    if (route.path !== "/discovery") {
      void router.push("/discovery");
    }
  });
  registerShortcutHandler("navigator.goUp", () => {
    void ensureNavigatorThenGoUp();
  });
  registerShortcutHandler("navigator.newTab", () => {
    void onNewNavigatorTab();
  });
  registerShortcutHandler("navigator.nextTabGroup", () => {
    void onSwitchNavigatorTabAdjacent("next");
  });
  registerShortcutHandler("navigator.prevTabGroup", () => {
    void onSwitchNavigatorTabAdjacent("previous");
  });
  registerShortcutHandler("navigator.back", () => {
    void onGoBack();
  });
  registerShortcutHandler("navigator.forward", () => {
    void onGoForward();
  });
  registerShortcutHandler("navigator.backCtrlAlt", () => {
    void onGoBack();
  });
  registerShortcutHandler("navigator.forwardCtrlAlt", () => {
    void onGoForward();
  });
  registerShortcutHandler("chrome.focusAddress", () => {
    focusAddressBar();
  });
  registerShortcutHandler("view.settings", () => {
    void router.push("/settings");
  });
  registerShortcutHandler("view.shortcuts", () => {
    void router.push("/settings/shortcuts");
  });
}

function unregisterShellShortcuts() {
  for (const id of [
    "commandPanel.toggle",
    "discovery.globalSearch",
    "navigator.goUp",
    "navigator.newTab",
    "navigator.nextTabGroup",
    "navigator.prevTabGroup",
    "navigator.back",
    "navigator.forward",
    "navigator.backCtrlAlt",
    "navigator.forwardCtrlAlt",
    "chrome.focusAddress",
    "view.settings",
    "view.shortcuts",
  ]) {
    unregisterShortcutHandler(id);
  }
}

const navItems = SIDE_NAV_ITEMS;

watch(
  currentPath,
  (path) => {
    addressDraft.value = path;
  },
  { immediate: true },
);

watch(isNavigatorRoute, (onNavigator) => {
  if (!onNavigator) {
    closeChromeMenus();
    clearNavigatorFilter();
  }
});

watch(workGroupSoftNotice, (notice) => {
  if (!notice) return;
  toastInfo(
    t("workGroups.softSync", { paths: notice.openedMemberPaths.join(", ") }),
  );
  dismissWorkGroupSoftNotice();
});

watch(workGroupChromeError, (err) => {
  if (!err || err === "REQUIRE_SAVE_PROMPT") return;
  toastError(err);
});

/** Files-like primary status (item counts) — left side of the shell status bar. */
const browseStatusText = computed(() => {
  void locale.value;
  if (route.path !== "/navigator") return "";
  if (showingGuide.value) return t("guide.status");
  if (listLoading.value) return t("chrome.navigatorLoading");
  if (listError.value) return t("chrome.navigatorError");
  if (listing.value) {
    return t("chrome.navigatorCounts", {
      dirs: listing.value.dirCount,
      files: listing.value.fileCount,
    });
  }
  return t("navigator.noPath");
});

const healthStatusText = computed(() => {
  void locale.value;
  if (healthError.value) return t("chrome.healthError");
  if (health.value?.ok) {
    return t("chrome.healthOk", { version: health.value.version });
  }
  return t("chrome.healthPending");
});

async function loadHealth() {
  healthError.value = null;
  try {
    health.value = await invoke<HealthPayload>("health_get");
  } catch (err) {
    health.value = null;
    healthError.value = err instanceof Error ? err.message : String(err);
  }
}

async function submitAddress() {
  addressEditing.value = false;
  if (route.path !== "/navigator") {
    await router.push("/navigator");
  }
  await navigateTo(addressDraft.value);
}

function onAddressBlur() {
  // Defer so click on Go / breadcrumb can run first.
  window.setTimeout(() => {
    if (document.activeElement?.id === "address-input") return;
    addressEditing.value = false;
    addressDraft.value = currentPath.value;
  }, 120);
}

async function onGoUp() {
  if (route.path !== "/navigator") {
    await router.push("/navigator");
  }
  await goUp();
}

async function refreshMaximized() {
  try {
    isMaximized.value = await getCurrentWindow().isMaximized();
  } catch {
    isMaximized.value = false;
  }
}

async function minimizeWindow() {
  try {
    await getCurrentWindow().minimize();
  } catch (err) {
    console.error("window minimize failed", err);
  }
}

async function toggleMaximizeWindow() {
  try {
    await getCurrentWindow().toggleMaximize();
    await refreshMaximized();
  } catch (err) {
    console.error("window toggleMaximize failed", err);
  }
}

async function closeWindow() {
  try {
    await getCurrentWindow().close();
  } catch (err) {
    console.error("window close failed", err);
  }
}

async function applyCloseAction(action: Exclude<CloseBehavior, "ask">) {
  closeConfirmBusy.value = true;
  try {
    if (closeConfirmRemember.value) {
      await persistCloseBehavior(action);
    }
    closeConfirmOpen.value = false;
    await flushOpenStatePersist();
    if (action === "tray") {
      await hideMainWindowToTray();
      return;
    }
    await quitApplication();
  } catch (err) {
    console.error("close action failed", err);
  } finally {
    closeConfirmBusy.value = false;
  }
}

function cancelCloseConfirm() {
  if (closeConfirmBusy.value) return;
  closeConfirmOpen.value = false;
}

async function handleCloseRequested(event: { preventDefault: () => void }) {
  if (permitWindowClose.value) {
    await flushOpenStatePersist();
    return;
  }
  event.preventDefault();
  const behavior = await loadCloseBehavior();
  if (behavior === "tray") {
    await flushOpenStatePersist();
    await hideMainWindowToTray();
    return;
  }
  if (behavior === "quit") {
    await flushOpenStatePersist();
    await quitApplication();
    return;
  }
  // Ask: flush now so a later force-kill still keeps tabs.
  await flushOpenStatePersist();
  closeConfirmRemember.value = true;
  closeConfirmOpen.value = true;
}

/** Drop plugin boot sentinel / any fixed ghost titlebar so Fluent caption receives clicks. */
function reclaimFluentTitlebar() {
  document.getElementById("afm-frame-boot")?.remove();
  for (const el of document.querySelectorAll("[data-tauri-frame-tb]")) {
    if (el.classList.contains("titlebar")) continue;
    const pos = window.getComputedStyle(el).position;
    if (pos === "fixed" || el.id === "afm-frame-boot") {
      el.remove();
    }
  }
}

function isNavActive(item: (typeof SIDE_NAV_ITEMS)[number]) {
  return isSideNavActive(item, route.path);
}

function openTab(id: string, path: string) {
  activeTab.value = id;
  void router.push(path);
}

function tabLabel(path: string) {
  if (isGuidePath(path)) return t("guide.tabName");
  const parts = path.replace(/\\/g, "/").replace(/\/+$/, "").split("/");
  return parts[parts.length - 1] || path;
}

async function ensureNavigatorRoute() {
  activeTab.value = "navigator";
  if (route.path !== "/navigator") {
    await router.push("/navigator");
  }
}

async function onSwitchNavigatorTab(groupIndex: number) {
  await ensureNavigatorRoute();
  await switchNavigatorTab(groupIndex);
}

async function onSwitchNavigatorTabAdjacent(direction: "next" | "previous") {
  await ensureNavigatorRoute();
  await switchNavigatorTabAdjacent(direction);
}

const tabDragFrom = ref<number | null>(null);
const tabDragOver = ref<number | null>(null);
/** Suppress click / +new-tab after a real drag (capture-loss can retarget click onto +). */
const tabDragMoved = ref(false);

/** Pointer-based reorder — HTML5 DnD is unreliable in WebView2 chrome areas. */
const TAB_DRAG_THRESHOLD_PX = 6;
const TAB_DRAG_CLICK_SUPPRESS_MS = 200;
let tabPointerId: number | null = null;
let tabPointerStartX = 0;
let tabPointerStartY = 0;
/** Stable identity across live Open State updates (never trust stale indices alone). */
let tabPointerFromId: string | null = null;
let tabReorderBusy = false;
let tabClickSuppressTimer: ReturnType<typeof setTimeout> | null = null;

function clearTabPointerListeners() {
  window.removeEventListener("pointermove", onTabPointerMove);
  window.removeEventListener("pointerup", onTabPointerUp);
  window.removeEventListener("pointercancel", onTabPointerUp);
  tabPointerId = null;
}

function armTabClickSuppress() {
  tabDragMoved.value = true;
  if (tabClickSuppressTimer != null) clearTimeout(tabClickSuppressTimer);
  tabClickSuppressTimer = setTimeout(() => {
    tabDragMoved.value = false;
    tabClickSuppressTimer = null;
  }, TAB_DRAG_CLICK_SUPPRESS_MS);
}

function onTabPointerDown(event: PointerEvent, groupIndex: number) {
  if (event.button !== 0) return;
  if (event.target instanceof Element && event.target.closest(".tab-close")) {
    return;
  }
  // Reset any half-finished gesture (duplicate window listeners → duplicate reorders).
  clearTabPointerListeners();
  const tab = stripTabs.value[groupIndex];
  if (!tab) return;

  tabPointerId = event.pointerId;
  tabPointerFromId = tab.id;
  tabPointerStartX = event.clientX;
  tabPointerStartY = event.clientY;
  tabDragFrom.value = groupIndex;
  tabDragOver.value = null;
  // Do not clear suppress mid-gesture if a prior drag just finished.
  if (tabClickSuppressTimer == null) {
    tabDragMoved.value = false;
  }
  // No setPointerCapture: Vue remount after reorder retargets click onto neighboring + tab.
  window.addEventListener("pointermove", onTabPointerMove);
  window.addEventListener("pointerup", onTabPointerUp);
  window.addEventListener("pointercancel", onTabPointerUp);
}

function tabIndexAtPoint(clientX: number, clientY: number): number | null {
  const nodes = document.querySelectorAll<HTMLElement>("[data-tab-group-index]");
  for (const node of nodes) {
    const rect = node.getBoundingClientRect();
    if (
      clientX >= rect.left &&
      clientX <= rect.right &&
      clientY >= rect.top &&
      clientY <= rect.bottom
    ) {
      const idx = Number(node.dataset.tabGroupIndex);
      return Number.isFinite(idx) ? idx : null;
    }
  }
  return null;
}

function currentFromIndex(): number {
  if (!tabPointerFromId) return -1;
  return stripTabs.value.findIndex((tab) => tab.id === tabPointerFromId);
}

function onTabPointerMove(event: PointerEvent) {
  if (tabPointerId !== event.pointerId || !tabPointerFromId) return;
  const dist = Math.hypot(
    event.clientX - tabPointerStartX,
    event.clientY - tabPointerStartY,
  );
  if (!tabDragMoved.value) {
    if (dist < TAB_DRAG_THRESHOLD_PX) return;
    tabDragMoved.value = true;
  }
  const from = currentFromIndex();
  tabDragFrom.value = from >= 0 ? from : null;
  const idx = tabIndexAtPoint(event.clientX, event.clientY);
  if (idx == null || idx === from) {
    tabDragOver.value = null;
    return;
  }
  tabDragOver.value = idx;
}

async function onTabPointerUp(event: PointerEvent) {
  if (tabPointerId !== event.pointerId) return;
  const fromId = tabPointerFromId;
  const to = tabDragOver.value;
  const moved = tabDragMoved.value;
  clearTabPointerListeners();
  tabDragFrom.value = null;
  tabDragOver.value = null;
  tabPointerFromId = null;

  if (moved) {
    // Prevent the post-drag click (often retargeted onto + new tab after DOM shuffle).
    event.preventDefault();
    armTabClickSuppress();
  }

  if (!moved || !fromId || to == null || tabReorderBusy) return;
  const from = stripTabs.value.findIndex((tab) => tab.id === fromId);
  if (from < 0 || from === to) return;
  tabReorderBusy = true;
  try {
    await reorderNavigatorTabs(from, to);
  } finally {
    tabReorderBusy = false;
  }
}

async function onTabClick(groupIndex: number) {
  if (tabDragMoved.value) return;
  await onSwitchNavigatorTab(groupIndex);
}

async function onCloseNavigatorTab(event: Event, groupIndex: number) {
  event.stopPropagation();
  event.preventDefault();
  await ensureNavigatorRoute();
  await closeNavigatorTab(groupIndex);
}

async function onNewNavigatorTab() {
  // Drag-end click retarget onto + must not spawn duplicate tabs.
  if (tabDragMoved.value) return;
  await ensureNavigatorRoute();
  await newNavigatorTab();
}

onBeforeMount(() => {
  reclaimFluentTitlebar();
});

function onVisibilityFlush() {
  if (document.visibilityState === "hidden") {
    void flushOpenStatePersist();
  }
}

onMounted(() => {
  reclaimFluentTitlebar();
  void loadHealth();
  void refreshMaximized();
  void bootstrapNavigatorOpenState();
  void loadFavorites().catch(() => undefined);
  void loadVisitStats().catch(() => undefined);
  void loadNavigatorPlaces().catch(() => undefined);
  void loadNavigatorViewMode().catch(() => undefined);
  void loadWorkGroupsChrome().catch(() => undefined);
  void loadCloseBehavior().catch(() => undefined);
  registerShellShortcuts();
  window.addEventListener("keydown", onGlobalKeydown);
  document.addEventListener("pointerdown", onDocumentPointerDown, true);
  document.addEventListener("visibilitychange", onVisibilityFlush);

  const win = getCurrentWindow();
  void win.onCloseRequested((event) => {
    void handleCloseRequested(event);
  }).then((unlisten) => {
    unlisteners.push(unlisten);
  });
  void win.onResized(() => {
    void refreshMaximized();
  }).then((unlisten) => {
    unlisteners.push(unlisten);
  });

  // Snap overlay owns the maximize hit-test (HTMAXBUTTON). Wire hover/click back to Fluent UI.
  void listen("tauri-frame://snap/click", () => {
    void toggleMaximizeWindow();
  }).then((unlisten) => unlisteners.push(unlisten));
  void listen("tauri-frame://snap/mouseenter", () => {
    maxHover.value = true;
  }).then((unlisten) => unlisteners.push(unlisten));
  void listen("tauri-frame://snap/mouseleave", () => {
    maxHover.value = false;
  }).then((unlisten) => unlisteners.push(unlisten));
});

onUnmounted(() => {
  window.removeEventListener("keydown", onGlobalKeydown);
  document.removeEventListener("pointerdown", onDocumentPointerDown, true);
  document.removeEventListener("visibilitychange", onVisibilityFlush);
  void flushOpenStatePersist();
  unregisterShellShortcuts();
  for (const unlisten of unlisteners) {
    unlisten();
  }
});
</script>

<template>
  <div class="shell fluent-chrome">
    <header class="titlebar" data-tauri-frame-tb :aria-label="t('chrome.titleBar')">
      <div
        class="titlebar-drag"
        data-tauri-drag-region
        :title="t('chrome.titleBar')"
        @dblclick.prevent="toggleMaximizeWindow"
      >
        <span class="brand" data-tauri-drag-region>{{ t("app.name") }}</span>
        <span class="caption-meta" data-tauri-drag-region>{{ t("app.captionMeta") }}</span>
      </div>
      <div class="titlebar-actions" role="group" :aria-label="t('chrome.commands')">
        <button
          type="button"
          class="fluent-control titlebar-action"
          :title="t('chrome.commandsTitle')"
          :aria-label="t('chrome.commands')"
          aria-keyshortcuts="Control+Shift+P"
          @click="toggleCommandPanel"
        >
          <FluentIcon name="commands" size="sm" />
        </button>
      </div>
      <div class="window-controls" :aria-label="t('chrome.windowControls')">
        <button
          id="frame-tb-minimize"
          type="button"
          class="win-btn"
          :aria-label="t('chrome.minimize')"
          :title="t('chrome.minimize')"
          @click.stop.prevent="minimizeWindow"
        >
          <FluentIcon name="minimize" size="sm" />
        </button>
        <!--
          Snap: native HTMAXBUTTON overlay (tauri-plugin-frame) covers this button.
          :hover / click come from tauri-frame://snap/* events — see script.
        -->
        <button
          id="frame-tb-maximize"
          type="button"
          class="win-btn"
          :class="{ 'win-btn--hover': maxHover }"
          :aria-label="isMaximized ? t('chrome.restore') : t('chrome.maximize')"
          :title="isMaximized ? t('chrome.restore') : t('chrome.maximize')"
          @click.stop.prevent="toggleMaximizeWindow"
        >
          <FluentIcon :name="isMaximized ? 'restore' : 'maximize'" size="sm" />
        </button>
        <button
          id="frame-tb-close"
          type="button"
          class="win-btn win-btn--close"
          :aria-label="t('chrome.close')"
          :title="t('chrome.close')"
          @click.stop.prevent="closeWindow"
        >
          <FluentIcon name="close" size="sm" />
        </button>
      </div>
    </header>

    <div class="body">
      <nav class="sidenav fluent-layer-alt" :aria-label="t('chrome.sideNav')">
        <button
          v-for="item in navItems"
          :key="item.id"
          type="button"
          class="fluent-list-row sidenav-item"
          :class="{
            'sidenav-item--active': isNavActive(item),
            'sidenav-item--later': item.later,
          }"
          :disabled="item.later"
          :title="item.later ? t('chrome.later') : undefined"
          @click="item.to && router.push(item.to)"
        >
          <FluentIcon :name="sideNavIcon(item.id)" size="md" class="sidenav-icon" />
          <span class="sidenav-label">{{ t(item.labelKey) }}</span>
          <span v-if="item.later" class="later-badge">{{ t("chrome.later") }}</span>
        </button>
      </nav>

      <div class="workspace">
      <template v-if="isNavigatorRoute">
        <form
          class="navigator-chrome"
          role="toolbar"
          :aria-label="t('chrome.toolbar')"
          @submit.prevent="submitAddress"
        >
          <div id="navigator-work-group-menu" class="places-menu wg-menu">
            <button
              type="button"
              class="fluent-control wg-trigger"
              :class="{ 'wg-trigger--active': workGroupMenuOpen }"
              :title="t('chrome.workGroupMenu')"
              :aria-label="t('chrome.workGroupMenu')"
              :aria-expanded="workGroupMenuOpen"
              aria-haspopup="menu"
              :disabled="workGroupChromeBusy"
              @click.stop="toggleWorkGroupMenu"
            >
              <FluentIcon name="viewAll" size="sm" />
              <span class="wg-trigger-label">{{
                activeWorkGroup?.name || t("nav.workGroups")
              }}</span>
              <FluentIcon name="chevronDown" size="sm" class="places-chevron" />
            </button>
            <div
              v-if="workGroupMenuOpen"
              class="places-popover fluent-surface"
              role="menu"
              :aria-label="t('chrome.workGroupMenu')"
            >
              <p class="places-heading">{{ t("chrome.workGroupSwitch") }}</p>
              <button
                v-for="group in workGroupsDoc?.workGroups ?? []"
                :key="'wg-' + group.id"
                type="button"
                class="fluent-list-row places-item"
                :class="{ 'places-item--active': group.id === workGroupsDoc?.activeWorkGroupId }"
                role="menuitem"
                :disabled="workGroupChromeBusy || group.id === workGroupsDoc?.activeWorkGroupId"
                :title="group.id"
                @click="onSilentSwitchWorkGroup(group.id)"
              >
                <FluentIcon :name="group.isDefault ? 'home' : 'viewAll'" size="sm" />
                <span>{{ group.name }}</span>
              </button>
              <p v-if="!(workGroupsDoc?.workGroups.length)" class="places-empty">
                {{ t("chrome.workGroupEmpty") }}
              </p>

              <p class="places-heading places-heading--fav">{{ t("chrome.workGroupSave") }}</p>
              <button
                type="button"
                class="fluent-list-row places-item"
                role="menuitem"
                :disabled="workGroupChromeBusy || !activeWorkGroup?.memberPaths.length"
                @click="onOpenMembersIntoOpenState"
              >
                <FluentIcon name="folderOpen" size="sm" />
                <span>{{ t("workGroups.openMembersIntoOpenState") }}</span>
              </button>
              <button
                type="button"
                class="fluent-list-row places-item"
                role="menuitem"
                :disabled="workGroupChromeBusy || !activeWorkGroup"
                @click="onSaveMembersFromTabs"
              >
                <FluentIcon name="folder" size="sm" />
                <span>{{ t("workGroups.saveMembersFromTabs") }}</span>
              </button>
              <button
                type="button"
                class="fluent-list-row places-item"
                role="menuitem"
                @click="openWorkGroupsPage"
              >
                <FluentIcon name="settings" size="sm" />
                <span>{{ t("chrome.workGroupManage") }}</span>
              </button>
            </div>
          </div>

          <span class="navigator-chrome-sep" aria-hidden="true" />

          <button
            type="button"
            class="fluent-control nav-icon-btn"
            :disabled="!canGoBack"
            :title="t('chrome.back')"
            :aria-label="t('chrome.back')"
            @click="onGoBack"
          >
            <FluentIcon name="back" size="sm" />
          </button>
          <button
            type="button"
            class="fluent-control nav-icon-btn"
            :disabled="!canGoForward"
            :title="t('chrome.forward')"
            :aria-label="t('chrome.forward')"
            @click="onGoForward"
          >
            <FluentIcon name="forward" size="sm" />
          </button>
          <button
            type="button"
            class="fluent-control nav-icon-btn"
            :title="t('chrome.upTitle')"
            :aria-label="t('chrome.up')"
            @click="onGoUp"
          >
            <FluentIcon name="up" size="sm" />
          </button>

          <button
            id="navigator-home-btn"
            type="button"
            class="fluent-control nav-icon-btn"
            :title="t('chrome.home')"
            :aria-label="t('chrome.home')"
            @click="goHome"
          >
            <FluentIcon name="home" size="sm" />
          </button>

          <div id="navigator-places-menu" class="places-menu">
            <button
              type="button"
              class="fluent-control nav-icon-btn places-trigger"
              :class="{ 'nav-icon-btn--active': placesMenuOpen }"
              :title="t('chrome.placesMenu')"
              :aria-label="t('chrome.placesMenu')"
              :aria-expanded="placesMenuOpen"
              aria-haspopup="menu"
              @click.stop="togglePlacesMenu"
            >
              <FluentIcon name="star" size="sm" />
              <FluentIcon name="chevronDown" size="sm" class="places-chevron" />
            </button>
            <div
              v-if="placesMenuOpen"
              class="places-popover fluent-surface"
              role="menu"
              :aria-label="t('chrome.placesMenu')"
            >
              <p class="places-heading">{{ t("quickAccess.label") }}</p>
              <button
                v-for="item in quickAccessList"
                :key="'qa-' + item.path"
                type="button"
                class="fluent-list-row places-item"
                role="menuitem"
                :title="`${item.path} · ${t('quickAccess.openCount', { count: item.openCount })}`"
                @click="openPlacesPath(item.path)"
              >
                <FluentIcon :name="item.isDir ? 'folder' : 'document'" size="sm" />
                <span>{{ quickAccessLabel(item.path) }}</span>
              </button>
              <p v-if="!quickAccessList.length" class="places-empty">{{ t("quickAccess.empty") }}</p>

              <p class="places-heading places-heading--fav">{{ t("favorites.label") }}</p>
              <button
                v-for="fav in favoritesDoc.favorites"
                :key="'fav-' + fav.path"
                type="button"
                class="fluent-list-row places-item"
                role="menuitem"
                :title="fav.path"
                @click="openPlacesPath(fav.path)"
              >
                <FluentIcon :name="fav.isDir ? 'folder' : 'document'" size="sm" />
                <span>{{ favoriteLabel(fav.path) }}</span>
              </button>
              <p v-if="!favoritesDoc.favorites.length" class="places-empty">
                {{ t("favorites.empty") }}
              </p>
            </div>
          </div>

          <div id="navigator-view-mode-menu" class="places-menu">
            <button
              type="button"
              class="fluent-control nav-icon-btn places-trigger"
              :class="{ 'nav-icon-btn--active': viewModeMenuOpen }"
              :title="t('chrome.viewMode')"
              :aria-label="t('chrome.viewMode')"
              :aria-expanded="viewModeMenuOpen"
              aria-haspopup="menu"
              @click.stop="toggleViewModeMenu"
            >
              <FluentIcon name="view" size="sm" />
              <FluentIcon name="chevronDown" size="sm" class="places-chevron" />
            </button>
            <div
              v-if="viewModeMenuOpen"
              class="places-popover fluent-surface view-mode-popover"
              role="menu"
              :aria-label="t('chrome.viewMode')"
            >
              <button
                v-for="mode in NAVIGATOR_VIEW_MODES"
                :key="mode"
                type="button"
                class="fluent-list-row places-item"
                role="menuitemradio"
                :aria-checked="navigatorViewMode === mode"
                :class="{ 'places-item--active': navigatorViewMode === mode }"
                @click="chooseViewMode(mode)"
              >
                <FluentIcon
                  :name="mode === 'details' || mode.startsWith('icons') ? 'view' : 'viewAll'"
                  size="sm"
                />
                <span>{{ t(viewModeLabelKey(mode)) }}</span>
              </button>
            </div>
          </div>

          <button
            type="button"
            class="fluent-control nav-icon-btn"
            :class="{ 'nav-icon-btn--active': dualPaneEnabled }"
            :title="t('chrome.dualPane')"
            :aria-label="t('chrome.dualPane')"
            :aria-pressed="dualPaneEnabled"
            @click="toggleDualPane"
          >
            <FluentIcon name="dualPane" size="sm" force-svg />
          </button>

          <div
            v-if="!addressEditing"
            id="navigator-crumb-menu"
            class="breadcrumbs"
            role="navigation"
            :aria-label="t('chrome.breadcrumbs')"
            @click="focusAddressBar"
          >
            <template v-if="breadcrumbs.length">
              <div
                v-for="(crumb, index) in breadcrumbs"
                :key="crumb.path + String(index) + (crumb.kind ?? '')"
                class="crumb-seg"
                :class="{ 'crumb-seg--root': crumb.kind === 'root' }"
              >
                <button
                  type="button"
                  class="crumb"
                  :class="{ 'crumb--root': crumb.kind === 'root' }"
                  :title="crumb.kind === 'root' ? t('guide.rootTitle') : crumb.path"
                  @click.stop="
                    crumb.kind === 'root' ? onRootCrumb() : onBreadcrumb(crumb.path)
                  "
                >
                  <FluentIcon
                    v-if="crumb.kind === 'root'"
                    name="home"
                    size="sm"
                    class="crumb-root-icon"
                  />
                  <span class="crumb-label">{{ crumb.label }}</span>
                </button>
                <div class="crumb-chevron-wrap">
                  <button
                    type="button"
                    class="crumb-chevron"
                    :data-crumb-chevron="index"
                    :class="{ 'crumb-chevron--open': crumbMenuIndex === index }"
                    :title="
                      crumb.kind === 'root'
                        ? t('guide.rootMenu')
                        : t('chrome.crumbSiblings')
                    "
                    :aria-label="
                      crumb.kind === 'root'
                        ? t('guide.rootMenu')
                        : t('chrome.crumbSiblings')
                    "
                    :aria-expanded="crumbMenuIndex === index"
                    :aria-controls="
                      crumbMenuIndex === index ? 'navigator-crumb-siblings' : undefined
                    "
                    @click.stop="toggleCrumbMenu(index, crumb.path, $event)"
                  >
                    ›
                  </button>
                </div>
              </div>
            </template>
            <span v-else class="crumb-placeholder">{{ t("chrome.pathPlaceholder") }}</span>
          </div>
          <input
            v-else
            id="address-input"
            v-model="addressDraft"
            class="address-input"
            type="text"
            spellcheck="false"
            autocomplete="off"
            :placeholder="t('chrome.pathPlaceholder')"
            @blur="onAddressBlur"
            @keydown.escape.prevent="addressEditing = false"
          />
          <button type="submit" class="fluent-control address-go" :title="t('chrome.go')">
            {{ t("chrome.go") }}
          </button>

          <div
            id="navigator-filter-bar"
            class="navigator-filter-inline"
            role="search"
            :aria-label="t('navigator.filterToggle')"
          >
            <FluentIcon name="search" size="sm" class="navigator-filter-icon" />
            <input
              id="navigator-folder-filter"
              v-model="navigatorFilterQuery"
              class="fluent-control navigator-filter-input"
              type="search"
              :placeholder="t('navigator.filterPlaceholder')"
              :title="t('navigator.filterToggle')"
              spellcheck="false"
              @keydown.enter.prevent
              @keydown.escape.prevent="clearNavigatorFilter"
            />
            <button
              v-if="navigatorFilterQuery.trim()"
              type="button"
              class="fluent-control navigator-filter-clear"
              :title="t('navigator.filterClear')"
              :aria-label="t('navigator.filterClear')"
              @click="clearNavigatorFilter"
            >
              <FluentIcon name="close" size="sm" />
            </button>
          </div>
        </form>

        <Teleport to="body">
          <div
            v-if="crumbMenuIndex !== null"
            id="navigator-crumb-siblings"
            class="crumb-menu fluent-surface"
            role="menu"
            :aria-label="t('chrome.crumbSiblings')"
            :style="crumbMenuStyle"
            @click.stop
          >
            <p v-if="crumbMenuLoading" class="crumb-menu-status">
              {{ t("chrome.crumbSiblingsLoading") }}
            </p>
            <p v-else-if="crumbMenuError" class="crumb-menu-status crumb-menu-status--error">
              {{ crumbMenuError }}
            </p>
            <p v-else-if="!crumbSiblings.length" class="crumb-menu-status">
              {{ t("chrome.crumbSiblingsEmpty") }}
            </p>
            <button
              v-for="sibling in crumbSiblings"
              :key="sibling.path"
              type="button"
              class="crumb-menu-item"
              role="menuitem"
              :class="{
                'crumb-menu-item--active':
                  crumbMenuActivePath &&
                  pathKey(sibling.path) === pathKey(crumbMenuActivePath),
              }"
              :title="sibling.path"
              @click="onCrumbSibling(sibling.path)"
            >
              <FluentIcon name="folder" size="sm" />
              <span>{{ sibling.name }}</span>
            </button>
          </div>
        </Teleport>

        <div
          class="tabstrip"
          role="tablist"
          :aria-label="t('chrome.tabs')"
        >
          <div
            v-for="tab in stripTabs"
            :key="tab.id"
            class="tab"
            :class="{
              'tab--active': liveOpenState.currentTabGroupIndex === tab.groupIndex,
              'tab--drag-over': tabDragOver === tab.groupIndex,
              'tab--dragging': tabDragFrom === tab.groupIndex && tabDragMoved,
            }"
            role="tab"
            :data-tab-group-index="tab.groupIndex"
            :aria-selected="liveOpenState.currentTabGroupIndex === tab.groupIndex"
            :title="tab.path"
            @click="onTabClick(tab.groupIndex)"
            @pointerdown="onTabPointerDown($event, tab.groupIndex)"
          >
            <FluentIcon name="folder" size="sm" class="tab-icon" aria-hidden="true" />
            <span class="tab-label">{{ tab.name || tabLabel(tab.path) }}</span>
            <button
              type="button"
              class="tab-close"
              :aria-label="t('chrome.closeTab')"
              :title="t('chrome.closeTab')"
              @click="onCloseNavigatorTab($event, tab.groupIndex)"
              @pointerdown.stop
            >
              <FluentIcon name="close" size="sm" aria-hidden="true" />
            </button>
          </div>
          <button
            v-if="!stripTabs.length"
            type="button"
            role="tab"
            class="tab tab--active"
            :aria-selected="true"
            @click="openTab('navigator', '/navigator')"
          >
            <FluentIcon name="folderOpen" size="sm" />
            <span>{{ t("nav.navigator") }}</span>
          </button>
          <button
            type="button"
            class="tab tab--add"
            :title="t('chrome.newTab')"
            :aria-label="t('chrome.newTab')"
            @click="onNewNavigatorTab"
          >
            <FluentIcon name="add" size="sm" />
          </button>
        </div>

        </template>

        <main class="content" :class="{ 'content--navigator': isNavigatorRoute }">
          <slot />
        </main>
      </div>
    </div>

    <footer class="statusbar" :aria-label="t('chrome.status')">
      <span class="status-text status-primary">{{ browseStatusText }}</span>
      <div class="status-trailing">
        <span
          class="status-health"
          :class="{
            'fluent-status-text--down': Boolean(healthError),
            'fluent-status-text--ok': health?.ok && !healthError,
          }"
        >{{ healthStatusText }}</span>
        <button type="button" class="status-refresh" @click="loadHealth">
          {{ t("chrome.refreshHealth") }}
        </button>
      </div>
    </footer>

    <div
      v-if="closeConfirmOpen"
      id="close-confirm-dialog"
      class="close-confirm-backdrop"
      role="presentation"
      @click.self="cancelCloseConfirm"
    >
      <div
        class="close-confirm-dialog fluent-surface"
        role="dialog"
        aria-modal="true"
        :aria-label="t('chrome.closeConfirmTitle')"
      >
        <h2 class="close-confirm-title">{{ t("chrome.closeConfirmTitle") }}</h2>
        <p class="close-confirm-message">{{ t("chrome.closeConfirmMessage") }}</p>
        <label class="close-confirm-remember">
          <input v-model="closeConfirmRemember" type="checkbox" class="fluent-field-check" />
          {{ t("chrome.closeConfirmRemember") }}
        </label>
        <div class="close-confirm-actions">
          <button
            type="button"
            class="fluent-control"
            :disabled="closeConfirmBusy"
            @click="cancelCloseConfirm"
          >
            {{ t("chrome.closeConfirmCancel") }}
          </button>
          <button
            type="button"
            class="fluent-control"
            :disabled="closeConfirmBusy"
            @click="applyCloseAction('tray')"
          >
            {{ t("chrome.closeConfirmTray") }}
          </button>
          <button
            type="button"
            class="fluent-control fluent-control--accent"
            :disabled="closeConfirmBusy"
            @click="applyCloseAction('quit')"
          >
            {{ t("chrome.closeConfirmQuit") }}
          </button>
        </div>
      </div>
    </div>

    <ToastHost />
    <CommandPanel :commands="panelCommands" />
  </div>
</template>

<style scoped>
.shell {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 100%;
  background: var(--fluent-color-fill-subtle);
}

.titlebar {
  display: flex;
  align-items: stretch;
  height: var(--fluent-titlebar-height);
  flex-shrink: 0;
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  background: var(--fluent-color-fill-control);
  user-select: none;
}

.titlebar-drag {
  display: flex;
  flex: 1;
  align-items: center;
  gap: var(--fluent-space-md);
  padding-inline: var(--fluent-space-lg);
  min-width: 0;
}

.brand {
  font-size: var(--fluent-font-size-subtitle);
  font-weight: var(--fluent-font-weight-semibold);
}

.caption-meta {
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.window-controls {
  display: flex;
  flex-shrink: 0;
  position: relative;
  z-index: 2;
  pointer-events: auto;
  /* Must not be a drag region — caption buttons need real clicks. */
  -webkit-app-region: no-drag;
  app-region: no-drag;
}

.win-btn {
  width: var(--fluent-titlebar-button-width);
  height: var(--fluent-titlebar-height);
  border: 0;
  background: transparent;
  color: var(--fluent-color-text-primary);
  font-size: 12px;
  line-height: 1;
  cursor: default;
  pointer-events: auto;
  -webkit-app-region: no-drag;
  app-region: no-drag;
  transition: background-color 80ms ease;
}

.win-btn:hover,
.win-btn--hover {
  background: var(--fluent-color-fill-control-hover);
}

.win-btn:active {
  background: var(--fluent-color-fill-control-pressed);
}

.win-btn--close:hover {
  background: var(--fluent-color-status-danger);
  color: var(--fluent-color-text-on-accent);
}

.titlebar-actions {
  display: flex;
  align-items: center;
  flex-shrink: 0;
  padding-inline-end: var(--fluent-space-xs);
  -webkit-app-region: no-drag;
  app-region: no-drag;
}

.titlebar-action {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: var(--fluent-control-height-standard);
  height: var(--fluent-control-height-standard);
  min-width: var(--fluent-control-height-standard);
  min-height: var(--fluent-control-height-standard);
  padding: 0;
}

.navigator-chrome {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-xs);
  height: var(--fluent-addressbar-height);
  flex-shrink: 0;
  padding-inline: var(--fluent-space-md);
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  background: var(--fluent-color-fill-control);
}

.nav-icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: var(--fluent-control-height-standard);
  height: var(--fluent-control-height-standard);
  min-width: var(--fluent-control-height-standard);
  min-height: var(--fluent-control-height-standard);
  padding: 0;
  flex-shrink: 0;
}

.nav-icon-btn--active {
  background: var(--fluent-color-fill-selected);
}

.view-mode-group {
  display: inline-flex;
  flex-shrink: 0;
  gap: 1px;
}

.places-menu {
  position: relative;
  flex-shrink: 0;
}

.places-trigger {
  width: auto;
  min-width: var(--fluent-control-height-standard);
  padding-inline: var(--fluent-space-xs);
  gap: 2px;
}

.places-chevron {
  opacity: 0.75;
}

.places-popover {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  z-index: 40;
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 16rem;
  max-width: 22rem;
  max-height: min(70vh, 28rem);
  overflow: auto;
  padding: var(--fluent-space-sm);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  box-shadow: var(--fluent-shadow-flyout, 0 8px 24px rgba(0, 0, 0, 0.18));
}

.places-heading {
  margin: var(--fluent-space-xs) var(--fluent-space-xs) 2px;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
  font-weight: var(--fluent-font-weight-semibold);
}

.places-heading--fav {
  margin-top: var(--fluent-space-sm);
  padding-top: var(--fluent-space-sm);
  border-top: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
}

.places-item {
  gap: var(--fluent-space-sm);
  text-align: left;
  width: 100%;
  border: 0;
  background: transparent;
  color: var(--fluent-color-text-primary);
  cursor: pointer;
}

.places-item span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.places-empty {
  margin: 0;
  padding: var(--fluent-space-xs) var(--fluent-space-sm);
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.places-item--active {
  background: var(--fluent-color-fill-selected);
  font-weight: var(--fluent-font-weight-semibold);
}

.navigator-chrome-sep {
  display: inline-block;
  width: 1px;
  height: 1.1rem;
  margin-inline: var(--fluent-space-xs);
  background: var(--fluent-color-stroke-divider);
  flex-shrink: 0;
}

.wg-menu {
  flex-shrink: 0;
}

.wg-trigger {
  display: inline-flex;
  align-items: center;
  gap: var(--fluent-space-xs);
  max-width: 12rem;
  height: var(--fluent-control-height-standard);
  min-height: var(--fluent-control-height-standard);
  padding-inline: var(--fluent-space-sm);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-subtle);
  color: var(--fluent-color-text-primary);
  font-weight: var(--fluent-font-weight-semibold);
}

.wg-trigger:hover:not(:disabled) {
  background: var(--fluent-color-fill-subtle-secondary);
}

.wg-trigger--active {
  background: var(--fluent-color-fill-selected);
  border-color: var(--fluent-color-stroke-accent-soft, var(--fluent-color-fill-accent));
}

.wg-trigger-label {
  max-width: 7rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: var(--fluent-font-size-caption);
}

.breadcrumbs {
  flex: 1;
  display: flex;
  align-items: center;
  flex-wrap: nowrap;
  overflow: hidden;
  min-height: var(--fluent-control-height-standard);
  padding-inline: var(--fluent-space-sm);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-subtle);
  cursor: text;
}

.breadcrumbs:hover {
  background: var(--fluent-color-fill-subtle-secondary);
}

.crumb-seg {
  position: relative;
  display: inline-flex;
  align-items: center;
  flex-shrink: 1;
  min-width: 0;
}

.crumb {
  display: inline-flex;
  align-items: center;
  max-width: 10rem;
  border: 0;
  padding: 0.1rem 0.25rem;
  border-radius: var(--fluent-radius-sm);
  background: transparent;
  color: var(--fluent-color-text-primary);
  font: inherit;
  font-size: var(--fluent-font-size-body);
  cursor: pointer;
}

.crumb:hover,
.crumb:focus-visible {
  background: var(--fluent-color-fill-selected);
  outline: none;
}

.crumb-label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.crumb-chevron-wrap {
  position: relative;
  display: inline-flex;
  align-items: center;
}

.crumb-chevron {
  border: 0;
  padding: 0.1rem 0.2rem;
  border-radius: var(--fluent-radius-sm);
  background: transparent;
  color: var(--fluent-color-text-secondary);
  font: inherit;
  line-height: 1;
  cursor: pointer;
}

.crumb-chevron:hover,
.crumb-chevron:focus-visible,
.crumb-chevron--open {
  background: var(--fluent-color-fill-selected);
  color: var(--fluent-color-text-primary);
  outline: none;
}

/* Portaled to body (fixed via inline style) so path-bar overflow cannot clip it. */
.crumb-menu {
  min-width: 11rem;
  max-width: min(18rem, 70vw);
  max-height: min(18rem, 50vh);
  overflow: auto;
  padding: var(--fluent-space-xs);
  display: flex;
  flex-direction: column;
  gap: 0.1rem;
  box-shadow: var(--fluent-shadow-flyout, 0 8px 24px rgba(0, 0, 0, 0.18));
}

.crumb-menu-status {
  margin: 0;
  padding: var(--fluent-space-sm);
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
}

.crumb-menu-status--error {
  color: var(--fluent-color-status-danger);
}

.crumb-menu-item {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  width: 100%;
  border: 0;
  border-radius: var(--fluent-radius-sm);
  padding: 0.35rem 0.5rem;
  background: transparent;
  color: var(--fluent-color-text-primary);
  font: inherit;
  text-align: start;
  cursor: pointer;
}

.crumb-menu-item span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.crumb-menu-item:hover,
.crumb-menu-item:focus-visible {
  background: var(--fluent-color-fill-selected);
  outline: none;
}

.crumb-menu-item--active {
  background: var(--fluent-color-fill-subtle-secondary, var(--fluent-color-fill-selected));
  font-weight: var(--fluent-font-weight-semibold);
}

.crumb-placeholder {
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
  padding-inline: var(--fluent-space-xs);
}

.address-input {
  flex: 1;
  height: var(--fluent-control-height-standard);
  min-height: var(--fluent-control-height-standard);
  padding-inline: var(--fluent-space-md);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-subtle);
  color: var(--fluent-color-text-primary);
  font: inherit;
}

.address-input:hover {
  background: var(--fluent-color-fill-subtle-secondary);
}

.address-input:focus {
  outline: 2px solid var(--fluent-color-stroke-focus);
  outline-offset: 1px;
  background: var(--fluent-color-fill-control);
}

.address-go {
  height: var(--fluent-control-height-standard);
  min-height: var(--fluent-control-height-standard);
}

.tabstrip {
  display: flex;
  align-items: flex-end;
  gap: var(--fluent-space-xs);
  min-height: var(--fluent-control-height-standard);
  flex-shrink: 0;
  padding: var(--fluent-space-xs) var(--fluent-space-md) 0;
  border-bottom: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  background: var(--fluent-color-fill-subtle);
  -webkit-app-region: no-drag;
  app-region: no-drag;
}

.tab {
  display: inline-flex;
  align-items: center;
  gap: var(--fluent-space-xs);
  max-width: 14rem;
  min-height: var(--fluent-control-height-standard);
  padding: 0 var(--fluent-space-sm) 0 var(--fluent-space-md);
  border: var(--fluent-stroke-width) solid transparent;
  border-bottom: 0;
  border-radius: var(--fluent-radius-sm) var(--fluent-radius-sm) 0 0;
  background: transparent;
  color: var(--fluent-color-text-secondary);
  font: inherit;
  cursor: grab;
  touch-action: none;
  -webkit-app-region: no-drag;
  app-region: no-drag;
  user-select: none;
}

.tab .tab-icon,
.tab .tab-label {
  pointer-events: none;
}

.tab:hover:not(:disabled) {
  background: var(--fluent-color-fill-control-hover);
  color: var(--fluent-color-text-primary);
}

.tab--active {
  background: var(--fluent-color-fill-control);
  border-color: var(--fluent-color-stroke-divider);
  color: var(--fluent-color-text-primary);
  font-weight: var(--fluent-font-weight-semibold);
  box-shadow: inset 0 -2px 0 var(--fluent-color-fill-accent);
}

.tab--drag-over {
  outline: 2px solid var(--fluent-color-stroke-accent-soft, var(--fluent-color-fill-accent));
  outline-offset: -2px;
}

.tab--dragging {
  opacity: 0.55;
}

.tab:active {
  cursor: grabbing;
}

.tab-close {
  pointer-events: auto;
  cursor: pointer;
  touch-action: manipulation;
}

.tab-icon {
  flex-shrink: 0;
  opacity: 0.85;
}

.tab-label {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tab-close {
  flex-shrink: 0;
  width: 1.25rem;
  height: 1.25rem;
  border: 0;
  border-radius: var(--fluent-radius-sm);
  background: transparent;
  color: inherit;
  font: inherit;
  line-height: 1;
  cursor: pointer;
  opacity: 0.7;
}

.tab-close:hover {
  background: var(--fluent-color-fill-control-pressed);
  opacity: 1;
}

.tab--add {
  min-width: var(--fluent-control-height-standard);
  justify-content: center;
  padding-inline: 0;
}

.body {
  display: flex;
  flex: 1;
  min-height: 0;
}

.workspace {
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.sidenav {
  width: var(--fluent-sidenav-width);
  flex-shrink: 0;
  padding: var(--fluent-space-sm);
  border-right: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  overflow: auto;
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-xs);
}

.sidenav-item {
  position: relative;
  width: 100%;
  justify-content: flex-start;
  gap: var(--fluent-space-sm);
  border: 0;
  border-radius: var(--fluent-radius-sm);
  background: transparent;
  color: inherit;
  text-align: left;
  cursor: pointer;
}

.sidenav-item:hover:not(:disabled) {
  background: var(--fluent-color-fill-control-hover);
}

.sidenav-item--active {
  background: var(--fluent-color-fill-selected);
  font-weight: var(--fluent-font-weight-semibold);
  color: var(--fluent-color-text-primary);
}

.sidenav-item--active::before {
  content: "";
  position: absolute;
  left: 0;
  top: 6px;
  bottom: 6px;
  width: 3px;
  border-radius: 0 2px 2px 0;
  background: var(--fluent-color-stroke-accent-soft);
}

.sidenav-icon {
  color: var(--fluent-color-fill-accent);
}

.sidenav-item--active .sidenav-icon {
  color: var(--fluent-color-stroke-accent-soft);
}

.sidenav-label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sidenav-item--later {
  opacity: 0.55;
  cursor: not-allowed;
}

.later-badge {
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.content {
  flex: 1;
  min-width: 0;
  min-height: 0;
  padding: var(--fluent-space-lg);
  overflow: auto;
  background: var(--fluent-color-fill-subtle);
  color: var(--fluent-color-text-primary);
}

.content--navigator {
  padding-top: var(--fluent-space-md);
}

.statusbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--fluent-space-md);
  min-height: var(--fluent-statusbar-height);
  flex-shrink: 0;
  padding-inline: var(--fluent-space-md);
  border-top: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  background: var(--fluent-color-fill-layer-alt);
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.status-text {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.status-primary {
  flex: 1;
}

.status-trailing {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  flex-shrink: 0;
}

.status-health {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 28ch;
}

.status-refresh {
  border: 0;
  background: transparent;
  color: var(--fluent-color-fill-accent);
  font: inherit;
  cursor: pointer;
  min-height: var(--fluent-control-height-standard);
  padding-inline: var(--fluent-space-sm);
}

.close-confirm-backdrop {
  position: fixed;
  inset: 0;
  z-index: 80;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--fluent-space-lg);
  background: color-mix(in srgb, var(--fluent-color-text-primary) 28%, transparent);
}

.close-confirm-dialog {
  width: min(26rem, 100%);
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
  padding: var(--fluent-space-lg);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  box-shadow: var(--fluent-shadow-flyout, 0 12px 32px rgba(0, 0, 0, 0.22));
}

.close-confirm-title {
  margin: 0;
  font-size: var(--fluent-font-size-subtitle);
  font-weight: var(--fluent-font-weight-semibold);
}

.close-confirm-message {
  margin: 0;
  color: var(--fluent-color-text-secondary);
  line-height: 1.45;
}

.close-confirm-remember {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  font-size: var(--fluent-font-size-body);
}

.close-confirm-actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: var(--fluent-space-sm);
}

.navigator-filter-inline {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-xs);
  flex: 0 1 14rem;
  min-width: 8rem;
  max-width: 18rem;
  height: var(--fluent-control-height-standard);
  padding-inline: var(--fluent-space-sm);
  border: var(--fluent-stroke-width) solid var(--fluent-color-stroke-divider);
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-subtle);
}

.navigator-filter-inline:focus-within {
  outline: 2px solid var(--fluent-color-stroke-focus);
  outline-offset: 1px;
  background: var(--fluent-color-fill-control);
}

.navigator-filter-icon {
  color: var(--fluent-color-fill-accent);
  flex-shrink: 0;
}

.navigator-filter-input {
  flex: 1;
  min-width: 0;
  min-height: calc(var(--fluent-control-height-standard) - 4px);
  height: calc(var(--fluent-control-height-standard) - 4px);
  border: 0;
  padding-inline: 0;
  background: transparent;
  box-shadow: none;
}

.navigator-filter-input:focus {
  outline: none;
  box-shadow: none;
}

.navigator-filter-clear {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1.5rem;
  height: 1.5rem;
  min-width: 1.5rem;
  min-height: 1.5rem;
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--fluent-color-text-secondary);
  flex-shrink: 0;
}

.navigator-filter-clear:hover {
  color: var(--fluent-color-text-primary);
  background: var(--fluent-color-fill-selected);
}
</style>
