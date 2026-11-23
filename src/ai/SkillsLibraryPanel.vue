<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/**
 * 公共 Skill 库：导入区按来源分段 + 库设置独立 Tab；多 Agent 一次扫描合并；
 * 删除需确认；添加到指定单个项目。
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
import {
  mergeSkillCandidates,
  toSkillCandidate,
  type DuplicateReason,
  type HarvestSkillCandidateView,
} from "./harvestDedupe";
import { listProjectOptions, type ProjectOption } from "./projectOptions";
import type {
  AgentTarget,
  HarvestSkillScanReport,
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
const skillDeployMode = ref("junction");
const agents = ref<AgentTarget[]>([]);
const harvestAgentIds = ref<string[]>([]);
const harvestCandidates = ref<HarvestSkillCandidateView[]>([]);
const harvestSelected = ref<Record<string, boolean>>({});
const harvestRename = ref<Record<string, string>>({});
const harvestScanning = ref(false);
const sourceOptions = [
  { source: "agent", labelKey: "ai.skills.fromAgent" },
  { source: "file", labelKey: "ai.skills.fromFolder" },
] as const;
type HarvestSource = (typeof sourceOptions)[number]["source"];
const harvestSource = ref<HarvestSource>("agent");
const toolTab = ref<"import" | "settings">("import");

/** Agent Target 是源头：只有中央 enabled 的 Agent 才能作为导入来源。 */
const enabledAgents = computed(() => agents.value.filter((a) => a.enabled));
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
  AI_HARVEST_AGENT_NOT_ENABLED: "ai.harvest.errorAgentNotEnabled",
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

function duplicateLabel(reason: DuplicateReason): string {
  if (reason === "existing-name") return t("ai.harvest.reasonExistingName");
  if (reason === "name-conflict") return t("ai.harvest.reasonNameConflict");
  return "";
}

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
    const validIds = enabledAgents.value.map((a) => a.id);
    const stillValid = harvestAgentIds.value.filter((id) => validIds.includes(id));
    harvestAgentIds.value = stillValid.length ? stillValid : validIds;
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
  try {
    const report: SkillDeployReport = await aiSkillsDeploy(name, null);
    toastSuccess(
      t("ai.skills.noticeDeployed", {
        count: report.deployments.length,
        skipped: report.skipped.length,
      }),
    );
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
  try {
    const report: SkillDeployReport = await aiSkillsDeploy(name, ws);
    const project =
      projects.value.find((p) => p.path === ws)?.displayName ?? ws;
    toastSuccess(
      t("ai.skills.noticeDeployedToProject", {
        project,
        count: report.deployments.length,
        skipped: report.skipped.length,
      }),
    );
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
  try {
    const report: SkillDeployReport = await aiSkillsUndeploy(name, ws);
    toastSuccess(
      t("ai.skills.noticeUndeployed", {
        count: report.deployments.length,
      }),
    );
  } catch (err) {
    error.value = mapError(err);
  }
}

function toggleHarvestAgent(id: string) {
  const set = new Set(harvestAgentIds.value);
  if (set.has(id)) set.delete(id);
  else set.add(id);
  harvestAgentIds.value = [...set];
}

async function scanHarvest() {
  if (!harvestAgentIds.value.length) return;
  error.value = null;
  harvestScanning.value = true;
  try {
    const results = await Promise.allSettled(
      harvestAgentIds.value.map((id) => aiHarvestSkillsScan(id)),
    );
    const reports: HarvestSkillScanReport[] = [];
    let failed = 0;
    for (const result of results) {
      if (result.status === "fulfilled") reports.push(result.value);
      else failed += 1;
    }
    harvestCandidates.value = mergeSkillCandidates(reports, status.value?.skills ?? []);
    const picked: Record<string, boolean> = {};
    harvestRename.value = {};
    harvestCandidates.value.forEach((c, index) => {
      picked[String(index)] = c.duplicateReason === "none";
    });
    harvestSelected.value = picked;
    if (failed) error.value = t("ai.harvest.partialFailure", { count: failed });
  } catch (err) {
    error.value = mapError(err);
  } finally {
    harvestScanning.value = false;
  }
}

async function importHarvest() {
  const picks = harvestCandidates.value.filter((_c, index) => harvestSelected.value[String(index)]);
  if (!picks.length) return;
  error.value = null;
  try {
    const [next, added, skipped] = await aiHarvestSkillsImport(
      picks.map((c, index) => toSkillCandidate(c, harvestRename.value[String(index)])),
    );
    status.value = next;
    toastSuccess(
      t("ai.harvest.noticeSkillsImported", {
        added: added.length,
        skipped: skipped.length,
      }),
    );
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
    <p v-if="loading" class="fluent-hint">{{ t("common.loading") }}</p>
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
      <div class="skills-tabs" role="tablist">
        <button
          type="button"
          class="skills-tab"
          :class="{ 'skills-tab--active': toolTab === 'import' }"
          role="tab"
          :aria-selected="toolTab === 'import'"
          @click="toolTab = 'import'"
        >
          {{ t("ai.skills.toolsTitle") }}
        </button>
        <button
          type="button"
          class="skills-tab"
          :class="{ 'skills-tab--active': toolTab === 'settings' }"
          role="tab"
          :aria-selected="toolTab === 'settings'"
          @click="toolTab = 'settings'"
        >
          {{ t("ai.skills.tabSettings") }}
        </button>
      </div>

      <div v-if="toolTab === 'import'" class="harvest-block">
        <div class="harvest-segment" role="tablist">
          <button
            v-for="opt in sourceOptions"
            :key="opt.source"
            type="button"
            class="fluent-control harvest-segment-item"
            :class="{ 'harvest-segment-item--active': harvestSource === opt.source }"
            :aria-selected="harvestSource === opt.source"
            role="tab"
            @click="harvestSource = opt.source"
          >
            {{ t(opt.labelKey) }}
          </button>
        </div>

        <div v-if="harvestSource === 'agent'" class="harvest-agent-area">
          <div class="harvest-agent-head">
            <span class="harvest-label">{{ t("ai.harvest.agentsLabel") }}</span>
            <span class="skills-hint">{{ t("ai.harvest.multiSelectHint") }}</span>
          </div>
          <p v-if="!enabledAgents.length" class="skills-hint">
            {{ t("ai.harvest.noSources") }}
          </p>
          <div v-else class="harvest-agents">
            <label v-for="agent in enabledAgents" :key="agent.id" class="harvest-agent-chip">
              <input
                type="checkbox"
                :checked="harvestAgentIds.includes(agent.id)"
                @change="toggleHarvestAgent(agent.id)"
              />
              <span>{{ agent.displayName }}</span>
            </label>
          </div>
          <div class="skills-row harvest-scan-row">
            <button
              type="button"
              class="fluent-control"
              :disabled="!harvestAgentIds.length || harvestScanning"
              @click="scanHarvest"
            >
              {{ harvestScanning ? t("common.loading") : t("ai.harvest.scan") }}
            </button>
            <button
              type="button"
              class="fluent-control"
              @click="harvestAgentIds = enabledAgents.map((a) => a.id)"
            >
              {{ t("ai.harvest.allAgents") }}
            </button>
            <button type="button" class="fluent-control" @click="harvestAgentIds = []">
              {{ t("ai.harvest.clearAgents") }}
            </button>
          </div>

          <p v-if="!harvestCandidates.length" class="skills-hint harvest-empty">
            {{ t("ai.harvest.scanEmpty") }}
          </p>
          <div v-else class="harvest-candidates">
            <div v-for="(c, index) in harvestCandidates" :key="`${c.name}-${index}`" class="harvest-candidate">
              <label class="harvest-candidate-main">
                <input v-model="harvestSelected[String(index)]" type="checkbox" />
                <strong>{{ c.name }}</strong>
                <span class="skills-hint">{{ t("ai.harvest.fromCount", { count: c.sources.length }) }}</span>
              </label>
              <span v-if="c.duplicateReason !== 'none'" class="harvest-reason">
                {{ duplicateLabel(c.duplicateReason) }}
              </span>
              <input
                v-if="c.duplicateReason !== 'none'"
                v-model="harvestRename[String(index)]"
                class="fluent-control harvest-rename"
                type="text"
                :placeholder="t('ai.harvest.renamePlaceholder')"
              />
            </div>
          </div>
          <div class="harvest-footer">
            <button type="button" class="fluent-control" @click="showTools = false">
              {{ t("common.cancel") }}
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
        </div>

        <div v-else class="harvest-file-area">
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
          <p class="skills-hint">{{ t("ai.skills.importHint") }}</p>
          <div class="harvest-footer">
            <button type="button" class="fluent-control" @click="showTools = false">
              {{ t("common.cancel") }}
            </button>
            <button
              type="button"
              class="fluent-control fluent-control--accent"
              :disabled="!sourcePath.trim()"
              @click="importSkill"
            >
              {{ t("ai.skills.import") }}
            </button>
          </div>
        </div>
      </div>

      <div v-else class="skills-settings">
        <label class="skills-field">
          <span>{{ t("ai.skills.libraryRoot") }}</span>
          <input v-model="rootDraft" class="fluent-control" type="text" />
        </label>
        <p class="skills-hint">{{ t("ai.skills.libraryRootHint") }}</p>
        <div class="skills-row">
          <button type="button" class="fluent-control" @click="saveRoot">
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
  -webkit-user-select: text;
  user-select: text;
}

.skills-tools {
  padding: var(--fluent-space-md);
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
}
.skills-tabs {
  display: flex;
  gap: var(--fluent-space-md);
  border-bottom: 0.5px solid var(--fluent-color-stroke-divider);
}
.skills-tab {
  border: 0;
  background: transparent;
  padding: 0 0 var(--fluent-space-xs);
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
  cursor: pointer;
}
.skills-tab--active {
  color: var(--fluent-color-stroke-accent-soft);
  box-shadow: inset 0 -2px 0 var(--fluent-color-stroke-accent-soft);
}
.harvest-block,
.harvest-agent-area,
.harvest-file-area,
.skills-settings {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
}
.harvest-segment {
  display: inline-flex;
  gap: var(--fluent-space-xs);
  align-self: flex-start;
}
.harvest-segment-item--active {
  background: var(--fluent-color-fill-selected);
  color: var(--fluent-color-stroke-accent-soft);
  font-weight: var(--fluent-font-weight-semibold);
}
.harvest-agent-head {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fluent-space-sm);
  align-items: baseline;
}
.harvest-label {
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}
.harvest-agents {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fluent-space-sm);
}
.harvest-agent-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-subtle);
  font-size: var(--fluent-font-size-caption);
  cursor: pointer;
}
.harvest-scan-row {
  align-items: center;
}
.harvest-candidates {
  display: flex;
  flex-direction: column;
  gap: 4px;
  max-height: 11rem;
  overflow: auto;
}
.harvest-candidate {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fluent-space-sm);
  align-items: center;
  padding: 4px 6px;
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-subtle);
}
.harvest-candidate-main {
  display: flex;
  flex: 1;
  min-width: 10rem;
  gap: var(--fluent-space-sm);
  align-items: center;
  cursor: pointer;
}
.harvest-reason {
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}
.harvest-rename {
  min-width: 10rem;
}
.harvest-footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--fluent-space-sm);
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
