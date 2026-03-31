<template>
  <SidebarInfoSection>
    <h3>Information</h3>

    <div
      v-for="item in generalItems"
      :key="item.dialogId"
      class="clickable-info-item"
      @click="openDialog(item.dialogId)"
    >
      <p>{{ item.label }}</p>
      <img :src="'/images/expand.svg'">
    </div>

    <div class="collapsible-sub-section">
      <button class="sub-section-toggle" @click="brokerageExpanded = !brokerageExpanded">
        <img class="toggle-icon" :class="{ expanded: brokerageExpanded }" :src="'/images/chevron-right.svg'" />
        <span>Brokerage Support</span>
      </button>
      <div class="sub-section-content" :class="brokerageExpanded ? 'expanded' : 'collapsed'">
        <div
          v-for="item in brokerageItems"
          :key="item.dialogId"
          class="clickable-info-item sub-item"
          @click="openDialog(item.dialogId)"
        >
          <p>{{ item.label }}</p>
          <img :src="'/images/expand.svg'">
        </div>
      </div>
    </div>

    <div class="collapsible-sub-section">
      <button class="sub-section-toggle" @click="moreExpanded = !moreExpanded">
        <img class="toggle-icon" :class="{ expanded: moreExpanded }" :src="'/images/chevron-right.svg'" />
        <span>More</span>
      </button>
      <div class="sub-section-content" :class="moreExpanded ? 'expanded' : 'collapsed'">
        <div
          v-for="item in moreItems"
          :key="item.dialogId"
          class="clickable-info-item sub-item"
          @click="openDialog(item.dialogId)"
        >
          <p>{{ item.label }}</p>
          <img :src="'/images/expand.svg'">
        </div>
      </div>
    </div>
  </SidebarInfoSection>
</template>

<script lang="ts">
import { defineComponent, ref } from 'vue';
import { openDialog } from './info_dialog_store.js';
import SidebarInfoSection from './SidebarInfoSection.vue';

const generalItems = [
   { dialogId: 'appDescriptionDialog', label: 'What is this tool?' },
   { dialogId: 'fileFormatsDialog', label: 'Spreadsheet Format' },
];

const brokerageItems = [
   { dialogId: 'etradeInstructionsDialog', label: 'E*TRADE' },
   { dialogId: 'questradeInstructionsDialog', label: 'Questrade' },
   { dialogId: 'rbcDiInstructionsDialog', label: 'RBC Direct Investing' },
];

const moreItems = [
   { dialogId: 'liabilityDialog', label: 'Liability Disclaimer' },
   { dialogId: 'dataPolicyDialog', label: 'Data Policy' },
   { dialogId: 'licenseDialog', label: 'Copyright' },
];

export default defineComponent({
   name: 'SidebarInfoItems',
   components: { SidebarInfoSection },
   setup() {
      const brokerageExpanded = ref(false);
      const moreExpanded = ref(false);
      return { generalItems, brokerageItems, moreItems, openDialog, brokerageExpanded, moreExpanded };
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

.clickable-info-item.sub-item {
  padding-left: 22px;
}

.collapsible-sub-section {
  margin-top: 4px;
}

.sub-section-toggle {
  background: none;
  border: none;
  cursor: pointer;
  padding: 4px 0;
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 16px;
  font-weight: 500;
  color: var(--primary-color);
}

.sub-section-toggle:hover {
  opacity: 0.8;
}

.toggle-icon {
  width: 12px;
  height: 12px;
  filter: invert(0.4);
  transition: transform 0.2s ease;
}

.toggle-icon.expanded {
  transform: rotate(90deg);
}

.sub-section-content {
  overflow: hidden;
  transition: max-height 0.3s ease, opacity 0.3s ease;
}

.sub-section-content.collapsed {
  max-height: 0;
  opacity: 0;
}

.sub-section-content.expanded {
  max-height: 200px;
  opacity: 1;
}
</style>
