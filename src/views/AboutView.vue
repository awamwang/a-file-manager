<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import FluentIcon from "../components/icons/FluentIcon.vue";
import type { FluentGlyphId } from "../components/icons/glyphs";
import {
  ABOUT_AUTHOR,
  ABOUT_FACTS,
  ABOUT_LINKS,
} from "../config/about.ts";

type HealthPayload = {
  ok: boolean;
  name: string;
  version: string;
  identifier: string;
  aiAddon?: boolean;
};

const { t } = useI18n();
const version = ref<string>("");
const identifier = ref<string>("");

/** Open external links with the OS browser; ignore failures (e.g. web-only preview). */
function openExternal(url: string): void {
  void openUrl(url).catch(() => undefined);
}

type LinkRow = {
  id: string;
  glyph: FluentGlyphId;
  url: string;
};

const linkRows: LinkRow[] = [
  { id: "repository", glyph: "globe", url: ABOUT_LINKS.repository },
  { id: "releases", glyph: "download", url: ABOUT_LINKS.releases },
  { id: "issues", glyph: "bug", url: ABOUT_LINKS.issues },
  { id: "pulls", glyph: "link", url: ABOUT_LINKS.pulls },
];

onMounted(() => {
  void invoke<HealthPayload>("health_get")
    .then((h) => {
      version.value = h.version ?? "";
      identifier.value = h.identifier ?? "";
    })
    .catch(() => undefined);
});
</script>

<template>
  <section class="fluent-page panel">
    <header class="fluent-page-header">
      <h1 class="fluent-page-title">{{ t("about.title") }}</h1>
      <p class="fluent-page-meta">{{ t("about.meta") }}</p>
    </header>

    <!-- Hero: app identity + star call to action -->
    <div class="about-hero fluent-surface">
      <div class="about-brand">
        <span class="about-app-name">{{ t("app.name") }}</span>
        <span v-if="version" class="about-version">v{{ version }}</span>
      </div>
      <p class="about-tagline">{{ t("about.tagline") }}</p>
      <button
        class="fluent-control fluent-control--accent about-star"
        type="button"
        @click="openExternal(ABOUT_LINKS.repository)"
      >
        <FluentIcon name="star" size="sm" />
        <span>{{ t("about.star") }}</span>
      </button>
      <p class="about-star-hint">{{ t("about.starHint") }}</p>
    </div>

    <!-- Facts: version, license, stack, identifier -->
    <dl class="about-facts fluent-surface">
      <dt>{{ t("about.fieldVersion") }}</dt>
      <dd>{{ version || "—" }}</dd>
      <dt>{{ t("about.fieldLicense") }}</dt>
      <dd>
        <button class="about-inline-link" type="button" @click="openExternal(ABOUT_LINKS.license)">
          {{ ABOUT_FACTS.license }}
        </button>
      </dd>
      <dt>{{ t("about.fieldStack") }}</dt>
      <dd>{{ ABOUT_FACTS.stack }}</dd>
      <template v-if="identifier">
        <dt>{{ t("about.fieldIdentifier") }}</dt>
        <dd>{{ identifier }}</dd>
      </template>
    </dl>

    <!-- Links -->
    <h2 class="about-section-title">{{ t("about.linksTitle") }}</h2>
    <ul class="about-links">
      <li v-for="row in linkRows" :key="row.id">
        <button class="about-link fluent-surface" type="button" @click="openExternal(row.url)">
          <span class="fluent-icon-wash">
            <FluentIcon :name="row.glyph" size="md" />
          </span>
          <span class="about-link-text">
            <span class="about-link-label">{{ t(`about.link.${row.id}.label`) }}</span>
            <span class="about-link-desc">{{ t(`about.link.${row.id}.desc`) }}</span>
            <span class="about-link-url">{{ row.url }}</span>
          </span>
          <span class="about-link-arrow" aria-hidden="true">↗</span>
        </button>
      </li>
    </ul>

    <!-- Author -->
    <h2 class="about-section-title">{{ t("about.authorTitle") }}</h2>
    <div class="about-author fluent-surface">
      <span class="about-author-label">{{ t("about.authorLabel") }}</span>
      <button class="about-author-name" type="button" @click="openExternal(ABOUT_AUTHOR.homepage)">
        {{ ABOUT_AUTHOR.name }}
        <FluentIcon name="link" size="sm" />
      </button>
    </div>
  </section>
</template>

<style scoped>
.panel {
  max-width: 44rem;
}

.about-hero {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--fluent-space-sm);
  padding: var(--fluent-space-lg);
}

.about-brand {
  display: flex;
  align-items: baseline;
  gap: var(--fluent-space-sm);
}

.about-app-name {
  font-size: var(--fluent-font-size-subtitle);
  font-weight: var(--fluent-font-weight-semibold);
}

.about-version {
  font-family: var(--fluent-font-family-mono);
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-text-secondary);
}

.about-tagline {
  margin: 0;
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-body);
}

.about-star {
  display: inline-flex;
  align-items: center;
  gap: var(--fluent-space-xs);
  margin-top: var(--fluent-space-xs);
}

.about-star-hint {
  margin: 0;
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
}

.about-facts {
  display: grid;
  grid-template-columns: 8rem 1fr;
  gap: var(--fluent-space-sm) var(--fluent-space-md);
  margin: var(--fluent-space-md) 0 0;
  padding: var(--fluent-space-md) var(--fluent-space-lg);
}

.about-facts dt {
  color: var(--fluent-color-text-secondary);
}

.about-facts dd {
  margin: 0;
  font-family: var(--fluent-font-family-mono);
  font-size: var(--fluent-font-size-caption);
  overflow-wrap: anywhere;
}

.about-inline-link,
.about-author-name {
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--fluent-color-fill-accent);
  font: inherit;
  cursor: pointer;
}

.about-inline-link:hover,
.about-author-name:hover {
  text-decoration: underline;
}

.about-section-title {
  margin: var(--fluent-space-lg) 0 var(--fluent-space-sm);
  font-size: var(--fluent-font-size-subtitle);
}

.about-links {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--fluent-space-sm);
}

.about-link {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  align-items: center;
  gap: var(--fluent-space-md);
  width: 100%;
  padding: var(--fluent-space-sm) var(--fluent-space-lg);
  border: 0;
  text-align: left;
  font: inherit;
  color: inherit;
  cursor: pointer;
}

.about-link:hover {
  background: var(--fluent-color-fill-control-hover);
}

.about-link-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.about-link-label {
  font-weight: var(--fluent-font-weight-semibold);
}

.about-link-desc {
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
}

.about-link-url {
  font-family: var(--fluent-font-family-mono);
  font-size: var(--fluent-font-size-caption);
  color: var(--fluent-color-fill-accent);
  overflow-wrap: anywhere;
  word-break: break-all;
  line-height: 1.4;
}

.about-link:hover .about-link-url {
  text-decoration: underline;
}

.about-link-arrow {
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-body);
  transition:
    transform 120ms ease,
    color 120ms ease;
}

.about-link:hover .about-link-arrow {
  color: var(--fluent-color-fill-accent);
  transform: translate(2px, -2px);
}

.about-author {
  display: flex;
  align-items: center;
  gap: var(--fluent-space-md);
  padding: var(--fluent-space-md) var(--fluent-space-lg);
}

.about-author-label {
  color: var(--fluent-color-text-secondary);
  font-size: var(--fluent-font-size-caption);
}

.about-author-name {
  display: inline-flex;
  align-items: center;
  gap: var(--fluent-space-xs);
  font-size: var(--fluent-font-size-body);
  font-weight: var(--fluent-font-weight-semibold);
}

@media (max-width: 720px) {
  .about-link {
    grid-template-columns: auto minmax(0, 1fr) auto;
    gap: var(--fluent-space-sm);
  }
}
</style>
