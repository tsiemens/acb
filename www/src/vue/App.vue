<template>
  <div class="container">
    <AppHeader :onAutoRun="onAutoRun" />

    <div class="tab-navigation">
      <div class="tab active">ACB Calculator</div>
    </div>

    <div class="content-area">
      <InfoDialogs :store="infoDialogStore" />

      <Sidebar />
      <MainContent
        :onFilesDropped="onFilesDropped"
        :onRunAction="onRunAction"
      />
    </div>
  </div>

  <FileManagerDrawer
    :store="fileManagerStore"
    :onFilesDropped="onFilesDropped"
  />
</template>

<script lang="ts">
import { defineComponent, type PropType } from 'vue';
import AppHeader from './AppHeader.vue';
import InfoDialogs from './InfoDialogs.vue';
import Sidebar from './Sidebar.vue';
import MainContent from './MainContent.vue';
import FileManagerDrawer from './FileManagerDrawer.vue';
import { getInfoDialogStore } from './info_dialog_store.js';
import { getFileManagerStore } from './file_manager_store.js';
import { AcbAppRunMode } from '../common/acb_app_types.js';

export default defineComponent({
   name: 'App',
   components: { AppHeader, InfoDialogs, Sidebar, MainContent, FileManagerDrawer },
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

      return { infoDialogStore, fileManagerStore };
   },
});
</script>
