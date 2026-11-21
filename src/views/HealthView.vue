<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";

type HealthPayload = {
  ok: boolean;
  name: string;
  version: string;
  identifier: string;
  aiAddon?: boolean;
};

type DependencyItem = {
  id: string;
  kind: string;
  status: string;
  title: string;
  detail: string;
  fix: string;
  /** 托管投影（Q9a）：config|managed；未 pack 化依赖缺省（undefined）。 */
  mode?: string;
  /** 注册表有记录 → true，渲染启停按钮。 */
  actionable?: boolean;
  /** 双失败态归因（Q18）：configOnly | managedOnly | both。 */
  unavailableReason?: string;
};

type DependencyReport = {
  generatedAtMs: number;
  items: DependencyItem[];
  requiredOk: boolean;
  missingRequired: number;
  missingOptional: number;
};

const { t, te } = useI18n();
const health = ref<HealthPayload | null>(null);
const deps = ref<DependencyReport | null>(null);
const error = ref<string | null>(null);
/** 身份卡 / 依赖卡各自加载，页面壳始终可交互。 */
const healthLoading = ref(true);
const depsLoading = ref(true);
/** 离开页面或新一轮刷新时递增，作废进行中的结果写回。 */
const loadEpoch = ref(0);

const loading = computed(() => healthLoading.value || depsLoading.value);

// 托管依赖动作（B 决策：不暴露 mode 切换；仅来源标签 + 启停按钮 + 双失败态隐身）。
const starting = ref<Record<string, boolean>>({});
const stopping = ref<Record<string, boolean>>({});
const actionErrors = ref<Record<string, string>>({});

const SKELETON_CARD_COUNT = 6;

function showSourceLabel(item: DependencyItem): boolean {
  return item.mode === "config" || item.mode === "managed";
}

function sourceLabel(item: DependencyItem): string {
  return item.mode ? t(`managedDep.status.${item.mode}`) : "";
}

/** 双失败态（both）→ 隐身启动入口，不误导成「AFM 能帮你启动」（Q18）。 */
function canStart(item: DependencyItem): boolean {
  return item.actionable === true && item.unavailableReason !== "both";
}

function canStop(item: DependencyItem): boolean {
  // 停止权在后端兜底（只 kill 自启 pid）；非自启会被拒绝并把错误显示在此条目上。
  return item.actionable === true && item.unavailableReason !== "both";
}

function failureText(item: DependencyItem): string {
  if (!item.unavailableReason) return "";
  const key = `managedDep.failure.${item.unavailableReason}`;
  return te(key) ? String(t(key)) : "";
}

async function startManaged(item: DependencyItem): Promise<void> {
  starting.value = { ...starting.value, [item.id]: true };
  actionErrors.value = { ...actionErrors.value, [item.id]: "" };
  try {
    await invoke("managed_dependency_ensure", { id: item.id });
    await loadHealth();
  } catch (err) {
    actionErrors.value = {
      ...actionErrors.value,
      [item.id]: t("managedDep.action.failed", {
        error: err instanceof Error ? err.message : String(err),
      }),
    };
  } finally {
    starting.value = { ...starting.value, [item.id]: false };
  }
}

async function stopManaged(item: DependencyItem): Promise<void> {
  stopping.value = { ...stopping.value, [item.id]: true };
  actionErrors.value = { ...actionErrors.value, [item.id]: "" };
  try {
    await invoke("managed_dependency_stop", { id: item.id });
    await loadHealth();
  } catch (err) {
    actionErrors.value = {
      ...actionErrors.value,
      [item.id]: t("managedDep.action.failed", {
        error: err instanceof Error ? err.message : String(err),
      }),
    };
  } finally {
    stopping.value = { ...stopping.value, [item.id]: false };
  }
}

function depTitle(item: DependencyItem): string {
  const key = `health.deps.items.${item.id}.title`;
  return te(key) ? String(t(key)) : item.title;
}

/**
 * Backend fallback copy can be Windows-flavoured; macOS overrides win.
 * Status-specific text (`...items.<id>.<status>.detail`) wins when present,
 * then the generic `...items.<id>.detail`, then the backend fallback — so a
 * running-but-busy QuickLook can read differently from a truly-missing one.
 */
function depText(item: DependencyItem, part: "detail" | "fix"): string {
  const statusKey = `health.deps.items.${item.id}.${item.status}.${part}`;
  if (te(statusKey)) return String(t(statusKey));
  const key = `health.deps.items.${item.id}.${part}`;
  return te(key) ? String(t(key)) : item[part];
}

function statusLabel(status: string): string {
  const key = `health.deps.status.${status}`;
  return te(key) ? String(t(key)) : status;
}

function kindLabel(kind: string): string {
  const key = `health.deps.kind.${kind}`;
  return te(key) ? String(t(key)) : kind;
}

/**
 * QuickLook "ok" comes from an existence probe (named pipe present) — AFM never
 * writes a Toggle, so green means "detected / running", not "a real preview
 * connection was verified". Tag it with a neutral note instead of faking certainty.
 */
function showUnverifiedNote(item: DependencyItem): boolean {
  return item.id === "quicklook" && item.status === "ok";
}

const summaryText = computed(() => {
  if (!deps.value) return "";
  if (!deps.value.requiredOk) {
    return t("health.deps.summaryRequiredMissing", {
      count: deps.value.missingRequired,
    });
  }
  if (deps.value.missingOptional > 0) {
    return t("health.deps.summaryOptionalMissing", {
      count: deps.value.missingOptional,
    });
  }
  return t("health.deps.summaryAllOk");
});

async function loadHealth() {
  const epoch = ++loadEpoch.value;
  healthLoading.value = true;
  depsLoading.value = true;
  error.value = null;

  const healthTask = invoke<HealthPayload>("health_get")
    .then((h) => {
      if (epoch !== loadEpoch.value) return;
      health.value = h;
    })
    .catch((err) => {
      if (epoch !== loadEpoch.value) return;
      health.value = null;
      error.value = err instanceof Error ? err.message : String(err);
    })
    .finally(() => {
      if (epoch === loadEpoch.value) healthLoading.value = false;
    });

  const depsTask = invoke<DependencyReport>("deps_probe_get")
    .then((d) => {
      if (epoch !== loadEpoch.value) return;
      deps.value = d;
    })
    .catch((err) => {
      if (epoch !== loadEpoch.value) return;
      deps.value = null;
      error.value = err instanceof Error ? err.message : String(err);
    })
    .finally(() => {
      if (epoch === loadEpoch.value) depsLoading.value = false;
    });

  await Promise.all([healthTask, depsTask]);
}

onMounted(() => {
  void loadHealth();
});

onUnmounted(() => {
  // 作废进行中的检查，避免写回已卸载视图。
  loadEpoch.value += 1;
});
</script>

<template>
  <section class="fluent-page panel" :aria-busy="loading ? 'true' : undefined">
    <header class="fluent-page-header fluent-page-header--row">
      <div class="fluent-page-header__text">
        <h1 class="fluent-page-title">{{ t("health.title") }}</h1>
        <p class="fluent-page-meta">{{ t("health.meta") }}</p>
      </div>
      <button
        class="fluent-control fluent-control--accent"
        type="button"
        :disabled="loading"
        @click="loadHealth"
      >
        {{ t("health.refresh") }}
      </button>
    </header>

    <p v-if="loading" class="fluent-hint" data-health-loading>
      {{ t("health.loading") }}
    </p>
    <p v-if="error" class="fluent-banner fluent-banner--error">
      {{ t("health.failed", { error }) }}
    </p>

    <div
      v-if="healthLoading && !health"
      class="health-skeleton fluent-surface"
      data-health-skeleton
      aria-hidden="true"
    >
      <div class="health-skeleton__banner" />
      <div class="health-skeleton__row" />
      <div class="health-skeleton__row" />
      <div class="health-skeleton__row" />
    </div>
    <template v-else-if="health">
      <p
        class="fluent-banner"
        :class="health.ok ? 'fluent-banner--success' : 'fluent-banner--error'"
      >
        <span
          class="fluent-chip fluent-chip--status"
          :class="health.ok ? 'fluent-chip--ok' : 'fluent-chip--down'"
        >{{ health.ok ? "ok" : "down" }}</span>
        {{ health.name }} · v{{ health.version }}
      </p>
      <dl class="fluent-surface grid">
        <dt>{{ t("health.fieldOk") }}</dt>
        <dd :class="health.ok ? 'fluent-status-text--ok' : 'fluent-status-text--down'">
          {{ health.ok }}
        </dd>
        <dt>{{ t("health.fieldName") }}</dt>
        <dd>{{ health.name }}</dd>
        <dt>{{ t("health.fieldVersion") }}</dt>
        <dd>{{ health.version }}</dd>
        <dt>{{ t("health.fieldIdentifier") }}</dt>
        <dd>{{ health.identifier }}</dd>
        <dt>{{ t("health.fieldAiAddon") }}</dt>
        <dd>{{ health.aiAddon ? t("health.aiAddonOn") : t("health.aiAddonOff") }}</dd>
      </dl>
    </template>

    <h2 class="deps-heading">{{ t("health.deps.title") }}</h2>
    <p class="fluent-page-meta">{{ t("health.deps.meta") }}</p>
    <p
      v-if="deps"
      class="fluent-banner"
      :class="
        !deps.requiredOk
          ? 'fluent-banner--error'
          : deps.missingOptional > 0
            ? 'fluent-banner--warning'
            : 'fluent-banner--success'
      "
    >
      {{ summaryText }}
    </p>

    <ul
      v-if="depsLoading && !deps"
      class="deps-list deps-list--skeleton"
      data-health-deps-skeleton
      aria-hidden="true"
    >
      <li
        v-for="n in SKELETON_CARD_COUNT"
        :key="n"
        class="deps-card deps-card--skeleton fluent-surface"
      >
        <div class="deps-skeleton__chip" />
        <div class="deps-skeleton__title" />
        <div class="deps-skeleton__line" />
        <div class="deps-skeleton__line deps-skeleton__line--short" />
      </li>
    </ul>
    <ul v-else-if="deps" class="deps-list" data-health-deps>
      <li
        v-for="item in deps.items"
        :key="item.id"
        class="deps-card fluent-surface"
        :class="`deps-card--${item.status}`"
        :data-dep-id="item.id"
        :data-dep-status="item.status"
      >
        <div class="deps-card__head">
          <span
            class="fluent-chip fluent-chip--status"
            :class="{
              'fluent-chip--ok': item.status === 'ok',
              'fluent-chip--down': item.status === 'missing',
              'fluent-chip--warn': item.status === 'warn' || item.status === 'unbridged',
            }"
          >{{ statusLabel(item.status) }}</span>
          <span
            v-if="showUnverifiedNote(item)"
            class="fluent-chip fluent-chip--note"
          >{{ t("health.deps.quicklookUnverified") }}</span>
          <span
            v-if="showSourceLabel(item)"
            class="fluent-chip fluent-chip--note"
          >{{ sourceLabel(item) }}</span>
          <span class="deps-kind">{{ kindLabel(item.kind) }}</span>
          <strong class="deps-title">{{ depTitle(item) }}</strong>
        </div>
        <p class="deps-detail">{{ depText(item, 'detail') }}</p>
        <p class="deps-fix">
          <span class="deps-fix-label">{{ t("health.deps.fixLabel") }}</span>
          {{ depText(item, 'fix') }}
        </p>
        <p v-if="failureText(item)" class="deps-managed-failure">
          {{ failureText(item) }}
        </p>
        <p v-if="actionErrors[item.id]" class="deps-managed-error">
          {{ actionErrors[item.id] }}
        </p>
        <div
          v-if="canStart(item) || canStop(item)"
          class="deps-managed-actions"
        >
          <button
            v-if="canStart(item)"
            class="fluent-control"
            type="button"
            :disabled="starting[item.id] === true"
            @click="startManaged(item)"
          >{{ starting[item.id] ? t("managedDep.action.starting") : t("managedDep.action.start") }}</button>
          <button
            v-if="canStop(item)"
            class="fluent-control"
            type="button"
            :disabled="stopping[item.id] === true"
            @click="stopManaged(item)"
          >{{ stopping[item.id] ? t("managedDep.action.stopping") : t("managedDep.action.stop") }}</button>
        </div>
      </li>
    </ul>
  </section>
</template>

<style scoped>
.panel {
  max-width: 44rem;
}

.grid {
  display: grid;
  grid-template-columns: 8rem 1fr;
  gap: var(--fluent-space-sm) var(--fluent-space-md);
  margin: 0;
  padding: var(--fluent-space-md) var(--fluent-space-lg);
}

dt {
  color: var(--fluent-color-text-secondary);
}

dd {
  margin: 0;
  font-family: var(--fluent-font-family-mono);
}

.fluent-banner {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--fluent-space-sm);
}

.fluent-chip {
  pointer-events: none;
  cursor: default;
  font-size: var(--fluent-font-size-caption);
}

.deps-heading {
  margin: var(--fluent-space-lg) 0 var(--fluent-space-xs);
  font-size: var(--fluent-font-size-subtitle);
}

.deps-list {
  list-style: none;
  margin: 0 0 var(--fluent-space-md);
  padding: 0;
  display: grid;
  /* 与 path-card-grid 同口径：宽屏 auto-fill，窄窗仍单列。 */
  grid-template-columns: repeat(auto-fill, minmax(min(100%, 22rem), 1fr));
  gap: var(--fluent-space-sm);
}

.deps-card {
  padding: var(--fluent-space-md) var(--fluent-space-lg);
  border-left: 3px solid var(--fluent-color-stroke-divider);
}

.deps-card--ok {
  border-left-color: var(--fluent-color-status-success);
}

.deps-card--missing {
  border-left-color: var(--fluent-color-status-danger);
}

.deps-card--warn {
  border-left-color: var(--fluent-color-status-warning);
}

.deps-card--unbridged {
  border-left-color: var(--fluent-color-status-warning);
}

.deps-card__head {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--fluent-space-sm);
  margin-bottom: var(--fluent-space-xs);
}

.deps-kind {
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.deps-title {
  font-weight: 600;
}

.deps-detail,
.deps-fix {
  margin: var(--fluent-space-2xs, 0.25rem) 0 0;
  font-size: var(--fluent-font-size-body);
  line-height: 1.45;
}

.deps-fix {
  color: var(--fluent-color-text-secondary);
}

.deps-fix-label {
  font-weight: 600;
  color: var(--fluent-color-text-primary);
  margin-right: 0.35rem;
}

.deps-managed-failure {
  margin: var(--fluent-space-2xs, 0.25rem) 0 0;
  font-size: var(--fluent-font-size-body);
  color: var(--fluent-color-status-warning, var(--fluent-color-text-secondary));
}

.deps-managed-error {
  margin: var(--fluent-space-2xs, 0.25rem) 0 0;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-status-danger, var(--fluent-color-text-secondary));
}

.deps-managed-actions {
  display: flex;
  gap: var(--fluent-space-sm);
  margin-top: var(--fluent-space-sm);
}

button {
  align-self: flex-start;
}

.fluent-page-header--row {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--fluent-space-md);
}

.fluent-page-header__text {
  min-width: 0;
}

/* Neutral, non-alarming marker: green status stays, this says "existence-only". */
.fluent-chip--note {
  color: var(--fluent-color-text-secondary);
  background: var(--fluent-color-fill-quaternary, rgba(128, 128, 128, 0.14));
  border: 1px solid var(--fluent-color-stroke-divider);
}

.health-skeleton {
  display: grid;
  gap: var(--fluent-space-sm);
  padding: var(--fluent-space-md) var(--fluent-space-lg);
  margin-bottom: var(--fluent-space-sm);
}

.health-skeleton__banner,
.health-skeleton__row,
.deps-skeleton__chip,
.deps-skeleton__title,
.deps-skeleton__line {
  border-radius: 4px;
  background: var(--fluent-color-fill-tertiary, rgba(128, 128, 128, 0.16));
}

.health-skeleton__banner {
  height: 1.75rem;
  width: min(100%, 16rem);
}

.health-skeleton__row {
  height: 1rem;
  width: 70%;
}

.deps-card--skeleton {
  border-left-color: var(--fluent-color-stroke-divider);
  display: grid;
  gap: var(--fluent-space-xs);
}

.deps-skeleton__chip {
  height: 1.25rem;
  width: 3.5rem;
}

.deps-skeleton__title {
  height: 1.1rem;
  width: 55%;
}

.deps-skeleton__line {
  height: 0.85rem;
  width: 90%;
}

.deps-skeleton__line--short {
  width: 60%;
}
</style>
