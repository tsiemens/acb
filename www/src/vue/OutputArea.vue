<template>
  <div class="output-area">
    <div class="output-header">
      <h2 class="output-title">Results</h2>
    </div>

    <AcbAppOutputViewSettings :store="store" />

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
          :rowClassFn="summaryRowClassFn"
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
          :rowClassFn="aggregateRowClassFn"
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
import { defineComponent, computed, type PropType } from 'vue';
import { InactiveFilterMode, type OutputStore, AcbOutputViewMode, getViewModeLabel, affiliateMatches } from './output_store.js';
import DataTable from './DataTable.vue';
import SecurityTablesOutput from './SecurityTablesOutput.vue';
import AcbAppOutputViewSettings from './AcbAppOutputViewSettings.vue';

export default defineComponent({
   name: 'OutputArea',
   components: { DataTable, SecurityTablesOutput, AcbAppOutputViewSettings },
   props: {
      store: {
         type: Object as PropType<OutputStore>,
         required: true,
      },
   },
   setup(props) {
      function filterClassForMatch(matching: boolean): string {
         if (matching) return '';
         const mode = props.store.inactiveFilterMode;
         return mode === InactiveFilterMode.HideRows ? 'filter-hidden' : 'filter-dimmed';
      }

      function makeRowClassFn(table: { header: string[] }): (row: string[]) => string {
         const affIdx = table.header.findIndex(h => h.toLowerCase() === 'affiliate');
         // "Year" for aggregate table, date columns for summary/tally tables
         const yearIdx = table.header.findIndex(h => h.toLowerCase() === 'year');
         const settleDateIdx = table.header.findIndex(h => h.toLowerCase() === 'settlement date');
         return (row: string[]) => {
            let matching = true;
            if (affIdx >= 0 && props.store.selectedAffiliate) {
               matching = affiliateMatches(row[affIdx], props.store.selectedAffiliate);
            }
            if (matching && props.store.highlightedYear) {
               if (yearIdx >= 0) {
                  matching = row[yearIdx] === props.store.highlightedYear;
               } else if (settleDateIdx >= 0) {
                  const year = row[settleDateIdx]?.split('-')[0];
                  matching = year === props.store.highlightedYear;
               }
            }
            return filterClassForMatch(matching);
         };
      }

      const summaryRowClassFn = computed(() =>
         props.store.summaryTable ? makeRowClassFn(props.store.summaryTable) : null);

      const aggregateRowClassFn = computed(() =>
         props.store.aggregateTable ? makeRowClassFn(props.store.aggregateTable) : null);

      return {
         ViewMode: AcbOutputViewMode,
         getViewModeLabel,
         summaryRowClassFn,
         aggregateRowClassFn,
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

:deep(.filter-dimmed) {
  filter: opacity(0.4);
}

:deep(.filter-hidden) {
  display: none;
}
</style>
