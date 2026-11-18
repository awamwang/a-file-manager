<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { computed, defineAsyncComponent, ref } from "vue";
import { useI18n } from "vue-i18n";
import {
  loadFuzzy,
  loadScopes,
  persistFuzzy,
  persistScopes,
  SEARCH_PALETTE_SCOPES_STORAGE_KEY,
  toggleScope as applyScopeToggle,
} from "../../search/scope.ts";
import {
  SEARCH_SOURCE_ORDER,
  type SearchSourceId,
} from "../../search/types.ts";

/** AIA-276：进入搜索设置分区时再加载 Discovery 探测页。 */
const DiscoveryView = defineAsyncComponent(
  () => import("../../views/DiscoveryView.vue"),
);

defineProps<{
  /** 是否展示「优先 Everything」行（受平台能力门控）。 */
  everythingPreferredVisible: boolean;
  everythingPreferred: boolean;
  everythingPreferredSaving: boolean;
}>();

const emit = defineEmits<{
  "update:everythingPreferred": [value: boolean];
}>();

const { t } = useI18n();

const storage =
  typeof window === "undefined" ? undefined : window.localStorage;

const fuzzy = ref(loadFuzzy(storage));
/** null = 未持久化 / 按「全部」处理。 */
const storedScopes = ref<SearchSourceId[] | null>(loadScopes(storage));

const activeScopes = computed<SearchSourceId[]>(() => {
  const available = [...SEARCH_SOURCE_ORDER];
  const stored = storedScopes.value;
  if (!stored) return available;
  const effective = stored.filter((id) => available.includes(id));
  return effective.length ? effective : available;
});

function onFuzzyChange(event: Event) {
  const checked = (event.target as HTMLInputElement).checked;
  fuzzy.value = checked;
  persistFuzzy(checked, storage);
}

function onScopeToggle(id: SearchSourceId) {
  const next = applyScopeToggle(
    activeScopes.value,
    [...SEARCH_SOURCE_ORDER],
    id,
  );
  // 全选时回落 null，与面板「全部」语义一致。
  const allSelected =
    next.length === SEARCH_SOURCE_ORDER.length &&
    SEARCH_SOURCE_ORDER.every((sourceId) => next.includes(sourceId));
  storedScopes.value = allSelected ? null : next;
  if (allSelected) {
    try {
      storage?.removeItem(SEARCH_PALETTE_SCOPES_STORAGE_KEY);
    } catch {
      /* 持久化失败不阻断设置页。 */
    }
  } else {
    persistScopes(next, storage);
  }
}

function onEverythingPreferredChange(event: Event) {
  emit(
    "update:everythingPreferred",
    (event.target as HTMLInputElement).checked,
  );
}
</script>

<template>
  <div class="search-settings">
    <section class="fluent-surface search-settings-block">
      <h2 class="search-settings-title">{{ t("settings.search.paletteTitle") }}</h2>
      <p class="search-settings-meta">{{ t("settings.search.paletteMeta") }}</p>

      <label class="search-settings-toggle">
        <input
          type="checkbox"
          class="fluent-field-check"
          :checked="fuzzy"
          @change="onFuzzyChange"
        />
        <span>
          <strong>{{ t("searchPalette.fuzzy") }}</strong>
          <span class="search-settings-desc">{{
            t("settings.search.fuzzyHint")
          }}</span>
        </span>
      </label>

      <div
        class="search-settings-scopes"
        role="group"
        :aria-label="t('searchPalette.scopesLabel')"
      >
        <span class="search-settings-scopes-label">{{
          t("searchPalette.scopesLabel")
        }}</span>
        <p class="search-settings-desc">{{ t("settings.search.scopesHint") }}</p>
        <div class="search-settings-scope-chips">
          <label
            v-for="id in SEARCH_SOURCE_ORDER"
            :key="id"
            class="search-settings-scope-chip"
          >
            <input
              type="checkbox"
              class="fluent-field-check"
              :checked="activeScopes.includes(id)"
              @change="onScopeToggle(id)"
            />
            <span>{{ t(`searchPalette.scopes.${id}`) }}</span>
          </label>
        </div>
      </div>

      <label
        v-if="everythingPreferredVisible"
        class="search-settings-toggle"
      >
        <input
          type="checkbox"
          class="fluent-field-check"
          :checked="everythingPreferred"
          :disabled="everythingPreferredSaving"
          @change="onEverythingPreferredChange"
        />
        <span>
          <strong>{{
            t("settings.fields.core.everythingPreferred.title")
          }}</strong>
          <span class="search-settings-desc">{{
            t("settings.fields.core.everythingPreferred.description")
          }}</span>
        </span>
      </label>
    </section>

    <section class="search-settings-probe">
      <h2 class="search-settings-title">{{ t("settings.search.probeTitle") }}</h2>
      <p class="search-settings-meta">{{ t("settings.search.probeMeta") }}</p>
      <DiscoveryView mode="test" />
    </section>
  </div>
</template>

<style scoped>
.search-settings {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-lg);
  min-width: 0;
}

.search-settings-block {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
  padding: var(--fluent-space-md) var(--fluent-space-lg);
}

.search-settings-title {
  margin: 0;
  font-size: var(--fluent-font-size-body);
  font-weight: var(--fluent-font-weight-semibold);
}

.search-settings-meta,
.search-settings-desc {
  margin: 0;
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
  line-height: 1.45;
}

.search-settings-toggle {
  display: flex;
  align-items: flex-start;
  gap: var(--fluent-space-sm);
  cursor: pointer;
}

.search-settings-toggle span {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
}

.search-settings-scopes {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-xs);
}

.search-settings-scopes-label {
  font-weight: var(--fluent-font-weight-semibold);
}

.search-settings-scope-chips {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fluent-space-sm);
}

.search-settings-scope-chip {
  display: inline-flex;
  align-items: center;
  gap: var(--fluent-space-xs);
  cursor: pointer;
}

.search-settings-probe {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
  min-width: 0;
}

.search-settings-probe :deep(.fluent-page) {
  padding-block: 0;
}

.search-settings-probe :deep(.fluent-page-header) {
  display: none;
}
</style>
