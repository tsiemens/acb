<template>
  <div v-if="store.securityTables">
    <div
      v-for="symbol in sortedSymbols"
      :key="symbol"
      class="security-wrapper"
      v-show="shouldShowSecurity(symbol)"
    >
      <div class="table-title">{{ symbol }}</div>

      <div v-if="getErrors(symbol).length > 0" class="security-errors">
        <p v-for="(err, i) in getErrors(symbol)" :key="i">{{ err }}</p>
        <p>Information is of parsed state only, and may not be fully correct.</p>
      </div>

      <div class="table-fixed-head">
        <table>
          <thead>
            <tr>
              <th v-for="(col, i) in getTable(symbol).header" :key="i">{{ col }}</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="(row, ri) in getTable(symbol).rows"
              :key="ri"
              :class="[rowActionClass(row), rowYearClass(row), rowHighlightClass(row)]"
            >
              <td v-for="(cell, ci) in row" :key="ci">{{ cell }}</td>
            </tr>
            <tr v-if="getTable(symbol).footer">
              <td v-for="(cell, ci) in getTable(symbol).footer" :key="ci">{{ cell }}</td>
            </tr>
          </tbody>
        </table>
      </div>

      <div v-if="getNotes(symbol).length > 0" class="security-notes">
        <p v-for="(note, i) in getNotes(symbol)" :key="i">{{ note }}</p>
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, computed, type PropType } from 'vue';
import type { OutputStore } from './output_store.js';
import type { RenderTable } from '../acb_wasm_types.js';

const SETTLE_DATE_COL = 2;
const ACTION_COL = 3;

export default defineComponent({
   name: 'SecurityTablesOutput',
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

      function getErrors(symbol: string): string[] {
         return getTable(symbol).errors || [];
      }

      function getNotes(symbol: string): string[] {
         return getTable(symbol).notes || [];
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

      function rowActionClass(row: string[]): string {
         const action = row[ACTION_COL] || '';
         if (/buy/i.test(action)) return 'buy-row';
         if (/sell/i.test(action)) return 'sell-row';
         if (/sprf/i.test(action)) return 'sfla-row';
         if (/split/i.test(action)) return 'split-row';
         return 'other-row';
      }

      function rowYearClass(row: string[]): string {
         const year = row[SETTLE_DATE_COL]?.split('-')[0] || 'unknown';
         return `year-${year}-row`;
      }

      function rowHighlightClass(row: string[]): string {
         if (!props.store.highlightedYear) return '';
         const year = row[SETTLE_DATE_COL]?.split('-')[0];
         return year !== props.store.highlightedYear ? 'year-dimmed' : '';
      }

      return {
         sortedSymbols, getTable, getErrors, getNotes,
         shouldShowSecurity, rowActionClass, rowYearClass, rowHighlightClass,
      };
   },
});
</script>

<style scoped>
.year-dimmed {
   filter: opacity(0.4);
}
</style>
