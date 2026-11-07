<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/**
 * AIA-216 / A3 — Public Skill library UI.
 * Import requires root SKILL.md; deploy to agents is A4.
 */
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { toastSuccess } from "../state/toast";

type LibrarySkill = {
  name: string;
  path: string;
  manifestPath: string;
  importedAt: number;
};

type SkillsLibraryStatus = {
  root: string;
  skills: LibrarySkill[];
};

const { t } = useI18n();

const status = ref<SkillsLibraryStatus | null>(null);
const error = ref<string | null>(null);
const loading = ref(false);
const sourcePath = ref("");
const importName = ref("");
const rootDraft = ref("");

const SKILL_ERROR_KEYS: Record<string, string> = {
  AI_SKILL_SOURCE_INVALID: "ai.skills.errorSourceInvalid",
  AI_SKILL_MANIFEST_MISSING: "ai.skills.errorManifestMissing",
  AI_SKILL_NAME_INVALID: "ai.skills.errorNameInvalid",
  AI_SKILL_EXISTS: "ai.skills.errorExists",
  AI_SKILL_UNKNOWN: "ai.skills.errorUnknown",
  AI_SKILL_ROOT_INVALID: "ai.skills.errorRootInvalid",
};

function mapError(err: unknown): string {
  const raw = err instanceof Error ? err.message : String(err);
  for (const [code, key] of Object.entries(SKILL_ERROR_KEYS)) {
    if (raw.includes(code)) return t(key);
  }
  return raw;
}

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    status.value = await invoke<SkillsLibraryStatus>("ai_skills_library_get");
    rootDraft.value = status.value.root;
  } catch (err) {
    error.value = mapError(err);
  } finally {
    loading.value = false;
  }
}

async function saveRoot() {
  error.value = null;
  try {
    const trimmed = rootDraft.value.trim();
    status.value = await invoke<SkillsLibraryStatus>("ai_skills_library_set_root", {
      root: trimmed || null,
    });
    rootDraft.value = status.value.root;
    toastSuccess(t("ai.skills.noticeRootSaved"));
  } catch (err) {
    error.value = mapError(err);
  }
}

async function resetRoot() {
  error.value = null;
  try {
    status.value = await invoke<SkillsLibraryStatus>("ai_skills_library_set_root", {
      root: null,
    });
    rootDraft.value = status.value.root;
    toastSuccess(t("ai.skills.noticeRootReset"));
  } catch (err) {
    error.value = mapError(err);
  }
}

async function importSkill() {
  if (!sourcePath.value.trim()) return;
  error.value = null;
  try {
    const [next] = await invoke<[SkillsLibraryStatus, LibrarySkill]>("ai_skills_import", {
      sourcePath: sourcePath.value.trim(),
      name: importName.value.trim() || null,
    });
    status.value = next;
    sourcePath.value = "";
    importName.value = "";
    toastSuccess(t("ai.skills.noticeImported"));
  } catch (err) {
    error.value = mapError(err);
  }
}

async function removeSkill(name: string) {
  error.value = null;
  try {
    status.value = await invoke<SkillsLibraryStatus>("ai_skills_remove", { name });
    toastSuccess(t("ai.skills.noticeRemoved"));
  } catch (err) {
    error.value = mapError(err);
  }
}

onMounted(() => {
  void refresh();
});
</script>

<template>
  <div class="skills-panel" data-ai-skills-panel>
    <p v-if="loading" class="fluent-banner">{{ t("common.loading") }}</p>
    <p v-else-if="error" class="fluent-banner fluent-banner--error">{{ error }}</p>

    <div class="fluent-surface skills-block">
      <h3 class="skills-subhead">{{ t("ai.skills.libraryRoot") }}</h3>
      <p class="skills-hint">{{ t("ai.skills.libraryRootHint") }}</p>
      <div class="skills-row">
        <input v-model="rootDraft" class="fluent-control skills-grow" type="text" />
        <button type="button" class="fluent-control fluent-control--accent" @click="saveRoot">
          {{ t("ai.skills.saveRoot") }}
        </button>
        <button type="button" class="fluent-control" @click="resetRoot">
          {{ t("ai.skills.resetRoot") }}
        </button>
      </div>
    </div>

    <div class="fluent-surface skills-block">
      <h3 class="skills-subhead">{{ t("ai.skills.importTitle") }}</h3>
      <p class="skills-hint">{{ t("ai.skills.importHint") }}</p>
      <label class="skills-field">
        <span>{{ t("ai.skills.fieldSource") }}</span>
        <input v-model="sourcePath" class="fluent-control" type="text" />
      </label>
      <label class="skills-field">
        <span>{{ t("ai.skills.fieldName") }}</span>
        <input
          v-model="importName"
          class="fluent-control"
          type="text"
          :placeholder="t('ai.skills.fieldNameHint')"
        />
      </label>
      <button type="button" class="fluent-control fluent-control--accent" @click="importSkill">
        {{ t("ai.skills.import") }}
      </button>
    </div>

    <div v-if="status" class="skills-list-wrap">
      <p class="skills-summary">
        {{ t("ai.skills.summary", { count: status.skills.length }) }}
      </p>
      <ul v-if="status.skills.length" class="skills-list">
        <li v-for="skill in status.skills" :key="skill.name" class="skills-list-item">
          <div class="skills-list-main">
            <strong>{{ skill.name }}</strong>
            <span class="skills-list-meta">{{ skill.path }}</span>
          </div>
          <button type="button" class="fluent-control" @click="removeSkill(skill.name)">
            {{ t("common.remove") }}
          </button>
        </li>
      </ul>
      <p v-else class="skills-empty">{{ t("ai.skills.empty") }}</p>
    </div>
  </div>
</template>

<style scoped>
.skills-panel {
  margin-top: var(--fluent-space-md);
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
}

.skills-block {
  padding: var(--fluent-space-md) var(--fluent-space-lg);
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
}

.skills-subhead {
  margin: 0;
  font-size: var(--fluent-font-size-body);
}

.skills-hint,
.skills-summary,
.skills-empty,
.skills-list-meta {
  margin: 0;
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
}

.skills-row {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fluent-space-sm);
}

.skills-grow {
  flex: 1;
  min-width: 12rem;
}

.skills-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.skills-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-xs);
}

.skills-list-item {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  padding: var(--fluent-space-sm) var(--fluent-space-md);
  border-radius: var(--fluent-radius-medium);
  background: var(--fluent-color-subtle-background);
}

.skills-list-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.skills-list-wrap {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
}
</style>
