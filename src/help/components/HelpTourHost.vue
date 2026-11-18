<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<script setup lang="ts">
/** Single host for the globally active guided tour; mounted once in AppShell. */
import GuidedTour from "./GuidedTour.vue";
import { activeTour, stopTour } from "../tourState.ts";
</script>

<template>
  <GuidedTour
    v-if="activeTour"
    :steps="activeTour.steps"
    :tour-id="activeTour.id"
    :record-progress="activeTour.recordProgress"
    @finish="stopTour"
  />
</template>
