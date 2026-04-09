<template>
  <div v-if="store.securityTables">
    <DataTable
      v-for="symbol in sortedSymbols"
      :key="symbol"
      v-show="shouldShowSecurity(symbol)"
      :table="getTable(symbol)"
      :title="symbol"
      :titleBadge="isNewSecurity(symbol) ? 'NEW' : ''"
      :titleBadgeHelpText="isNewSecurity(symbol) ? getNewSecurityDescription() : ''"
      :rowClassFn="rowClassFn"
      :cellClassFn="cellClassFn"
      :cellTagClassFn="cellTagClassFn"
      :cellHtmlFn="cellHtmlFn"
      errorSuffix="Information is of parsed state only, and may not be fully correct."
    />

  </div>
</template>

<script lang="ts">
import { defineComponent, computed, type PropType } from 'vue';
import { InactiveFilterMode, type OutputStore } from './output_store.js';
import type { RenderTable } from '../acb_wasm_types.js';
import DataTable from './DataTable.vue';

const TRADE_DATE_COL = 1;
const SETTLE_DATE_COL = 2;
const ACTION_COL = 3;
const AMOUNT_COL = 4;
const AMT_PER_SHARE_COL = 6;
const COMMISSION_COL = 8;
const CAP_GAIN_COL = 9;
const MEMO_COL = 15;

const BREAK_BEFORE_PAREN_COLS = new Set([AMOUNT_COL, AMT_PER_SHARE_COL, COMMISSION_COL]);

// Canadian personal tax deadline is April 30.
// Before the deadline, the "current" tax year is last year.
// After the deadline, the "current" tax year is this year.
const TAX_DEADLINE_MONTH = 4; // April (1-indexed)
const TAX_DEADLINE_DAY = 30;

function getCurrentTaxYear(): number {
   const now = new Date();
   const year = now.getFullYear();
   const month = now.getMonth() + 1;
   const day = now.getDate();
   if (month < TAX_DEADLINE_MONTH || (month === TAX_DEADLINE_MONTH && day <= TAX_DEADLINE_DAY)) {
      return year - 1;
   }
   return year;
}

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

      const currentTaxYear = getCurrentTaxYear();

      function getYearsForSymbol(symbol: string): Set<string> {
         const years = new Set<string>();
         for (const row of getTable(symbol).rows) {
            const year = row[SETTLE_DATE_COL]?.split('-')[0];
            if (year) years.add(year);
         }
         return years;
      }

      function isNewSecurity(symbol: string): boolean {
         const years = getYearsForSymbol(symbol);
         if (years.size === 0) return false;
         for (const year of years) {
            const y = parseInt(year, 10);
            if (y < currentTaxYear) return false;
         }
         return true;
      }

      function getNewSecurityDescription(): string {
         return `This security has no transactions before the ${currentTaxYear} tax year.`;
      }

      function shouldShowSecurity(symbol: string): boolean {
         if (!props.store.highlightedYear ||
             props.store.inactiveFilterMode === InactiveFilterMode.DimRows) {
            return true;
         }
         const table = getTable(symbol);
         const hasError = table.errors && table.errors.length > 0;
         if (hasError) return true;
         return getYearsForSymbol(symbol).has(props.store.highlightedYear);
      }

      function gainLossType(row: string[]): 'gain' | 'loss' | null {
         const capGain = row[CAP_GAIN_COL] || '';
         if (!capGain.includes('$')) return null;
         if (capGain.includes('-') || /sfl/i.test(capGain)) return 'loss';
         return 'gain';
      }

      function rowClassFn(row: string[]): string[] {
         const year = row[SETTLE_DATE_COL]?.split('-')[0];
         const yearClass = `year-${year || 'unknown'}-row`;

         const isNonMatching = props.store.highlightedYear && year !== props.store.highlightedYear;
         const filterMode = props.store.inactiveFilterMode;
         const dimmed = isNonMatching ? (
            filterMode === InactiveFilterMode.HideRows ? 'year-hidden' : 'year-dimmed'
         ) : '';

         const gl = gainLossType(row);
         const glClass = gl === 'gain' ? 'gain-row' : gl === 'loss' ? 'loss-row' : '';

         return [yearClass, dimmed, glClass].filter(Boolean);
      }

      function cellClassFn(row: string[], colIndex: number): string | string[] | null {
         const classes: string[] = [];
         if (colIndex === MEMO_COL) {
            classes.push('memo-cell');
         } else if (colIndex !== ACTION_COL) {
            classes.push('nowrap-cell');
         }
         if (colIndex === CAP_GAIN_COL) {
            const gl = gainLossType(row);
            if (gl === 'gain') classes.push('cap-gain-cell');
            else if (gl === 'loss') classes.push('cap-loss-cell');
         }
         return classes.length > 0 ? classes : null;
      }

      function cellTagClassFn(row: string[], colIndex: number): string | null {
         if (colIndex !== ACTION_COL) return null;
         const action = (row[ACTION_COL] || '').toLowerCase();
         if (/buy/i.test(action)) return 'tag-buy';
         if (/sell/i.test(action)) return 'tag-sell';
         if (/sprf|sfla/i.test(action)) return 'tag-sfla';
         if (/split/i.test(action)) return 'tag-split';
         if (/ret\.|reinv\.|cap\. gains div/i.test(action)) return 'tag-dist';
         return 'tag-other';
      }

      function escapeHtml(s: string): string {
         return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
      }

      function cellHtmlFn(cell: string, colIndex: number): string | null {
         if (BREAK_BEFORE_PAREN_COLS.has(colIndex)) {
            const parenIdx = cell.indexOf('(');
            if (parenIdx > 0) {
               return escapeHtml(cell.slice(0, parenIdx)) + '<br>' + escapeHtml(cell.slice(parenIdx));
            }
         }
         if (colIndex === CAP_GAIN_COL) {
            const starIdx = cell.indexOf('*');
            if (starIdx > 0) {
               return escapeHtml(cell.slice(0, starIdx)) + '<br>' + escapeHtml(cell.slice(starIdx));
            }
         }
         return null;
      }

      return { sortedSymbols, getTable, shouldShowSecurity, isNewSecurity, getNewSecurityDescription, rowClassFn, cellClassFn, cellTagClassFn, cellHtmlFn };
   },
});
</script>

<style scoped>
:deep(.year-dimmed) {
  filter: opacity(0.4);
}

:deep(.year-hidden) {
  display: none;
}

:deep(.nowrap-cell) {
  white-space: nowrap;
}

:deep(.memo-cell) {
  min-width: 200px;
}

:deep(.gain-row) td {
  background: #f0faf0;
}

:deep(.loss-row) td {
  background: #fef2f2;
}

:deep(.cap-gain-cell) {
  font-weight: bold;
  color: #16a34a;
}

:deep(.cap-loss-cell) {
  font-weight: bold;
  color: #dc2626;
}

/* TX action tags */
:deep(.tag-buy) {
  background: #dbeafe;
  color: #1e40af;
}

:deep(.tag-sell) {
  background: #fef3c7;
  color: #92400e;
}

:deep(.tag-sfla) {
  background: #fce7f3;
  color: #9d174d;
}

:deep(.tag-split) {
  background: #d1fae5;
  color: #065f46;
}

:deep(.tag-dist) {
  background: #ede9fe;
  color: #5b21b6;
}

:deep(.tag-other) {
  background: #f3f4f6;
  color: #374151;
}

</style>
