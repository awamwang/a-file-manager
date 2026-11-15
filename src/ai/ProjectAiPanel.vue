<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/**
 * 项目侧 AI 区：横向双栏；行首复选框即「是否已添加到本项目」的状态开关。
 * 已添加 / 未添加由后端只读扫描项目内的托管 MCP 键与 Skill 目录得出；
 * 列表始终把已添加（启用）的项排在前面，组内保持目录原有顺序。
 */
import { computed, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { toastSuccess } from "../state/toast";
import {
  aiAgentTargetsList,
  aiMcpCatalogList,
  aiMcpDeploy,
  aiMcpUndeploy,
  aiProjectAiGet,
  aiProjectAiSetEnabledAgents,
  aiProjectDeployments,
  aiSkillsDeploy,
  aiSkillsLibraryGet,
  aiSkillsUndeploy,
} from "./client";
import type {
  AgentTarget,
  AiProjectDeployments,
  AiProjectDocument,
  LibrarySkill,
  McpCatalogEntry,
} from "./types";

const props = defineProps<{
  workspacePath: string;
}>();

const { t } = useI18n();

const error = ref<string | null>(null);
const loading = ref(false);
const projectAi = ref<AiProjectDocument | null>(null);
const agents = ref<AgentTarget[]>([]);
const mcpEntries = ref<McpCatalogEntry[]>([]);
const skills = ref<LibrarySkill[]>([]);
const mcpFilter = ref("");
const skillFilter = ref("");
const deployments = ref<AiProjectDeployments>({ mcpIds: [], skillNames: [] });
/** 防止同一行在 IPC 未返回时被重复切换。 */
const pending = ref<Set<string>>(new Set());

const enabledSet = computed(() => new Set(projectAi.value?.enabledAgentIds ?? []));
const deployedMcpSet = computed(() => new Set(deployments.value.mcpIds));
const deployedSkillSet = computed(() => new Set(deployments.value.skillNames));

/**
 * 已添加（启用）优先；用原始下标兜底，避免依赖运行时排序稳定性，
 * 也保证筛选结果在切换勾选前后只有「上移 / 下移」，组内次序不变。
 */
function addedFirst<T>(items: T[], isAdded: (item: T) => boolean): T[] {
  return items
    .map((item, index) => ({ item, index, added: isAdded(item) }))
    .sort((a, b) => Number(b.added) - Number(a.added) || a.index - b.index)
    .map(({ item }) => item);
}

const filteredMcp = computed(() => {
  const q = mcpFilter.value.trim().toLowerCase();
  const matched = q
    ? mcpEntries.value.filter(
        (e) => e.id.toLowerCase().includes(q) || e.displayName.toLowerCase().includes(q),
      )
    : [...mcpEntries.value];
  return addedFirst(matched, (e) => deployedMcpSet.value.has(e.id));
});

const filteredSkills = computed(() => {
  const q = skillFilter.value.trim().toLowerCase();
  const matched = q
    ? skills.value.filter(
        (s) => s.name.toLowerCase().includes(q) || s.path.toLowerCase().includes(q),
      )
    : [...skills.value];
  return addedFirst(matched, (s) => deployedSkillSet.value.has(s.name));
});

function mapError(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

function isPending(kind: "mcp" | "skill", id: string): boolean {
  return pending.value.has(`${kind}:${id}`);
}

function setPending(key: string, value: boolean): void {
  const next = new Set(pending.value);
  if (value) next.add(key);
  else next.delete(key);
  pending.value = next;
}

/** 状态查询失败时退化为「全部未添加」，不让面板整体不可用。 */
async function refreshDeployments(): Promise<void> {
  if (!props.workspacePath.trim()) return;
  try {
    deployments.value = await aiProjectDeployments(props.workspacePath);
  } catch {
    deployments.value = { mcpIds: [], skillNames: [] };
  }
}

async function refresh() {
  if (!props.workspacePath.trim()) return;
  loading.value = true;
  error.value = null;
  try {
    const [aiDoc, agentDoc, mcpDoc, skillStatus] = await Promise.all([
      aiProjectAiGet(props.workspacePath),
      aiAgentTargetsList(),
      aiMcpCatalogList(),
      aiSkillsLibraryGet(),
      refreshDeployments(),
    ]);
    projectAi.value = aiDoc;
    agents.value = agentDoc.agentTargets;
    mcpEntries.value = mcpDoc.mcpCatalog ?? [];
    skills.value = skillStatus.skills;
  } catch (err) {
    error.value = mapError(err);
  } finally {
    loading.value = false;
  }
}

async function toggleAgent(id: string) {
  if (!projectAi.value) return;
  error.value = null;
  const next = new Set(projectAi.value.enabledAgentIds);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  try {
    projectAi.value = await aiProjectAiSetEnabledAgents(
      props.workspacePath,
      [...next],
    );
    toastSuccess(t("ai.project.noticeAgentsSaved"));
  } catch (err) {
    error.value = mapError(err);
  }
}

/** 行首复选框：勾选 = 添加到本项目，取消勾选 = 从本项目移除。 */
async function toggleMcp(id: string) {
  const key = `mcp:${id}`;
  if (isPending("mcp", id)) return;
  setPending(key, true);
  error.value = null;
  try {
    if (deployedMcpSet.value.has(id)) {
      await aiMcpUndeploy(id, props.workspacePath);
      toastSuccess(t("ai.project.noticeRemoved", { name: id }));
    } else {
      await aiMcpDeploy(id, props.workspacePath);
      toastSuccess(t("ai.project.noticeAdded", { name: id }));
    }
    await refreshDeployments();
  } catch (err) {
    error.value = mapError(err);
    await refreshDeployments();
  } finally {
    setPending(key, false);
  }
}

async function toggleSkill(name: string) {
  const key = `skill:${name}`;
  if (isPending("skill", name)) return;
  setPending(key, true);
  error.value = null;
  try {
    if (deployedSkillSet.value.has(name)) {
      await aiSkillsUndeploy(name, props.workspacePath);
      toastSuccess(t("ai.project.noticeRemoved", { name }));
    } else {
      await aiSkillsDeploy(name, props.workspacePath);
      toastSuccess(t("ai.project.noticeAdded", { name }));
    }
    await refreshDeployments();
  } catch (err) {
    error.value = mapError(err);
    await refreshDeployments();
  } finally {
    setPending(key, false);
  }
}

/** 筛选到只剩一个选项时，回车直接切换它的添加状态。 */
function toggleOnlyFiltered(kind: "mcp" | "skill"): void {
  if (kind === "mcp") {
    const only = filteredMcp.value.length === 1 ? filteredMcp.value[0] : null;
    if (only) void toggleMcp(only.id);
    return;
  }
  const only = filteredSkills.value.length === 1 ? filteredSkills.value[0] : null;
  if (only) void toggleSkill(only.name);
}

watch(
  () => props.workspacePath,
  () => {
    void refresh();
  },
);

onMounted(() => {
  void refresh();
});
</script>

<template>
  <div class="project-ai" data-ai-project-panel>
    <header class="project-ai-head">
      <div>
        <h3 class="project-ai-title">{{ t("ai.project.title") }}</h3>
        <p class="project-ai-hint">{{ t("ai.project.hint") }}</p>
      </div>
    </header>
    <p v-if="loading" class="fluent-banner">{{ t("common.loading") }}</p>
    <p v-else-if="error" class="fluent-banner fluent-banner--error">{{ error }}</p>

    <section class="fluent-surface project-ai-block">
      <div class="project-ai-block-head">
        <h4 class="project-ai-sub">{{ t("ai.project.agentsTitle") }}</h4>
        <p class="project-ai-hint">{{ t("ai.project.agentsHint") }}</p>
      </div>
      <div class="project-ai-agents">
        <label v-for="agent in agents" :key="agent.id" class="project-ai-chip">
          <input
            type="checkbox"
            :checked="enabledSet.has(agent.id)"
            @change="toggleAgent(agent.id)"
          />
          <span>{{ agent.displayName }}</span>
        </label>
      </div>
    </section>

    <div class="project-ai-cols">
      <section class="fluent-surface project-ai-block">
        <div class="project-ai-block-head">
          <h4 class="project-ai-sub">{{ t("ai.project.mcpTitle") }}</h4>
          <p class="project-ai-hint">{{ t("ai.project.catalogHint") }}</p>
        </div>
        <input
          v-model="mcpFilter"
          class="fluent-control"
          type="search"
          :placeholder="t('ai.mcp.filterPlaceholder')"
          @keydown.enter.prevent="toggleOnlyFiltered('mcp')"
        />
        <p v-if="filteredMcp.length === 1" class="project-ai-enter-hint">
          {{ t("ai.project.filterEnterHint") }}
        </p>
        <ul class="project-ai-scroll">
          <li
            v-for="entry in filteredMcp"
            :key="entry.id"
            class="project-ai-row"
            :class="{ 'project-ai-row--added': deployedMcpSet.has(entry.id) }"
          >
            <label class="project-ai-pick">
              <input
                type="checkbox"
                class="project-ai-check"
                :checked="deployedMcpSet.has(entry.id)"
                :disabled="isPending('mcp', entry.id)"
                :title="
                  deployedMcpSet.has(entry.id)
                    ? t('ai.project.undeploy')
                    : t('ai.project.deploy')
                "
                @click.prevent="toggleMcp(entry.id)"
              />
              <span class="project-ai-main">
                <strong>{{ entry.displayName }}</strong>
                <span class="project-ai-meta">{{ entry.id }}</span>
              </span>
              <span
                class="project-ai-state"
                :class="
                  deployedMcpSet.has(entry.id)
                    ? 'project-ai-state--added'
                    : 'project-ai-state--missing'
                "
              >
                {{
                  deployedMcpSet.has(entry.id)
                    ? t("ai.project.added")
                    : t("ai.project.notAdded")
                }}
              </span>
            </label>
          </li>
          <li v-if="!filteredMcp.length" class="project-ai-meta">{{ t("ai.mcp.empty") }}</li>
        </ul>
      </section>

      <section class="fluent-surface project-ai-block">
        <div class="project-ai-block-head">
          <h4 class="project-ai-sub">{{ t("ai.project.skillsTitle") }}</h4>
          <p class="project-ai-hint">{{ t("ai.project.skillsDefaultHint") }}</p>
        </div>
        <input
          v-model="skillFilter"
          class="fluent-control"
          type="search"
          :placeholder="t('ai.skills.filterPlaceholder')"
          @keydown.enter.prevent="toggleOnlyFiltered('skill')"
        />
        <p v-if="filteredSkills.length === 1" class="project-ai-enter-hint">
          {{ t("ai.project.filterEnterHint") }}
        </p>
        <ul class="project-ai-scroll">
          <li
            v-for="skill in filteredSkills"
            :key="skill.name"
            class="project-ai-row"
            :class="{ 'project-ai-row--added': deployedSkillSet.has(skill.name) }"
          >
            <label class="project-ai-pick">
              <input
                type="checkbox"
                class="project-ai-check"
                :checked="deployedSkillSet.has(skill.name)"
                :disabled="isPending('skill', skill.name)"
                :title="
                  deployedSkillSet.has(skill.name)
                    ? t('ai.project.undeploy')
                    : t('ai.project.deploy')
                "
                @click.prevent="toggleSkill(skill.name)"
              />
              <span class="project-ai-main">
                <strong>{{ skill.name }}</strong>
              </span>
              <span
                class="project-ai-state"
                :class="
                  deployedSkillSet.has(skill.name)
                    ? 'project-ai-state--added'
                    : 'project-ai-state--missing'
                "
              >
                {{
                  deployedSkillSet.has(skill.name)
                    ? t("ai.project.added")
                    : t("ai.project.notAdded")
                }}
              </span>
            </label>
          </li>
          <li v-if="!filteredSkills.length" class="project-ai-meta">{{ t("ai.skills.empty") }}</li>
        </ul>
      </section>
    </div>
  </div>
</template>

<style scoped>
.project-ai {
  margin-top: var(--fluent-space-lg);
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
  min-width: 0;
}
.project-ai-head {
  display: flex;
  justify-content: space-between;
  gap: var(--fluent-space-md);
  flex-wrap: wrap;
  align-items: flex-start;
}
.project-ai-title,
.project-ai-sub {
  margin: 0;
  font-size: var(--fluent-font-size-body);
}
.project-ai-hint,
.project-ai-meta,
.project-ai-enter-hint {
  margin: 0;
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
  -webkit-user-select: text;
  user-select: text;
}
.project-ai-enter-hint {
  color: var(--fluent-color-stroke-accent-soft);
}
.project-ai-block {
  padding: var(--fluent-space-md);
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
  min-width: 0;
}
.project-ai-block-head {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.project-ai-agents {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fluent-space-sm);
}
.project-ai-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  border-radius: var(--fluent-radius-md);
  background: var(--fluent-color-fill-subtle);
  font-size: var(--fluent-font-size-caption);
  cursor: pointer;
}
.project-ai-cols {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--fluent-space-md);
  align-items: stretch;
}
@media (max-width: 900px) {
  .project-ai-cols {
    grid-template-columns: 1fr;
  }
}
.project-ai-scroll {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-xs);
  max-height: 18rem;
  overflow: auto;
}
.project-ai-row {
  display: flex;
  align-items: stretch;
  padding: 0 2px;
  border-radius: var(--fluent-radius-md);
}
.project-ai-row:hover {
  background: var(--fluent-color-fill-subtle);
}
.project-ai-row--added {
  box-shadow: inset 3px 0 0 var(--fluent-color-stroke-accent-soft);
}
.project-ai-pick {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
  flex: 1;
  min-width: 0;
  padding: 4px 6px;
  cursor: pointer;
}
.project-ai-check {
  flex: 0 0 auto;
  width: 16px;
  height: 16px;
  margin: 0;
  cursor: pointer;
}
.project-ai-main {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
  overflow: hidden;
}
.project-ai-main strong {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.project-ai-state {
  flex: 0 0 auto;
  margin-inline-start: auto;
  align-self: center;
  padding: 2px 8px;
  border-radius: var(--fluent-radius-md);
  font-size: var(--fluent-font-size-caption);
  white-space: nowrap;
}
.project-ai-state--added {
  background: var(--fluent-color-fill-selected);
  color: var(--fluent-color-stroke-accent-soft);
}
.project-ai-state--missing {
  background: var(--fluent-color-fill-subtle);
  color: var(--fluent-color-text-secondary);
}
</style>
