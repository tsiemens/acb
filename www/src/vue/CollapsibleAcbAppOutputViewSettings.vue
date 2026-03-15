<template>
  <CollapsibleRegion>
    <template #header>
      <img :src="'/images/eye.svg'" style="height: 24px; filter: invert(0.5)">
      View Settings
    </template>

    <h3>Output View Settings</h3>

    <div class="form-group">
      <label for="yearHighlightSelect">Highlight Year:</label>
      <select
        id="yearHighlightSelect"
        :value="store.highlightedYear ?? 'None'"
        @change="onYearChange"
      >
        <option value="None">None</option>
        <option
          v-for="year in availableYears"
          :key="year"
          :value="year.toString()"
        >{{ year }}</option>
      </select>
    </div>

    <div class="form-group" style="display: inline-flex;">
      <input
        type="checkbox"
        id="hideNoActivityCheckbox"
        :checked="store.hideInactiveSecurities"
        @change="onHideChange"
        style="margin-right: 8px; width: fit-content;"
      />
      <label for="hideNoActivityCheckbox">Hide securities with no activity in selected year</label>
    </div>
  </CollapsibleRegion>
</template>

<script lang="ts">
import { defineComponent, computed, type PropType } from 'vue';
import type { OutputStore } from './output_store.js';
import CollapsibleRegion from './CollapsibleRegion.vue';

export default defineComponent({
   name: 'CollapsibleAcbAppOutputViewSettings',
   components: { CollapsibleRegion },
   props: {
      store: {
         type: Object as PropType<OutputStore>,
         required: true,
      },
   },
   setup(props) {
      const availableYears = computed(() => {
         const years = new Set<number>();
         if (props.store.securityTables) {
            for (const table of props.store.securityTables.values()) {
               for (const row of table.rows) {
                  const year = parseInt(row[2]?.split('-')[0], 10);
                  if (!isNaN(year)) years.add(year);
               }
            }
         }
         return Array.from(years).sort((a, b) => b - a);
      });

      function onYearChange(event: Event) {
         const value = (event.target as HTMLSelectElement).value;
         props.store.highlightedYear = value === 'None' ? null : value;
      }

      function onHideChange(event: Event) {
         props.store.hideInactiveSecurities = (event.target as HTMLInputElement).checked;
      }

      return { availableYears, onYearChange, onHideChange };
   },
});
</script>
