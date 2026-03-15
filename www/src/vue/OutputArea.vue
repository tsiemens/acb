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

<style scoped>
.output-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 15px;
}

.output-title {
  font-size: 20px;
  font-weight: 600;
}

.loading-spinner {
  display: none;
  text-align: center;
  padding: 20px;
}

.spinner {
  width: 40px;
  height: 40px;
  border: 4px solid rgba(0, 0, 0, 0.1);
  border-radius: 50%;
  border-top-color: var(--primary-color);
  animation: spin 1s ease-in-out infinite;
  margin: 0 auto;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.view-mode-toggle {
  display: flex;
  gap: 5px;
  margin-bottom: 15px;
}

.view-mode-btn {
  padding: 5px 10px;
  border: 1px solid #ddd;
  border-radius: 4px;
  background-color: #f8f9fa;
  cursor: pointer;
}

.view-mode-btn.active {
  background-color: var(--primary-color);
  color: white;
  border-color: var(--primary-color);
}

.view-mode-btn:hover {
  background-color: #e3e3e3;
}

.view-mode-btn.active:hover {
  /* No hover color while active */
  background-color: var(--primary-color);
}

.acb-output-container {
  margin: auto;
  width: -moz-fit-content;
  width: fit-content;
}

.acb-output.inactive {
  display: none;
}
</style>
