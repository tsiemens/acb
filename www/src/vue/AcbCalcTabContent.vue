<template>
  <TabContent tabId="acb-calc">
    <div class="files-and-buttons">

      <FileDropArea
         :onFilesDropped="onFilesDropped"
         dropMessage="Drop CSV Files Here" />

      <AppInputControls :store="appInputStore" />

      <div class="action-buttons">
        <SplitRunButton :tabId="TabId.AcbCalc" :onAction="onRunAction" :functionMode="appInputStore.functionMode" />
      </div>

    </div>

    <div class="content-separator"></div>

    <ErrorBox :store="mainErrorBoxStore" />

    <OutputArea :store="outputStore" />
  </TabContent>
</template>

<script lang="ts">
import { defineComponent, type PropType } from 'vue';
import TabContent from './TabContent.vue';
import FileDropArea from './FileDropArea.vue';
import AppInputControls from './AppInputControls.vue';
import SplitRunButton from './SplitRunButton.vue';
import ErrorBox from './ErrorBox.vue';
import OutputArea from './OutputArea.vue';
import { getAppInputStore } from './app_input_store.js';
import { getOutputStore } from './output_store.js';
import { getErrorBoxStore } from './error_box_store.js';
import { ErrorBox as ErrorBoxModel } from './error_box_store.js';
import { AcbAppRunMode } from '../common/acb_app_types.js';
import { TabId } from './tab_store.js';

export default defineComponent({
   name: 'AcbCalcTabContent',
   components: { TabContent, FileDropArea, AppInputControls, SplitRunButton, ErrorBox, OutputArea },
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
      const outputStore = getOutputStore();
      const mainErrorBoxStore = getErrorBoxStore(ErrorBoxModel.MAIN_ERRORS_ID);

      return { TabId, appInputStore, outputStore, mainErrorBoxStore };
   },
});
</script>
