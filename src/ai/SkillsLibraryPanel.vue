<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/**
 * 公共 Skill 库：横向主从布局；删除需确认；添加到指定单个项目。
 */
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { openConfirmDialog } from "../state/confirmDialog";
import { toastSuccess } from "../state/toast";
import {
  aiAgentTargetsList,
  aiHarvestSkillsImport,
  aiHarvestSkillsScan,
  aiSkillDeployModeSet,
  aiSkillsDeploy,
  aiSkillsImport,
  aiSkillsLibraryGet,
  aiSkillsLibrarySetRoot,
  aiSkillsRemove,
  aiSkillsUndeploy,
} from "./client";
import { listProjectOptions, type ProjectOption } from "./projectOptions";
import type {
  AgentTarget,
  HarvestSkillCandidate,
  LibrarySkill,
  SkillDeployReport,
  SkillsLibraryStatus,
} from "./types";

const { t } = useI18n();

const status = ref<SkillsLibraryStatus | null>(null);
const error = ref<string | null>(null);
const loading = ref(false);
const selectedName = ref<string | null>(null);
const sourcePath = ref("");
const importName = ref("");
const rootDraft = ref("");
const filterText = ref("");
const lastDeployNote = ref<string | null>(null);
const skillDeployMode = ref("junction");
const agents = ref<AgentTarget[]>([]);
const harvestAgentId = ref("");
const harvestCandidates = ref<HarvestSkillCandidate[]>([]);
const harvestSelected = ref<Record<string, boolean>>({});
const showTools = ref(false);
const projects = ref<ProjectOption[]>([]);
const targetProjectPath = ref("");

const SKILL_ERROR_KEYS: Record<string, string> = {
  AI_SKILL_SOURCE_INVALID: "ai.skills.errorSourceInvalid",
  AI_SKILL_MANIFEST_MISSING: "ai.skills.errorManifestMissing",
  AI_SKILL_NAME_INVALID: "ai.skills.errorNameInvalid",
  AI_SKILL_EXISTS: "ai.skills.errorExists",
  AI_SKILL_UNKNOWN: "ai.skills.errorUnknown",
  AI_SKILL_ROOT_INVALID: "ai.skills.errorRootInvalid",
  AI_DEPLOY_NO_ENABLED: "ai.skills.errorDeployNoEnabled",
  AI_DEPLOY_TARGET_CONFLICT: "ai.skills.errorDeployConflict",
  AI_DEPLOY_PROJECT_REQUIRED: "ai.skills.errorDeployProject",
  AI_HARVEST_AGENT_UNKNOWN: "ai.harvest.errorAgentUnknown",
  AI_HARVEST_PATH_INVALID: "ai.harvest.errorPath",
};

function mapError(err: unknown): string {
  const raw = err instanceof Error ? err.message : String(err);
  for (const [code, key] of Object.entries(SKILL_ERROR_KEYS)) {
    if (raw.includes(code)) return t(key);
  }
  return raw;
}

const harvestPickCount = computed(
  () => Object.values(harvestSelected.value).filter(Boolean).length,
);

const filteredSkills = computed(() => {
  const q = filterText.value.trim().toLowerCase();
  const list = status.value?.skills ?? [];
  if (!q) return list;
  return list.filter(
    (s) => s.name.toLowerCase().includes(q) || s.path.toLowerCase().includes(q),
  );
});

const selected = computed(
  () => status.value?.skills.find((s) => s.name === selectedName.value) ?? null,
);

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    const [next, agentDoc, projectList] = await Promise.all([
      aiSkillsLibraryGet(),
      aiAgentTargetsList(),
      listProjectOptions(),
    ]);
    status.value = next;
    rootDraft.value = next.root;
    skillDeployMode.value = agentDoc.skillDeployMode || "junction";
    agents.value = agentDoc.agentTargets;
    projects.value = projectList.options;
    if (!targetProjectPath.value) {
      const active = projectList.options.find((p) => p.slug === projectList.activeSlug);
      targetProjectPath.value = active?.path ?? projectList.options[0]?.path ?? "";
    }
    if (!harvestAgentId.value && agents.value.length) {
      harvestAgentId.value = agents.value[0]?.id ?? "";
    }
    if (
      selectedName.value &&
      !next.skills.some((skill) => skill.name === selectedName.value)
    ) {
      selectedName.value = null;
    }
    if (!selectedName.value && next.skills.length) {
      selectedName.value = next.skills[0]?.name ?? null;
    }
  } catch (err) {
    error.value = mapError(err);
  } finally {
    loading.value = false;
  }
}

function selectSkill(skill: LibrarySkill) {
  selectedName.value = skill.name;
}

async function saveRoot() {
  error.value = null;
  try {
    const trimmed = rootDraft.value.trim();
    status.value = await aiSkillsLibrarySetRoot(trimmed || null);
    rootDraft.value = status.value.root;
    toastSuccess(t("ai.skills.noticeRootSaved"));
  } catch (err) {
    error.value = mapError(err);
  }
}

async function resetRoot() {
  error.value = null;
  try {
    status.value = await aiSkillsLibrarySetRoot(null);
    rootDraft.value = status.value.root;
    toastSuccess(t("ai.skills.noticeRootReset"));
  } catch (err) {
    error.value = mapError(err);
  }
}

async function saveDeployMode() {
  error.value = null;
  try {
    const doc = await aiSkillDeployModeSet(skillDeployMode.value);
    skillDeployMode.value = doc.skillDeployMode || "junction";
    toastSuccess(t("ai.skills.noticeDeployModeSaved"));
  } catch (err) {
    error.value = mapError(err);
  }
}

async function importSkill() {
  if (!sourcePath.value.trim()) return;
  error.value = null;
  try {
    const [next] = await aiSkillsImport(
      sourcePath.value.trim(),
      importName.value.trim() || null,
    );
    status.value = next;
    sourcePath.value = "";
    importName.value = "";
    toastSuccess(t("ai.skills.noticeImported"));
  } catch (err) {
    error.value = mapError(err);
  }
}

async function removeSkill(name: string) {
  const result = await openConfirmDialog({
    title: t("ai.skills.confirmRemoveTitle"),
    message: t("ai.skills.confirmRemoveMessage", { name }),
    confirmLabel: t("common.remove"),
    cancelLabel: t("common.cancel"),
    danger: true,
  });
  if (!result.confirmed) return;
  error.value = null;
  try {
    status.value = await aiSkillsRemove(name);
    if (selectedName.value === name) {
      selectedName.value = status.value.skills[0]?.name ?? null;
    }
    toastSuccess(t("ai.skills.noticeRemoved"));
  } catch (err) {
    error.value = mapError(err);
  }
}

async function deployGlobal(name: string) {
  error.value = null;
  lastDeployNote.value = null;
  try {
    const report: SkillDeployReport = await aiSkillsDeploy(name, null);
    lastDeployNote.value = t("ai.skills.noticeDeployed", {
      count: report.deployments.length,
      skipped: report.skipped.length,
    });
    toastSuccess(lastDeployNote.value);
  } catch (err) {
    error.value = mapError(err);
  }
}

async function deployToSelectedProject(name: string) {
  const ws = targetProjectPath.value.trim();
  if (!ws) {
    error.value = t("ai.skills.errorPickProject");
    return;
  }
  error.value = null;
  lastDeployNote.value = null;
  try {
    const report: SkillDeployReport = await aiSkillsDeploy(name, ws);
    const project =
      projects.value.find((p) => p.path === ws)?.displayName ?? ws;
    lastDeployNote.value = t("ai.skills.noticeDeployedToProject", {
      project,
      count: report.deployments.length,
      skipped: report.skipped.length,
    });
    toastSuccess(lastDeployNote.value);
  } catch (err) {
    error.value = mapError(err);
  }
}

async function undeployFromSelectedProject(name: string) {
  const ws = targetProjectPath.value.trim();
  if (!ws) {
    error.value = t("ai.skills.errorPickProject");
    return;
  }
  error.value = null;
  lastDeployNote.value = null;
  try {
    const report: SkillDeployReport = await aiSkillsUndeploy(name, ws);
    lastDeployNote.value = t("ai.skills.noticeUndeployed", {
      count: report.deployments.length,
    });
    toastSuccess(lastDeployNote.value);
  } catch (err) {
    error.value = mapError(err);
  }
}

async function scanHarvest() {
  if (!harvestAgentId.value) return;
  error.value = null;
  try {
    const report = await aiHarvestSkillsScan(harvestAgentId.value);
    harvestCandidates.value = report.candidates;
    const selectedMap: Record<string, boolean> = {};
    for (const c of report.candidates) {
      selectedMap[c.name] = !c.alreadyInLibrary;
    }
    harvestSelected.value = selectedMap;
  } catch (err) {
    error.value = mapError(err);
  }
}

async function importHarvest() {
  const picks = harvestCandidates.value.filter((c) => harvestSelected.value[c.name]);
  if (!picks.length) return;
  error.value = null;
  try {
    const [next, added, skipped] = await aiHarvestSkillsImport(picks);
    status.value = next;
    lastDeployNote.value = t("ai.harvest.noticeSkillsImported", {
      added: added.length,
      skipped: skipped.length,
    });
    toastSuccess(lastDeployNote.value);
    await scanHarvest();
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

    <div class="skills-toolbar">
      <input
        v-model="filterText"
        class="fluent-control skills-filter"
        type="search"
        :placeholder="t('ai.skills.filterPlaceholder')"
      />
      <p class="skills-summary">
        {{ t("ai.skills.summary", { count: status?.skills.length ?? 0 }) }}
      </p>
      <button type="button" class="fluent-control" @click="showTools = !showTools">
        {{ showTools ? t("ai.skills.hideTools") : t("ai.skills.showTools") }}
      </button>
    </div>

    <div v-if="showTools" class="fluent-surface skills-tools">
      <div class="skills-tools-grid">
        <div class="skills-tool-col">
          <h3 class="skills-subhead">{{ t("ai.skills.libraryRoot") }}</h3>
          <div class="skills-row">
            <input v-model="rootDraft" class="fluent-control skills-grow" type="text" />
            <button type="button" class="fluent-control fluent-control--accent" @click="saveRoot">
              {{ t("ai.skills.saveRoot") }}
            </button>
            <button type="button" class="fluent-control" @click="resetRoot">
              {{ t("ai.skills.resetRoot") }}
            </button>
          </div>
          <label class="skills-field">
            <span>{{ t("ai.skills.deployMode") }}</span>
            <select v-model="skillDeployMode" class="fluent-control" @change="saveDeployMode">
              <option value="junction">{{ t("ai.skills.deployModeJunction") }}</option>
              <option value="copy">{{ t("ai.skills.deployModeCopy") }}</option>
            </select>
          </label>
        </div>
        <div class="skills-tool-col">
          <h3 class="skills-subhead">{{ t("ai.harvest.skillsTitle") }}</h3>
          <div class="skills-row">
            <select v-model="harvestAgentId" class="fluent-control skills-grow">
              <option v-for="agent in agents" :key="agent.id" :value="agent.id">
                {{ agent.displayName }}
              </option>
            </select>
            <button type="button" class="fluent-control" @click="scanHarvest">
              {{ t("ai.harvest.scan") }}
            </button>
            <button
              type="button"
              class="fluent-control fluent-control--accent"
              :disabled="!harvestPickCount"
              @click="importHarvest"
            >
              {{ t("ai.harvest.importSelected", { count: harvestPickCount }) }}
            </button>
          </div>
          <div v-if="harvestCandidates.length" class="skills-harvest-scroll">
            <label
              v-for="c in harvestCandidates"
              :key="c.name"
              class="skills-check"
            >
              <input
                v-model="harvestSelected[c.name]"
                type="checkbox"
                :disabled="c.alreadyInLibrary"
              />
              <span>{{ c.name }}</span>
            </label>
          </div>
        </div>
        <div class="skills-tool-col">
          <h3 class="skills-subhead">{{ t("ai.skills.importTitle") }}</h3>
          <input
            v-model="sourcePath"
            class="fluent-control"
            type="text"
            :placeholder="t('ai.skills.fieldSource')"
          />
          <input
            v-model="importName"
            class="fluent-control"
            type="text"
            :placeholder="t('ai.skills.fieldNameHint')"
          />
          <button type="button" class="fluent-control fluent-control--accent" @click="importSkill">
            {{ t("ai.skills.import") }}
          </button>
        </div>
      </div>
    </div>

    <div class="skills-split">
      <aside class="fluent-surface skills-list-pane">
        <ul
          v-if="filteredSkills.length"
          class="skills-list"
          role="listbox"
          :aria-label="t('ai.sections.skills.title')"
        >
          <li
            v-for="skill in filteredSkills"
            :key="skill.name"
            class="skills-list-item"
            :class="{ 'skills-list-item--active': skill.name === selectedName }"
            role="option"
            :aria-selected="skill.name === selectedName"
            @click="selectSkill(skill)"
          >
            <strong>{{ skill.name }}</strong>
          </li>
        </ul>
        <p v-else class="skills-empty">{{ t("ai.skills.empty") }}</p>
      </aside>

      <article v-if="selected" class="fluent-surface skills-detail">
        <header class="skills-detail-head">
          <div>
            <h3 class="skills-subhead">{{ selected.name }}</h3>
            <p class="skills-hint">{{ selected.path }}</p>
          </div>
          <button type="button" class="fluent-control" @click="removeSkill(selected.name)">
            {{ t("common.remove") }}
          </button>
        </header>

        <div class="skills-detail-actions">
          <button
            type="button"
            class="fluent-control"
            @click="deployGlobal(selected.name)"
          >
            {{ t("ai.skills.deployGlobal") }}
          </button>
          <label class="skills-field skills-project-pick">
            <span>{{ t("ai.skills.targetProject") }}</span>
            <select v-model="targetProjectPath" class="fluent-control">
              <option value="" disabled>{{ t("ai.skills.pickProject") }}</option>
              <option v-for="p in projects" :key="p.path" :value="p.path">
                {{ p.displayName }}
              </option>
            </select>
          </label>
          <button
            type="button"
            class="fluent-control fluent-control--accent"
            :disabled="!targetProjectPath"
            @click="deployToSelectedProject(selected.name)"
          >
            {{ t("ai.skills.addToProject") }}
          </button>
          <button
            type="button"
            class="fluent-control"
            :disabled="!targetProjectPath"
            @click="undeployFromSelectedProject(selected.name)"
          >
            {{ t("ai.skills.removeFromProject") }}
          </button>
        </div>
        <p class="skills-hint">{{ t("ai.skills.projectDefaultHint") }}</p>
        <p v-if="lastDeployNote" class="skills-hint">{{ lastDeployNote }}</p>
      </article>
      <article v-else class="fluent-surface skills-detail skills-detail--empty">
        <p class="skills-empty">{{ t("ai.skills.selectHint") }}</p>
      </article>
    </div>
  </div>
</template>

<style scoped>
.skills-panel {
  margin-top: var(--fluent-space-md);
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
  min-width: 0;
}

.skills-toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--fluent-space-sm);
}

.skills-filter {
  flex: 1;
  min-width: 12rem;
}

.skills-summary,
.skills-hint,
.skills-empty {
  margin: 0;
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
}

.skills-tools {
  padding: var(--fluent-space-md);
}

.skills-tools-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: var(--fluent-space-md);
}

@media (max-width: 960px) {
  .skills-tools-grid {
    grid-template-columns: 1fr;
  }
}

.skills-tool-col {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
  min-width: 0;
}

.skills-harvest-scroll {
  max-height: 8rem;
  overflow: auto;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.skills-split {
  display: grid;
  grid-template-columns: minmax(12rem, 18rem) minmax(0, 1fr);
  gap: var(--fluent-space-md);
  min-height: 22rem;
  align-items: stretch;
}

@media (max-width: 720px) {
  .skills-split {
    grid-template-columns: 1fr;
  }
}

.skills-list-pane {
  padding: var(--fluent-space-sm);
  min-height: 0;
  max-height: 32rem;
  overflow: auto;
}

.skills-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.skills-list-item {
  padding: var(--fluent-space-sm) var(--fluent-space-md);
  border-radius: var(--fluent-radius-md);
  cursor: pointer;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.skills-list-item:hover {
  background: var(--fluent-color-fill-control-hover);
}

.skills-list-item--active {
  background: var(--fluent-color-fill-selected);
  font-weight: var(--fluent-font-weight-semibold);
  box-shadow: inset 3px 0 0 var(--fluent-color-stroke-accent-soft);
}

.skills-detail {
  padding: var(--fluent-space-md) var(--fluent-space-lg);
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
  min-width: 0;
}

.skills-detail--empty {
  align-items: center;
  justify-content: center;
}

.skills-detail-head {
  display: flex;
  justify-content: space-between;
  gap: var(--fluent-space-md);
  align-items: flex-start;
}

.skills-detail-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-end;
  gap: var(--fluent-space-sm);
}

.skills-project-pick {
  min-width: 14rem;
  flex: 1;
}

.skills-subhead {
  margin: 0;
  font-size: var(--fluent-font-size-body);
}

.skills-row {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fluent-space-sm);
}

.skills-grow {
  flex: 1;
  min-width: 8rem;
}

.skills-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.skills-check {
  display: flex;
  gap: var(--fluent-space-sm);
  align-items: center;
  font-size: var(--fluent-font-size-caption);
}
</style>
