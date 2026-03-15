<template>
  <SidebarInfoSection>
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
  </SidebarInfoSection>
</template>

<script lang="ts">
import { defineComponent } from 'vue';
import { openDialog } from './info_dialog_store.js';
import SidebarInfoSection from './SidebarInfoSection.vue';

const items = [
   { dialogId: 'appDescriptionDialog', label: 'What is this tool?' },
   { dialogId: 'fileFormatsDialog', label: 'Spreadsheet Format' },
   { dialogId: 'liabilityDialog', label: 'Liability Disclaimer' },
   { dialogId: 'dataPolicyDialog', label: 'Data Policy' },
   { dialogId: 'licenseDialog', label: 'Copyright' },
];

export default defineComponent({
   name: 'SidebarInfoItems',
   components: { SidebarInfoSection },
   setup() {
      return { items, openDialog };
   },
});
</script>

<style scoped>
.clickable-info-item {
  display: flex;
  align-items: flex-start;
  cursor: pointer;
}

.clickable-info-item p {
  flex: 1;
}

.clickable-info-item img {
  height: 12pt;
  margin-left: 8px;
  margin-top: 5px;
}
</style>
