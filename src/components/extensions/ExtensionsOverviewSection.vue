<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/**
 * 扩展总览：每个能力后端一张卡片，显示 mode（managed/config/builtin/absent）+ 健康态 + 主操作。
 * ADR-0023: 统一收口两种扩展形态（安装 Pack vs config mode）。
 */
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { useRouter } from "vue-router";
import FluentIcon from "../icons/FluentIcon.vue";
import type { FluentGlyphId } from "../icons/glyphs.ts";

const { t } = useI18n();
const router = useRouter();

type ManagedRecord = {
  id: string;
  packId: string;
  mode: string;
  health: string;
  spawnedPid: number | null;
  declaredVersion: string;
  installedVersion: string;
  availableVersion: string;
};

type InstalledPack = {
  id: string;
  name: string;
  version: string;
  state: string;
};

type CapabilityDef = {
  packId: string;
  nameKey: string;
  descKey: string;
  icon: FluentGlyphId;
  section: string;
};

const CAPABILITIES: CapabilityDef[] = [
  {
    packId: "awam.file-gateway",
    nameKey: "extensions.cap.fileGateway.name",
    descKey: "extensions.cap.fileGateway.desc",
    icon: "cloud",
    section: "providers",
  },
  {
    packId: "awam.knowledge",
    nameKey: "extensions.cap.knowledge.name",
    descKey: "extensions.cap.knowledge.desc",
    icon: "search",
    section: "config",
  },
  {
    packId: "awam.tool-runtime",
    nameKey: "extensions.cap.toolRuntime.name",
    descKey: "extensions.cap.toolRuntime.desc",
    icon: "flash",
    section: "tool-runtime",
  },
];

const managed = ref<Record<string, ManagedRecord>>({});
const packs = ref<InstalledPack[]>([]);
const loading = ref(false);

type CapStatus = {
  mode: "managed" | "config" | "builtin" | "absent";
  health: "online" | "offline" | "unknown";
};

function resolveCapStatus(cap: CapabilityDef): CapStatus {
  const rec = managed.value[cap.packId];
  const pack = packs.value.find((p) => p.id === cap.packId);

  if (rec && rec.mode === "managed") {
    const healthy = rec.health === "healthy" || rec.health === "online";
    return { mode: "managed", health: healthy ? "online" : "offline" };
  }
  if (pack && pack.state === "enabled") {
    // Pack enabled but no managed record → config mode (user runs the service themselves)
    return { mode: "config", health: "unknown" };
  }
  if (rec && rec.mode === "config") {
    return { mode: "config", health: rec.health === "healthy" ? "online" : "offline" };
  }
  return { mode: "absent", health: "unknown" };
}

const capStatuses = computed(() =>
  CAPABILITIES.map((cap) => ({
    ...cap,
    status: resolveCapStatus(cap),
  })),
);

function modeLabel(mode: string): string {
  return t(`extensions.capMode.${mode}`);
}

function modeChipClass(mode: string): string {
  switch (mode) {
    case "managed": return "fluent-chip--ok";
    case "config": return "fluent-chip--stub";
    case "builtin": return "fluent-chip--ok";
    default: return "fluent-chip--down";
  }
}

function healthChipClass(health: string): string {
  switch (health) {
    case "online": return "fluent-chip--ok";
    case "offline": return "fluent-chip--down";
    default: return "";
  }
}

function primaryAction(section: string) {
  router.push(`/extensions/${section}`);
}

async function load() {
  loading.value = true;
  try {
    const [records, installed] = await Promise.all([
      invoke<ManagedRecord[]>("managed_dependency_list").catch(() => []),
      invoke<InstalledPack[]>("extension_host_list_packs").catch(() => []),
    ]);
    const map: Record<string, ManagedRecord> = {};
    for (const r of records) map[r.packId] = r;
    managed.value = map;
    packs.value = installed;
  } finally {
    loading.value = false;
  }
}

onMounted(() => {
  void load();
});
</script>

<template>
  <div class="cap-overview">
    <p v-if="loading" class="fluent-page-meta">{{ t("common.loading") }}</p>
    <div v-else class="cap-grid">
      <div
        v-for="cap in capStatuses"
        :key="cap.packId"
        class="cap-card fluent-surface"
        :class="{
          'fluent-accent-edge--ok': cap.status.health === 'online',
          'fluent-accent-edge--down': cap.status.mode !== 'absent' && cap.status.health === 'offline',
          'fluent-accent-edge--stub': cap.status.mode === 'absent',
        }"
      >
        <div class="cap-head">
          <span class="fluent-icon-wash"><FluentIcon :name="cap.icon" size="md" /></span>
          <strong class="cap-name">{{ t(cap.nameKey) }}</strong>
        </div>
        <p class="cap-desc">{{ t(cap.descKey) }}</p>
        <div class="cap-chips">
          <span class="fluent-chip fluent-chip--status" :class="modeChipClass(cap.status.mode)">
            {{ modeLabel(cap.status.mode) }}
          </span>
          <span
            v-if="cap.status.mode !== 'absent'"
            class="fluent-chip fluent-chip--status"
            :class="healthChipClass(cap.status.health)"
          >
            {{ cap.status.health === "online" ? t("extensions.capHealth.online") : t("extensions.capHealth.unknown") }}
          </span>
        </div>
        <button
          class="fluent-control cap-action"
          type="button"
          @click="primaryAction(cap.section)"
        >
          {{ cap.status.mode === "absent" ? t("extensions.capAction.install") : t("extensions.capAction.manage") }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.cap-overview {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-md);
}

.cap-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(16rem, 1fr));
  gap: var(--fluent-space-md);
}

.cap-card {
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
  padding: var(--fluent-space-md);
}

.cap-head {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-sm);
}

.cap-name {
  font-weight: var(--fluent-font-weight-semibold);
}

.cap-desc {
  margin: 0;
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
  line-height: 1.4;
}

.cap-chips {
  display: flex;
  flex-wrap: wrap;
  gap: var(--fluent-space-xs);
}

.cap-action {
  align-self: flex-start;
  margin-top: auto;
}
</style>
