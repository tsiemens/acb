<template>
  <div class="main-content">
    <div class="main-content-files-and-buttons">

      <FileDropArea :onFilesDropped="onFilesDropped" />

      <AppInputControls :store="appInputStore" />

      <div class="action-buttons">
        <SplitRunButton :store="fileManagerStore" :onAction="onRunAction" />
      </div>

    </div>

    <div class="separator"></div>

    <ErrorBox :store="mainErrorBoxStore" />

    <OutputArea :store="outputStore" />
  </div>
</template>

<script lang="ts">
import { defineComponent, type PropType } from 'vue';
import FileDropArea from './FileDropArea.vue';
import AppInputControls from './AppInputControls.vue';
import SplitRunButton from './SplitRunButton.vue';
import ErrorBox from './ErrorBox.vue';
import OutputArea from './OutputArea.vue';
import { getAppInputStore } from './app_input_store.js';
import { getFileManagerStore } from './file_manager_store.js';
import { getOutputStore } from './output_store.js';
import { getErrorBoxStore } from './error_box_store.js';
import { ErrorBox as ErrorBoxModel } from './error_box_store.js';
import { AcbAppRunMode } from '../common/acb_app_types.js';

export default defineComponent({
   name: 'MainContent',
   components: { FileDropArea, AppInputControls, SplitRunButton, ErrorBox, OutputArea },
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
      const appInputStore = getAppInputStore();
      const fileManagerStore = getFileManagerStore();
      const outputStore = getOutputStore();
      const mainErrorBoxStore = getErrorBoxStore(ErrorBoxModel.MAIN_ERRORS_ID);

      return { appInputStore, fileManagerStore, outputStore, mainErrorBoxStore };
   },
});
</script>

<style scoped>
.main-content {
  flex: 1;
  background-color: white;
  border-radius: var(--border-radius);
  padding: 20px;
  box-shadow: 0 2px 5px rgba(0,0,0,0.1);
}

.separator {
  height: 2px;
  background: linear-gradient(90deg, transparent, #e2e8f0, transparent);
  margin: 30px 0;
}

.action-buttons {
  display: flex;
  gap: 10px;
  margin: 20px 0;
}
</style>
