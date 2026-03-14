<template>
  <div class="form-group">
    <label for="acbFeatureModeSelect">Mode:</label>
    <select
      id="acbFeatureModeSelect"
      style="width: fit-content;"
      v-model="store.functionMode"
      @change="onModeChange"
    >
      <option value="calculate" selected>Calculate Gains</option>
      <option value="tx_summary">Summarize Transactions</option>
      <option value="tally_shares">Tally Share Balances</option>
    </select>

    <label
      for="acbSummaryDatePicker"
      v-show="dateVisible"
      style="margin-left: 10px;"
    >Latest Date:</label>
    <input
      type="date"
      id="acbSummaryDatePicker"
      v-show="dateVisible"
      v-model="store.summaryDateStr"
      @change="onDateChange"
      style="width: fit-content; margin-left: 10px;"
    >
  </div>
</template>

<script lang="ts">
import { defineComponent, computed, type PropType } from 'vue';
import {
   type AppInputStore,
   updateDateForMode,
   shouldShowDatePicker,
} from './app_input_store.js';

export default defineComponent({
   name: 'AppInputControls',
   props: {
      store: {
         type: Object as PropType<AppInputStore>,
         required: true,
      },
   },
   // setup() returns an object whose properties (refs, computed refs,
   // and functions) are exposed to the component's template. Vue unwraps
   // refs automatically in the template, so `dateVisible` can be used
   // directly rather than as `dateVisible.value`.
   setup(props) {
      const dateVisible = computed(() => shouldShowDatePicker(props.store));

      function onModeChange() {
         updateDateForMode(props.store);
      }

      function onDateChange() {
         if (props.store.summaryDateStr) {
            props.store.lastPickedDates.set(
               props.store.functionMode,
               props.store.summaryDateStr,
            );
         }
      }

      return { dateVisible, onModeChange, onDateChange };
   },
});
</script>
