<template>
  <div v-if="store.securityTables">
    <DataTable
      v-for="symbol in sortedSymbols"
      :key="symbol"
      v-show="shouldShowSecurity(symbol)"
      :table="getTable(symbol)"
      :title="symbol"
      :rowClassFn="rowClassFn"
      errorSuffix="Information is of parsed state only, and may not be fully correct."
    />
  </div>
</template>

<script lang="ts">
import { defineComponent, computed, type PropType } from 'vue';
import type { OutputStore } from './output_store.js';
import type { RenderTable } from '../acb_wasm_types.js';
import DataTable from './DataTable.vue';

const SETTLE_DATE_COL = 2;
const ACTION_COL = 3;

export default defineComponent({
   name: 'SecurityTablesOutput',
   components: { DataTable },
   props: {
      store: {
         type: Object as PropType<OutputStore>,
         required: true,
      },
   },
   setup(props) {
      const sortedSymbols = computed(() => {
         if (!props.store.securityTables) return [];
         return Array.from(props.store.securityTables.keys()).sort();
      });

      function getTable(symbol: string): RenderTable {
         return props.store.securityTables!.get(symbol)!;
      }

      function getYearsForSymbol(symbol: string): Set<string> {
         const years = new Set<string>();
         for (const row of getTable(symbol).rows) {
            const year = row[SETTLE_DATE_COL]?.split('-')[0];
            if (year) years.add(year);
         }
         return years;
      }

      function shouldShowSecurity(symbol: string): boolean {
         if (!props.store.highlightedYear || !props.store.hideInactiveSecurities) {
            return true;
         }
         const table = getTable(symbol);
         const hasError = table.errors && table.errors.length > 0;
         if (hasError) return true;
         return getYearsForSymbol(symbol).has(props.store.highlightedYear);
      }

      function rowClassFn(row: string[]): string[] {
         const action = row[ACTION_COL] || '';
         let actionClass = 'other-row';
         if (/buy/i.test(action)) actionClass = 'buy-row';
         else if (/sell/i.test(action)) actionClass = 'sell-row';
         else if (/sprf/i.test(action)) actionClass = 'sfla-row';
         else if (/split/i.test(action)) actionClass = 'split-row';

         const year = row[SETTLE_DATE_COL]?.split('-')[0];
         const yearClass = `year-${year || 'unknown'}-row`;

         const dimmed = props.store.highlightedYear && year !== props.store.highlightedYear
            ? 'year-dimmed' : '';

         return [actionClass, yearClass, dimmed].filter(Boolean);
      }

      return { sortedSymbols, getTable, shouldShowSecurity, rowClassFn };
   },
});
</script>

<style scoped>
:deep(.year-dimmed) {
  filter: opacity(0.4);
}

:deep(.buy-row) td {
  background: #fff5e0;
}

:deep(.sell-row) td {
  background: #e9f5ff;
}

:deep(.sfla-row) td {
  background: #fff6fd;
}

:deep(.split-row) td {
  background: #e4ffea;
}

:deep(.other-row) td {
  background: white;
}
</style>
