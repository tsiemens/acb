<template>
  <div class="sidebar">
    <SidebarBasicInfo :store="sidebarInfoStore" />
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
import SidebarBasicInfo from './SidebarBasicInfo.vue';
import SidebarInfoItems from './SidebarInfoItems.vue';
import { getSidebarInfoStore } from './sidebar_info_store.js';
import { getAppInputStore } from './app_input_store.js';

export default defineComponent({
   name: 'Sidebar',
   components: { SidebarBasicInfo, SidebarInfoItems },
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

<style scoped>
.sidebar {
  flex: 0 0 300px;
  background-color: white;
  border-radius: var(--border-radius);
  padding: 20px;
  box-shadow: 0 2px 5px rgba(0,0,0,0.1);
}

.options-section {
  margin-bottom: 25px;
}

.option-group {
  margin-bottom: 15px;
}

.option-label {
  display: block;
  margin-bottom: 5px;
  font-weight: 500;
}

.option-input {
  width: 100%;
  padding: 8px;
  border: 1px solid #ddd;
  border-radius: 4px;
}
</style>
