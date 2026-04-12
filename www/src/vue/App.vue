<template>
  <div class="container">
    <AppHeader />

    <div class="tab-navigation">
      <div
        v-for="tab in tabs" :key="tab.id"
        class="tab"
        :class="{
          active: tabStore.activeTab === tab.id,
          glowing: tabStore.glowingTabs.has(tab.id),
        }"
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
      <ConfigurationTabContent
        v-if="tabStore.activeTab === TabId.Configuration"
      />
    </div>

    <footer class="app-footer">
      <span>&copy; {{ copyrightYears }} Trevor Siemens</span>
      <a href="https://github.com/tsiemens/acb" class="github-link" title="GitHub" target="_blank" rel="noopener">
        <svg viewBox="0 0 16 16" width="16" height="16" fill="currentColor">
          <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38
            0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52
            -.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2
            -3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82
            .64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08
            2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01
            1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/>
        </svg>
      </a>
    </footer>
  </div>

  <OptionDialog />
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
import ConfigurationTabContent from './ConfigurationTabContent.vue';
import FileManagerDrawer from './FileManagerDrawer.vue';
import OptionDialog from './OptionDialog.vue';
import { getInfoDialogStore } from './info_dialog_store.js';
import { getFileManagerStore } from './file_manager_store.js';
import { getTabStore, tabs, TabId } from './tab_store.js';
import { AcbAppRunMode } from '../common/acb_app_types.js';
import { downloadSelectedFiles } from '../download_utils.js';
import { copyrightYears } from './copyright.js';

export default defineComponent({
   name: 'App',
   components: { AppHeader, InfoDialogs, Sidebar, AcbCalcTabContent, BrokerConvertTabContent, ConfigurationTabContent, FileManagerDrawer, OptionDialog },
   props: {
      onFilesDropped: {
         type: Function as PropType<(fileList: FileList) => void>,
         required: true,
      },
      onRunAction: {
         type: Function as PropType<(mode: AcbAppRunMode) => void>,
         required: true,
      },
   },
   setup() {
      const infoDialogStore = getInfoDialogStore();
      const fileManagerStore = getFileManagerStore();
      const tabStore = getTabStore();

      return { tabs, TabId, tabStore, infoDialogStore, fileManagerStore, downloadSelectedFiles, copyrightYears: copyrightYears() };
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

.app-footer {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 8px 0 48px;
  font-size: 13px;
  color: var(--secondary-color);
}

.github-link {
  color: var(--secondary-color);
  display: flex;
  align-items: center;
  transition: color 0.2s;
}

.github-link:hover {
  color: var(--dark-color);
}

/* Glow pulse for background tabs whose state changed */
.tab.glowing {
  animation: tab-pulse 1.5s ease-in-out infinite;
}
@keyframes tab-pulse {
  0%, 100% { box-shadow: 0 0 0 0 rgba(74, 111, 165, 0); }
  50%      { box-shadow: 0 0 8px 2px rgba(74, 111, 165, 0.5); }
}
</style>
