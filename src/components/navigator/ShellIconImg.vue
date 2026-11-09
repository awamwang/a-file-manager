<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!-- Isolates iconByPath reactivity so one thumb arrival does not repaint the whole list (AIA-278). -->
<script setup lang="ts">
import { computed } from "vue";
import { iconUrlFor } from "../../state/shellIcons";
import FluentIcon from "../icons/FluentIcon.vue";
import { entryKindIcon } from "../icons/glyphs";

const props = defineProps<{
  path: string;
  size: number;
  isDir: boolean;
}>();

const src = computed(() => iconUrlFor(props.path, props.size));
</script>

<template>
  <img
    v-if="src"
    class="shell-icon"
    :src="src"
    alt=""
    decoding="async"
    :width="size"
    :height="size"
  />
  <FluentIcon v-else :name="entryKindIcon(isDir)" size="md" />
</template>
