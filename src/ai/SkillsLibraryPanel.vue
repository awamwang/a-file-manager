<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/**
 * AIA-216 / A3 — Public Skill library UI + Harvest + 部署方式 + 项目分发。
 */
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
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
  aiSkillsSearchProject,
  aiSkillsUndeploy,
} from "./client";
import type {
  AgentTarget,
  HarvestSkillCandidate,
  ProjectSkillHit,
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
const projectWorkspace = ref("");
const lastDeployNote = ref<string | null>(null);
const projectHits = ref<ProjectSkillHit[]>([]);
const skillDeployMode = ref("junction");
const agents = ref<AgentTarget[]>([]);
const harvestAgentId = ref("");
const harvestCandidates = ref<HarvestSkillCandidate[]>([]);
const harvestSelected = ref<Record<string, boolean>>({});

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

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    const [next, agentDoc] = await Promise.all([
      aiSkillsLibraryGet(),
      aiAgentTargetsList(),
    ]);
    status.value = next;
    rootDraft.value = next.root;
    skillDeployMode.value = agentDoc.skillDeployMode || "junction";
    agents.value = agentDoc.agentTargets;
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

function selectSkill(name: string) {
  selectedName.value = name;
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

async function deployToProject(name: string) {
  const ws = projectWorkspace.value.trim();
  if (!ws) {
    error.value = t("ai.skills.errorDeployProject");
    return;
  }
  error.value = null;
  lastDeployNote.value = null;
  try {
    const report: SkillDeployReport = await aiSkillsDeploy(name, ws);
    lastDeployNote.value = t("ai.skills.noticeDeployed", {
      count: report.deployments.length,
      skipped: report.skipped.length,
    });
    toastSuccess(lastDeployNote.value);
  } catch (err) {
    error.value = mapError(err);
  }
}

async function undeploySkill(name: string) {
  error.value = null;
  lastDeployNote.value = null;
  try {
    const ws = projectWorkspace.value.trim() || null;
    const report: SkillDeployReport = await aiSkillsUndeploy(name, ws);
    lastDeployNote.value = t("ai.skills.noticeUndeployed", {
      count: report.deployments.length,
    });
    toastSuccess(lastDeployNote.value);
  } catch (err) {
    error.value = mapError(err);
  }
}

async function searchProject() {
  if (!projectWorkspace.value.trim()) return;
  error.value = null;
  try {
    projectHits.value = await aiSkillsSearchProject(projectWorkspace.value.trim());
  } catch (err) {
    error.value = mapError(err);
  }
}

async function importFromHit(path: string, name: string) {
  sourcePath.value = path;
  importName.value = name;
  await importSkill();
}

async function scanHarvest() {
  if (!harvestAgentId.value) return;
  error.value = null;
  try {
    const report = await aiHarvestSkillsScan(harvestAgentId.value);
    harvestCandidates.value = report.candidates;
    const selected: Record<string, boolean> = {};
    for (const c of report.candidates) {
      selected[c.name] = !c.alreadyInLibrary;
    }
    harvestSelected.value = selected;
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
      <label class="skills-field">
        <span>{{ t("ai.skills.deployMode") }}</span>
        <select v-model="skillDeployMode" class="fluent-control" @change="saveDeployMode">
          <option value="junction">{{ t("ai.skills.deployModeJunction") }}</option>
          <option value="copy">{{ t("ai.skills.deployModeCopy") }}</option>
        </select>
      </label>
      <p class="skills-hint">{{ t("ai.skills.deployModeHint") }}</p>
    </div>

    <div class="fluent-surface skills-block">
      <h3 class="skills-subhead">{{ t("ai.harvest.skillsTitle") }}</h3>
      <p class="skills-hint">{{ t("ai.harvest.skillsHint") }}</p>
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
      <ul v-if="harvestCandidates.length" class="skills-list">
        <li v-for="c in harvestCandidates" :key="c.name" class="skills-list-item">
          <label class="skills-check">
            <input v-model="harvestSelected[c.name]" type="checkbox" :disabled="c.alreadyInLibrary" />
            <span>
              <strong>{{ c.name }}</strong>
              <span class="skills-list-meta">
                {{ c.path }}
                <template v-if="c.alreadyInLibrary"> · {{ t("ai.harvest.alreadyIn") }}</template>
              </span>
            </span>
          </label>
        </li>
      </ul>
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

    <div class="fluent-surface skills-block">
      <h3 class="skills-subhead">{{ t("ai.skills.deployTitle") }}</h3>
      <p class="skills-hint">{{ t("ai.skills.deployHint") }}</p>
      <label class="skills-field">
        <span>{{ t("ai.skills.fieldProjectWorkspace") }}</span>
        <input
          v-model="projectWorkspace"
          class="fluent-control"
          type="text"
          :placeholder="t('ai.skills.fieldProjectWorkspaceHint')"
        />
      </label>
      <div class="skills-row">
        <button type="button" class="fluent-control" @click="searchProject">
          {{ t("ai.skills.searchProject") }}
        </button>
      </div>
      <ul v-if="projectHits.length" class="skills-list">
        <li v-for="hit in projectHits" :key="hit.path" class="skills-list-item">
          <div class="skills-list-main">
            <strong>{{ hit.name }}</strong>
            <span class="skills-list-meta">{{ hit.path }}</span>
          </div>
          <button
            type="button"
            class="fluent-control"
            @click="importFromHit(hit.path, hit.name)"
          >
            {{ t("ai.skills.import") }}
          </button>
        </li>
      </ul>
      <p v-if="lastDeployNote" class="skills-hint">{{ lastDeployNote }}</p>
    </div>

    <div v-if="status" class="skills-list-wrap">
      <p class="skills-summary">
        {{ t("ai.skills.summary", { count: status.skills.length }) }}
      </p>
      <ul
        v-if="status.skills.length"
        class="skills-list"
        role="listbox"
        :aria-label="t('ai.sections.skills.title')"
      >
        <li
          v-for="skill in status.skills"
          :key="skill.name"
          class="skills-list-item"
          :class="{ 'skills-list-item--active': skill.name === selectedName }"
          role="option"
          :aria-selected="skill.name === selectedName"
          @click="selectSkill(skill.name)"
        >
          <div class="skills-list-main">
            <strong>{{ skill.name }}</strong>
            <span class="skills-list-meta">{{ skill.path }}</span>
          </div>
          <div class="skills-row">
            <button
              type="button"
              class="fluent-control"
              @click.stop="deployGlobal(skill.name)"
            >
              {{ t("ai.skills.deployGlobal") }}
            </button>
            <button
              type="button"
              class="fluent-control"
              @click.stop="deployToProject(skill.name)"
            >
              {{ t("ai.skills.addToProject") }}
            </button>
            <button
              type="button"
              class="fluent-control"
              @click.stop="undeploySkill(skill.name)"
            >
              {{ t("ai.skills.undeploy") }}
            </button>
            <button
              type="button"
              class="fluent-control"
              @click.stop="removeSkill(skill.name)"
            >
              {{ t("common.remove") }}
            </button>
          </div>
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
  align-items: center;
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
  justify-content: space-between;
  gap: var(--fluent-space-sm);
  flex-wrap: wrap;
  padding: var(--fluent-space-sm) var(--fluent-space-md);
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-subtle);
  cursor: pointer;
}

.skills-list-item--active {
  background: var(--fluent-color-fill-selected);
  box-shadow: inset 3px 0 0 var(--fluent-color-stroke-accent-soft);
}

.skills-list-main {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.skills-check {
  display: flex;
  align-items: flex-start;
  gap: var(--fluent-space-sm);
  cursor: pointer;
}
</style>
