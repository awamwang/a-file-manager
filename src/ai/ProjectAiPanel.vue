<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/**
 * 项目侧 AI 区：项目 Agent 启用集 + 从中央挑选 MCP/Skills 部署/卸载。
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
  aiSkillsDeploy,
  aiSkillsLibraryGet,
  aiSkillsUndeploy,
} from "./client";
import type {
  AgentTarget,
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
const note = ref<string | null>(null);

const enabledSet = computed(() => new Set(projectAi.value?.enabledAgentIds ?? []));

function mapError(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
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

async function deployMcp(id: string) {
  error.value = null;
  note.value = null;
  try {
    const report = await aiMcpDeploy(id, props.workspacePath);
    note.value = t("ai.mcp.noticeDeployed", {
      count: report.written.length,
      skipped: report.skipped.length,
    });
    toastSuccess(note.value);
  } catch (err) {
    error.value = mapError(err);
  }
}

async function undeployMcp(id: string) {
  error.value = null;
  try {
    const report = await aiMcpUndeploy(id, props.workspacePath);
    note.value = t("ai.mcp.noticeUndeployed", { count: report.written.length });
    toastSuccess(note.value);
  } catch (err) {
    error.value = mapError(err);
  }
}

async function deploySkill(name: string) {
  error.value = null;
  note.value = null;
  try {
    const report = await aiSkillsDeploy(name, props.workspacePath);
    note.value = t("ai.skills.noticeDeployed", {
      count: report.deployments.length,
      skipped: report.skipped.length,
    });
    toastSuccess(note.value);
  } catch (err) {
    error.value = mapError(err);
  }
}

async function undeploySkill(name: string) {
  error.value = null;
  try {
    const report = await aiSkillsUndeploy(name, props.workspacePath);
    note.value = t("ai.skills.noticeUndeployed", {
      count: report.deployments.length,
    });
    toastSuccess(note.value);
  } catch (err) {
    error.value = mapError(err);
  }
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
    <h3 class="project-ai-title">{{ t("ai.project.title") }}</h3>
    <p class="project-ai-hint">{{ t("ai.project.hint") }}</p>
    <p v-if="loading" class="fluent-banner">{{ t("common.loading") }}</p>
    <p v-else-if="error" class="fluent-banner fluent-banner--error">{{ error }}</p>
    <p v-if="note" class="project-ai-hint">{{ note }}</p>

    <section class="fluent-surface project-ai-block">
      <h4 class="project-ai-sub">{{ t("ai.project.agentsTitle") }}</h4>
      <p class="project-ai-hint">{{ t("ai.project.agentsHint") }}</p>
      <ul class="project-ai-list">
        <li v-for="agent in agents" :key="agent.id" class="project-ai-row">
          <label class="project-ai-check">
            <input
              type="checkbox"
              :checked="enabledSet.has(agent.id)"
              @change="toggleAgent(agent.id)"
            />
            <span>
              <strong>{{ agent.displayName }}</strong>
              <span class="project-ai-meta">{{ agent.id }}</span>
            </span>
          </label>
        </li>
      </ul>
    </section>

    <section class="fluent-surface project-ai-block">
      <h4 class="project-ai-sub">{{ t("ai.project.mcpTitle") }}</h4>
      <ul class="project-ai-list">
        <li v-for="entry in mcpEntries" :key="entry.id" class="project-ai-row">
          <div class="project-ai-main">
            <strong>{{ entry.displayName }}</strong>
            <span class="project-ai-meta">{{ entry.id }}</span>
          </div>
          <div class="project-ai-actions">
            <button type="button" class="fluent-control" @click="deployMcp(entry.id)">
              {{ t("ai.project.deploy") }}
            </button>
            <button type="button" class="fluent-control" @click="undeployMcp(entry.id)">
              {{ t("ai.project.undeploy") }}
            </button>
          </div>
        </li>
        <li v-if="!mcpEntries.length" class="project-ai-meta">{{ t("ai.mcp.empty") }}</li>
      </ul>
    </section>

    <section class="fluent-surface project-ai-block">
      <h4 class="project-ai-sub">{{ t("ai.project.skillsTitle") }}</h4>
      <ul class="project-ai-list">
        <li v-for="skill in skills" :key="skill.name" class="project-ai-row">
          <div class="project-ai-main">
            <strong>{{ skill.name }}</strong>
            <span class="project-ai-meta">{{ skill.path }}</span>
          </div>
          <div class="project-ai-actions">
            <button type="button" class="fluent-control" @click="deploySkill(skill.name)">
              {{ t("ai.project.deploy") }}
            </button>
            <button type="button" class="fluent-control" @click="undeploySkill(skill.name)">
              {{ t("ai.project.undeploy") }}
            </button>
          </div>
        </li>
        <li v-if="!skills.length" class="project-ai-meta">{{ t("ai.skills.empty") }}</li>
      </ul>
    </section>
  </div>
</template>

<style scoped>
.project-ai {
  margin-top: var(--fluent-space-lg);
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
}
.project-ai-title {
  margin: 0;
  font-size: var(--fluent-font-size-body);
}
.project-ai-sub {
  margin: 0;
}
.project-ai-hint,
.project-ai-meta {
  margin: 0;
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
}
.project-ai-block {
  padding: var(--fluent-space-md) var(--fluent-space-lg);
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
}
.project-ai-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-xs);
}
.project-ai-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--fluent-space-sm);
  flex-wrap: wrap;
}
.project-ai-check {
  display: flex;
  align-items: flex-start;
  gap: var(--fluent-space-sm);
  cursor: pointer;
}
.project-ai-main {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}
.project-ai-actions {
  display: flex;
  gap: var(--fluent-space-xs);
}
</style>
