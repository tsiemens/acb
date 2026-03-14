<template>
  <div class="sidebar">
    <SidebarInfo :store="sidebarInfoStore" />
    <SidebarInfoItems />

    <div class="options-section">
      <h3>Options</h3>

      <div class="option-group">
        <div class="checkbox-container">
          <input
            type="checkbox"
            id="printFullValuesCheckbox"
            :checked="appInputStore.printFullValues"
            @change="onPrintFullChange"
          >
          <label for="printFullValuesCheckbox">Render high-precision dollars</label>
        </div>
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent } from 'vue';
import SidebarInfo from './SidebarInfo.vue';
import SidebarInfoItems from './SidebarInfoItems.vue';
import { getSidebarInfoStore } from './sidebar_info_store.js';
import { getAppInputStore } from './app_input_store.js';

export default defineComponent({
   name: 'Sidebar',
   components: { SidebarInfo, SidebarInfoItems },
   setup() {
      const sidebarInfoStore = getSidebarInfoStore();
      const appInputStore = getAppInputStore();

      function onPrintFullChange(event: Event) {
         appInputStore.printFullValues = (event.target as HTMLInputElement).checked;
      }

      return { sidebarInfoStore, appInputStore, onPrintFullChange };
   },
});
</script>
