<template>
  <div class="output-area">
    <div class="output-header">
      <h2 class="output-title">Results</h2>
    </div>

    <CollapsibleAcbAppOutputViewSettings :store="store" />

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
        class="acb-output"
        :class="{ inactive: store.activeViewMode !== ViewMode.SecurityTables }"
      >
        <SecurityTablesOutput :store="store" />
      </div>
      <div
        class="acb-output"
        :class="{ inactive: store.activeViewMode !== ViewMode.Summary }"
      >
        <DataTable
          v-if="store.summaryTable"
          :table="store.summaryTable"
          title="Summary"
        />
      </div>
      <div
        class="acb-output"
        :class="{ inactive: store.activeViewMode !== ViewMode.Aggregate }"
      >
        <DataTable
          v-if="store.aggregateTable"
          :table="store.aggregateTable"
          title="Aggregate Gains"
        />
      </div>
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
import DataTable from './DataTable.vue';
import SecurityTablesOutput from './SecurityTablesOutput.vue';
import CollapsibleAcbAppOutputViewSettings from './CollapsibleAcbAppOutputViewSettings.vue';

export default defineComponent({
   name: 'OutputArea',
   components: { DataTable, SecurityTablesOutput, CollapsibleAcbAppOutputViewSettings },
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
