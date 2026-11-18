<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!--
  知识页密钥 chip（ADR 0017）：markdown 文内掩码 token（[REDACTED:…] / [vault:…]）
  渲染为行内可交互徽标——复制（Rust 直写剪贴板）/ 显示（5s 短显自动回掩码）。
  明文边界：明文只经 vaultChannel 非响应式 Map 短显 5s，不进 Vue 响应式状态、不进日志。
-->
<script setup lang="ts">
import { computed } from "vue";
import {
  isSecretRevealed,
  maskSecretReveal,
  revealedValueOf,
} from "../../state/vaultChannel.ts";
import { takeVaultSecret } from "../../state/vaultAccess.ts";
import { toastError, toastInfo } from "../../state/toast.ts";
import FluentIcon from "../icons/FluentIcon.vue";

const props = defineProps<{
  /** vault 条目名（[REDACTED:label:hash8]，reveal/copy 的取用键）。 */
  entry: string;
  /** 展示标签（token label）。 */
  label: string;
  /** 翻译函数（宿主页传入，避免挂载点 appContext 依赖）。 */
  t: (key: string) => string;
}>();

const revealed = computed(() => isSecretRevealed(props.entry));

function onCopy(): void {
  void takeVaultSecret(props.t, props.entry, "copy").then((notice) => {
    if (!notice) return;
    if (notice.kind === "error") toastError(props.t(notice.toastKey));
    else toastInfo(props.t(notice.toastKey));
  });
}

function onToggleReveal(): void {
  if (revealed.value) {
    maskSecretReveal(props.entry);
    return;
  }
  void takeVaultSecret(props.t, props.entry, "reveal").then((notice) => {
    if (notice && notice.kind === "error") {
      toastError(props.t(notice.toastKey));
    }
  });
}
</script>

<template>
  <span
    class="kchip"
    :class="{ 'kchip--revealed': revealed }"
    :title="props.entry"
    :data-entry="props.entry"
  >
    <FluentIcon name="lock" size="sm" class="kchip-icon" />
    <span v-if="revealed" class="kchip-value">{{
      revealedValueOf(props.entry)
    }}</span>
    <span v-else class="kchip-label">{{ props.label }}</span>
    <button type="button" class="kchip-btn" @click="onCopy">
      {{ props.t("knowledgeView.chip.copy") }}
    </button>
    <button type="button" class="kchip-btn" @click="onToggleReveal">
      {{
        revealed
          ? props.t("knowledgeView.chip.hide")
          : props.t("knowledgeView.chip.show")
      }}
    </button>
  </span>
</template>

<style scoped>
.kchip {
  display: inline-flex;
  align-items: center;
  gap: var(--fluent-space-xxs, 2px);
  padding: 0 4px;
  border: var(--fluent-stroke-width, 1px) solid
    color-mix(in srgb, var(--afm-vault-purple) 55%, transparent);
  border-radius: 999px;
  background: color-mix(in srgb, var(--afm-vault-purple) 12%, transparent);
  color: var(--afm-vault-purple);
  font-size: 0.85em;
  line-height: 1.4;
  vertical-align: baseline;
  white-space: nowrap;
}

.kchip--revealed {
  background: color-mix(in srgb, var(--afm-vault-purple) 20%, transparent);
  font-weight: var(--fluent-font-weight-semibold, 600);
}

.kchip-icon {
  flex-shrink: 0;
}

.kchip-label {
  max-width: 14em;
  overflow: hidden;
  text-overflow: ellipsis;
}

.kchip-value {
  font-family: ui-monospace, Consolas, monospace;
  letter-spacing: 0.02em;
  max-width: 28em;
  overflow: hidden;
  text-overflow: ellipsis;
}

.kchip-btn {
  padding: 0 4px;
  border: 0;
  background: transparent;
  color: inherit;
  font: inherit;
  font-size: 0.92em;
  cursor: pointer;
  text-decoration: underline;
  text-underline-offset: 2px;
  opacity: 0.85;
}

.kchip-btn:hover,
.kchip-btn:focus-visible {
  opacity: 1;
}
</style>
