<template>
  <div v-if="hasOutput" class="view-settings-bar">
    <button
      class="visibility-btn"
      :class="visibilityBtnClass"
      :title="visibilityTitle"
      @click="cycleVisibility"
    >
      <!-- DimRows: open eye -->
      <svg v-if="store.inactiveFilterMode === FilterMode.DimRows"
        width="20" height="20" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
        <path fill-rule="evenodd" clip-rule="evenodd" d="M6.3 15.58C4.78 14.27 3.69 12.77 3.18 12C3.69 11.23 4.78 9.73 6.3 8.42C7.87 7.07 9.82 6 12 6C14.18 6 16.13 7.07 17.7 8.42C19.22 9.73 20.31 11.23 20.82 12C20.31 12.77 19.22 14.27 17.7 15.58C16.13 16.93 14.18 18 12 18C9.82 18 7.87 16.93 6.3 15.58ZM12 4C9.15 4 6.76 5.39 5 6.91C3.23 8.42 2.01 10.14 1.46 10.97C1.05 11.6 1.05 12.4 1.46 13.03C2.01 13.86 3.23 15.58 5 17.09C6.76 18.61 9.15 20 12 20C14.85 20 17.24 18.61 19 17.09C20.77 15.58 21.99 13.86 22.54 13.03C22.95 12.4 22.95 11.6 22.54 10.97C21.99 10.14 20.77 8.42 19 6.91C17.24 5.39 14.85 4 12 4ZM10 12C10 10.9 10.9 10 12 10C13.1 10 14 10.9 14 12C14 13.1 13.1 14 12 14C10.9 14 10 13.1 10 12ZM12 8C9.79 8 8 9.79 8 12C8 14.21 9.79 16 12 16C14.21 16 16 14.21 16 12C16 9.79 14.21 8 12 8Z" fill="currentColor"/>
      </svg>
      <!-- HideSecurities: half-closed eye -->
      <svg v-else-if="store.inactiveFilterMode === FilterMode.HideSecurities"
        width="20" height="20" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
        <path d="M12 17C9.82 17 7.87 15.93 6.3 14.58C5.14 13.56 4.23 12.42 3.64 11.5L2 14" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        <path d="M12 17C14.18 17 16.13 15.93 17.7 14.58C18.86 13.56 19.77 12.42 20.36 11.5L22 14" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        <path d="M7.5 15.5L6 18" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        <path d="M16.5 15.5L18 18" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        <path d="M12 17V20" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
      </svg>
      <!-- HideRows: slashed eye -->
      <svg v-else
        width="20" height="20" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
        <path d="M2 2L22 22" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        <path d="M6.71 6.71C3.94 8.56 2.28 11.22 1.46 12.97C1.32 13.28 1.32 13.63 1.46 13.94C2.4 16.01 5.83 21 12 21C14.03 21 15.8 20.37 17.29 19.44M10 5.06C10.65 5.02 11.32 5 12 5C18.17 5 21.6 9.99 22.54 12.06C22.68 12.37 22.68 12.72 22.54 13.03C22.16 13.86 21.42 15.15 20.31 16.39" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        <path d="M14 14.24C13.44 14.72 12.74 15 12 15C10.34 15 9 13.66 9 12C9 11.26 9.28 10.56 9.76 10" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
      </svg>
    </button>

    <div v-if="availableAffiliates.length > 1" class="pill-group">
      <button
        class="year-pill"
        :class="{ active: store.selectedAffiliate === null }"
        @click="store.selectedAffiliate = null"
      >All Affiliates</button>
      <button
        v-for="aff in availableAffiliates"
        :key="aff"
        class="year-pill"
        :class="{ active: store.selectedAffiliate === aff }"
        @click="store.selectedAffiliate = aff"
      >{{ aff }}</button>
    </div>

    <div v-if="availableAffiliates.length > 1" class="pill-separator"></div>

    <div class="pill-group">
      <button
        class="year-pill"
        :class="{ active: store.highlightedYear === null }"
        @click="store.highlightedYear = null"
      >All Years</button>
      <button
        v-for="year in availableYears"
        :key="year"
        class="year-pill"
        :class="{ active: store.highlightedYear === year.toString() }"
        @click="store.highlightedYear = year.toString()"
      >{{ year }}</button>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, computed, type PropType } from 'vue';
import { InactiveFilterMode, AcbOutputViewMode, affiliateBaseName, type OutputStore } from './output_store.js';
import { AFFILIATE_COL, SETTLE_DATE_COL } from '../render_table_utils.js';

const FILTER_CYCLE: InactiveFilterMode[] = [
   InactiveFilterMode.DimRows,
   InactiveFilterMode.HideSecurities,
   InactiveFilterMode.HideRows,
];

export default defineComponent({
   name: 'AcbAppOutputViewSettings',
   props: {
      store: {
         type: Object as PropType<OutputStore>,
         required: true,
      },
   },
   setup(props) {
      const FilterMode = InactiveFilterMode;

      const hasOutput = computed(() => {
         const s = props.store;
         return (s.securityTables && s.securityTables.size > 0) ||
            s.summaryTable !== null ||
            s.aggregateTable !== null ||
            (s.textOutput !== '');
      });

      const availableAffiliates = computed(() => {
         const baseNames = new Set<string>();
         const viewMode = props.store.activeViewMode;
         if (viewMode === AcbOutputViewMode.SecurityTables && props.store.securityTables) {
            for (const table of props.store.securityTables.values()) {
               for (const row of table.rows) {
                  const aff = row[AFFILIATE_COL];
                  if (aff) baseNames.add(affiliateBaseName(aff));
               }
            }
         }
         // summaryTable is shared by TxSummary and TallyShares modes; find the
         // affiliate column by name since the column layout differs between the two.
         if (viewMode === AcbOutputViewMode.Summary && props.store.summaryTable) {
            const affIdx = props.store.summaryTable.header
               .findIndex(h => h.toLowerCase() === 'affiliate');
            if (affIdx >= 0) {
               for (const row of props.store.summaryTable.rows) {
                  const aff = row[affIdx];
                  if (aff) baseNames.add(affiliateBaseName(aff));
               }
            }
         }
         return Array.from(baseNames).sort((a, b) => {
            const aIsDefault = a.toLowerCase() === 'default';
            const bIsDefault = b.toLowerCase() === 'default';
            if (aIsDefault && !bIsDefault) return -1;
            if (!aIsDefault && bIsDefault) return 1;
            return a.localeCompare(b);
         });
      });

      const availableYears = computed(() => {
         const years = new Set<number>();
         const viewMode = props.store.activeViewMode;
         if (viewMode === AcbOutputViewMode.SecurityTables && props.store.securityTables) {
            for (const table of props.store.securityTables.values()) {
               for (const row of table.rows) {
                  const year = parseInt(row[SETTLE_DATE_COL]?.split('-')[0], 10);
                  if (!isNaN(year)) years.add(year);
               }
            }
         }
         if (viewMode === AcbOutputViewMode.Summary && props.store.summaryTable) {
            // summaryTable is shared by TxSummary and TallyShares modes.
            // TallyShares produces a ["security", ["affiliate",] "shares"] table with
            // no date column, so we must find the column by name rather than assuming
            // a fixed index.
            const settleDateIdx = props.store.summaryTable.header
               .findIndex(h => h.toLowerCase() === 'settlement date');
            if (settleDateIdx >= 0) {
               for (const row of props.store.summaryTable.rows) {
                  const year = parseInt(row[settleDateIdx]?.split('-')[0], 10);
                  if (!isNaN(year)) years.add(year);
               }
            }
         }
         return Array.from(years).sort((a, b) => b - a);
      });

      const visibilityTitle = computed(() => {
         const m = props.store.inactiveFilterMode;
         if (m === FilterMode.DimRows) {
            return 'Dimming non-matching rows — click to also hide empty securities';
         } else if (m === FilterMode.HideSecurities) {
            return 'Hiding empty securities — click to also hide non-matching rows';
         }
         return 'Hiding non-matching rows and empty securities — click to show all';
      });

      const visibilityBtnClass = computed(() => {
         const m = props.store.inactiveFilterMode;
         return {
            'hide-securities': m === FilterMode.HideSecurities,
            'hide-rows': m === FilterMode.HideRows,
         };
      });

      function cycleVisibility() {
         const current = FILTER_CYCLE.indexOf(props.store.inactiveFilterMode);
         props.store.inactiveFilterMode =
            FILTER_CYCLE[(current + 1) % FILTER_CYCLE.length];
      }

      return { FilterMode, hasOutput, availableAffiliates, availableYears, visibilityTitle, visibilityBtnClass, cycleVisibility };
   },
});
</script>

<style scoped>
.view-settings-bar {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 12px;
  flex-wrap: wrap;
}

.pill-group {
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
}

.pill-separator {
  width: 1px;
  height: 20px;
  background: #ccc;
}

.year-pill {
  padding: 3px 10px;
  border: 1px solid #ccc;
  border-radius: 999px;
  background: #f0f0f0;
  cursor: pointer;
  font-size: 13px;
  line-height: 1.4;
  transition: background-color 0.15s, color 0.15s;
}

.year-pill:hover {
  background: #e0e0e0;
}

.year-pill.active {
  background: var(--primary-color, #4a90d9);
  color: white;
  border-color: var(--primary-color, #4a90d9);
}

.year-pill.active:hover {
  background: var(--primary-color, #4a90d9);
}

.visibility-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  padding: 0;
  border: 1px solid #ccc;
  border-radius: 6px;
  background: #f0f0f0;
  cursor: pointer;
  color: #555;
  transition: background-color 0.15s, color 0.15s;
}

.visibility-btn:hover {
  background: #e0e0e0;
}

.visibility-btn.hide-securities {
  background: #e8d44d;
  border-color: #d4b82e;
  color: #333;
}

.visibility-btn.hide-rows {
  background: #d9534f;
  border-color: #c9302c;
  color: white;
}

</style>
