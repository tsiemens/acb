<template>
  <div class="container">
    <AppHeader :onAutoRun="onAutoRun" />

    <div class="tab-navigation">
      <div
        v-for="tab in tabs" :key="tab.id"
        class="tab"
        :class="{ active: tabStore.activeTab === tab.id }"
        @click="tabStore.activeTab = tab.id"
      >{{ tab.label }}</div>
    </div>

    <div class="content-area">
      <InfoDialogs :store="infoDialogStore" />

      <Sidebar />
      <AcbCalcTabContent
        v-if="tabStore.activeTab === TabId.AcbCalc"
        :onFilesDropped="onFilesDropped"
        :onRunAction="onRunAction"
      />
      <BrokerConvertTabContent
        v-if="tabStore.activeTab === TabId.BrokerConvert"
        :onFilesDropped="onFilesDropped"
        :onRunAction="onRunAction"
      />
    </div>
  </div>

  <FileManagerDrawer
    :store="fileManagerStore"
    :onFilesDropped="onFilesDropped"
    :onDownloadSelected="downloadSelectedFiles"
  />
</template>

<script lang="ts">
import { defineComponent, type PropType } from 'vue';
import AppHeader from './AppHeader.vue';
import InfoDialogs from './InfoDialogs.vue';
import Sidebar from './Sidebar.vue';
import AcbCalcTabContent from './AcbCalcTabContent.vue';
import BrokerConvertTabContent from './BrokerConvertTabContent.vue';
import FileManagerDrawer from './FileManagerDrawer.vue';
import { getInfoDialogStore } from './info_dialog_store.js';
import { getFileManagerStore } from './file_manager_store.js';
import { getTabStore, tabs, TabId } from './tab_store.js';
import { AcbAppRunMode } from '../common/acb_app_types.js';
import { downloadSelectedFiles } from '../download_utils.js';

export default defineComponent({
   name: 'App',
   components: { AppHeader, InfoDialogs, Sidebar, AcbCalcTabContent, BrokerConvertTabContent, FileManagerDrawer },
   props: {
      onFilesDropped: {
         type: Function as PropType<(fileList: FileList) => void>,
         required: true,
      },
      onRunAction: {
         type: Function as PropType<(mode: AcbAppRunMode) => void>,
         required: true,
      },
      onAutoRun: {
         type: Function as PropType<() => void>,
         required: false,
      },
   },
   setup() {
      const infoDialogStore = getInfoDialogStore();
      const fileManagerStore = getFileManagerStore();
      const tabStore = getTabStore();

      return { tabs, TabId, tabStore, infoDialogStore, fileManagerStore, downloadSelectedFiles };
   },
});
</script>

<style scoped>
.container {
  /* max-width: 1200px; */
  margin: 0 auto;
  padding: 20px;
}

.tab-navigation {
  display: flex;
  border-bottom: 1px solid #ddd;
  margin-bottom: 20px;
}

.tab {
  padding: 10px 20px;
  cursor: pointer;
  border: 1px solid transparent;
  border-bottom: none;
  border-top-left-radius: var(--border-radius);
  border-top-right-radius: var(--border-radius);
  margin-right: 5px;
  font-weight: 500;
}

.tab.active {
  background-color: white;
  border-color: #ddd;
  margin-bottom: -1px;
}

.tab:hover:not(.active) {
  background-color: #f0f0f0;
}

.content-area {
  display: flex;
  gap: 20px;
  margin-bottom: 20px;
}
</style>
