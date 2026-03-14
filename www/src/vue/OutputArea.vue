<template>
  <div class="output-area">
    <div class="output-header">
      <h2 class="output-title">Results</h2>
    </div>

    <div id="collapsibleRegionApp"></div>

    <div class="view-mode-toggle">
      <button
        v-for="mode in store.selectableViewModes"
        :key="mode"
        class="view-mode-btn"
        :class="{ active: mode === store.activeViewMode }"
        @click="store.activeViewMode = mode"
      >{{ getViewModeLabel(mode) }}</button>
    </div>

    <div v-if="store.isLoading" class="loading-spinner">
      <div class="spinner"></div>
      <p>Processing files...</p>
    </div>

    <div class="acb-output-container">
      <div
        id="acbSecurityTablesOutput"
        class="acb-output"
        :class="{ inactive: store.activeViewMode !== ViewMode.SecurityTables }"
      ></div>
      <div
        id="acbSummaryOutput"
        class="acb-output"
        :class="{ inactive: store.activeViewMode !== ViewMode.Summary }"
      ></div>
      <div
        id="acbAggregateOutput"
        class="acb-output"
        :class="{ inactive: store.activeViewMode !== ViewMode.Aggregate }"
      ></div>
      <pre
        id="acbTextOutput"
        class="acb-output"
        :class="{ inactive: store.activeViewMode !== ViewMode.Text }"
      >{{ store.textOutput }}</pre>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, type PropType } from 'vue';
import { type OutputStore, AcbOutputViewMode, getViewModeLabel } from './output_store.js';

export default defineComponent({
   name: 'OutputArea',
   props: {
      store: {
         type: Object as PropType<OutputStore>,
         required: true,
      },
   },
   setup() {
      return {
         ViewMode: AcbOutputViewMode,
         getViewModeLabel,
      };
   },
});
</script>
