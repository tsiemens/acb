<template>
  <div class="info-section">
    <h3>Information</h3>

    <div
      v-for="item in items"
      :key="item.dialogId"
      class="clickable-info-item"
      @click="openDialog(item.dialogId)"
    >
      <p>{{ item.label }}</p>
      <img :src="'/images/expand.svg'">
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent } from 'vue';
import { openDialog } from './info_dialog_store.js';

const items = [
   { dialogId: 'appDescriptionDialog', label: 'What is this tool?' },
   { dialogId: 'fileFormatsDialog', label: 'Spreadsheet Format' },
   { dialogId: 'liabilityDialog', label: 'Liability Disclaimer' },
   { dialogId: 'dataPolicyDialog', label: 'Data Policy' },
   { dialogId: 'licenseDialog', label: 'Copyright' },
];

export default defineComponent({
   name: 'SidebarInfoItems',
   setup() {
      return { items, openDialog };
   },
});
</script>
