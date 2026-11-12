<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/**
 * 项目侧 AI 区：横向双栏；默认无 Skill/MCP，需显式添加到本项目。
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
const mcpFilter = ref("");
const skillFilter = ref("");

const enabledSet = computed(() => new Set(projectAi.value?.enabledAgentIds ?? []));

const filteredMcp = computed(() => {
  const q = mcpFilter.value.trim().toLowerCase();
  if (!q) return mcpEntries.value;
  return mcpEntries.value.filter(
    (e) => e.id.toLowerCase().includes(q) || e.displayName.toLowerCase().includes(q),
  );
});

const filteredSkills = computed(() => {
  const q = skillFilter.value.trim().toLowerCase();
  if (!q) return skills.value;
  return skills.value.filter(
    (s) => s.name.toLowerCase().includes(q) || s.path.toLowerCase().includes(q),
  );
});

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
  try {
    const report = await aiMcpDeploy(id, props.workspacePath);
    toastSuccess(
      t("ai.mcp.noticeDeployed", {
        count: report.written.length,
        skipped: report.skipped.length,
      }),
    );
  } catch (err) {
    error.value = mapError(err);
  }
}

async function undeployMcp(id: string) {
  error.value = null;
  try {
    const report = await aiMcpUndeploy(id, props.workspacePath);
    toastSuccess(t("ai.mcp.noticeUndeployed", { count: report.written.length }));
  } catch (err) {
    error.value = mapError(err);
  }
}

async function deploySkill(name: string) {
  error.value = null;
  try {
    const report = await aiSkillsDeploy(name, props.workspacePath);
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

async function undeploySkill(name: string) {
  error.value = null;
  try {
    const report = await aiSkillsUndeploy(name, props.workspacePath);
    toastSuccess(
      t("ai.skills.noticeUndeployed", {
        count: report.deployments.length,
      }),
    );
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
        />
        <ul class="project-ai-scroll">
          <li v-for="entry in filteredMcp" :key="entry.id" class="project-ai-row">
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
        />
        <ul class="project-ai-scroll">
          <li v-for="skill in filteredSkills" :key="skill.name" class="project-ai-row">
            <div class="project-ai-main">
              <strong>{{ skill.name }}</strong>
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
.project-ai-meta {
  margin: 0;
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
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
  align-items: center;
  justify-content: space-between;
  gap: var(--fluent-space-sm);
  padding: 4px 0;
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
.project-ai-actions {
  display: flex;
  gap: 4px;
  flex-shrink: 0;
}
</style>
