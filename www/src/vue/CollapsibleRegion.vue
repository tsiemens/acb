<template>
  <div class="collapsible-wrapper">
    <button
      class="collapsible-content-btn"
      :class="{ active: expanded, expanded: expanded }"
      @click="toggle"
    >
      <img class="toggle-icon" :src="'/images/chevron-right.svg'"/>
      <img :src="'/images/eye.svg'" style="height: 24px; filter: invert(0.5)">
      View Settings
    </button>
    <div
      class="collapsible-content"
      :class="expanded ? 'expanded' : 'collapsed'"
    >
      <div class="collapsible-content-inner">
        <div class="collapsible-content-padded-inner">
          <h3>Output View Settings</h3>

          <div class="form-group">
            <label for="yearHighlightSelect">Highlight Year:</label>
            <select id="yearHighlightSelect">
              <option value="None" selected>None</option>
            </select>
          </div>

          <div class="form-group" style="display: inline-flex;">
            <input
              type="checkbox"
              id="hideNoActivityCheckbox"
              style="margin-right: 8px; width: fit-content;"
            />
            <label for="hideNoActivityCheckbox">Hide securities with no activity in selected year</label>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref } from 'vue';

export default defineComponent({
   name: 'CollapsibleRegion',
   setup() {
      const expanded = ref(false);

      function toggle() {
         expanded.value = !expanded.value;
      }

      return { expanded, toggle };
   },
});
</script>
