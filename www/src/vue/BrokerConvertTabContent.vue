<template>
  <TabContent tabId="broker-convert">
    <div class="files-and-buttons">

      <FileDropArea
        :onFilesDropped="onFilesDropped"
        dropMessage="Drop XLS, XLSX, or PDF Files Here"
      />

      <div class="action-buttons">
        <SplitRunButton :store="fileManagerStore" :onAction="onRunAction" />
      </div>

    </div>

    <div class="content-separator"></div>

    <ErrorBox :store="errorBoxStore" />

    <BrokerConvertOutputArea :store="outputStore" />
  </TabContent>
</template>

<script lang="ts">
import { defineComponent, type PropType } from 'vue';
import TabContent from './TabContent.vue';
import FileDropArea from './FileDropArea.vue';
import SplitRunButton from './SplitRunButton.vue';
import ErrorBox from './ErrorBox.vue';
import BrokerConvertOutputArea from './BrokerConvertOutputArea.vue';
import { getFileManagerStore } from './file_manager_store.js';
import { getBrokerConvertOutputStore } from './broker_convert_output_store.js';
import { getErrorBoxStore } from './error_box_store.js';
import { ErrorBox as ErrorBoxModel } from './error_box_store.js';
import { AcbAppRunMode } from '../common/acb_app_types.js';

export default defineComponent({
   name: 'BrokerConvertTabContent',
   components: { TabContent, FileDropArea, SplitRunButton, ErrorBox, BrokerConvertOutputArea },
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
      const fileManagerStore = getFileManagerStore();
      const outputStore = getBrokerConvertOutputStore();
      const errorBoxStore = getErrorBoxStore(ErrorBoxModel.BROKER_CONVERT_ERRORS_ID);

      return { fileManagerStore, outputStore, errorBoxStore };
   },
});
</script>
