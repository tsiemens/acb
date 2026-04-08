<template>
  <TabContent tabId="broker-convert">
    <div class="files-and-buttons">

      <FileDropArea
        :onFilesDropped="onFilesDropped"
        dropMessage="Drop XLSX, PDF or CSV Files Here"
      />

      <BrokerConvertConfigPanel />

      <div class="action-buttons">
        <SplitRunButton :tabId="TabId.BrokerConvert" :onAction="onRunAction" />
      </div>

    </div>

    <div class="content-separator"></div>

    <ErrorBox :store="errorBoxStore" />
    <ErrorBox :store="warningBoxStore" severity="warning" />

    <BrokerConvertOutputArea :store="outputStore" />
  </TabContent>
</template>

<script lang="ts">
import { defineComponent, type PropType } from 'vue';
import TabContent from './TabContent.vue';
import FileDropArea from './FileDropArea.vue';
import SplitRunButton from './SplitRunButton.vue';
import BrokerConvertConfigPanel from './BrokerConvertConfigPanel.vue';
import ErrorBox from './ErrorBox.vue';
import BrokerConvertOutputArea from './BrokerConvertOutputArea.vue';
import { getBrokerConvertOutputStore } from './broker_convert_output_store.js';
import { getErrorBoxStore } from './error_box_store.js';
import { ErrorBox as ErrorBoxModel } from './error_box_store.js';
import { AcbAppRunMode } from '../common/acb_app_types.js';
import { TabId } from './tab_store.js';

export default defineComponent({
   name: 'BrokerConvertTabContent',
   components: { TabContent, FileDropArea, SplitRunButton, BrokerConvertConfigPanel, ErrorBox, BrokerConvertOutputArea },
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
      const outputStore = getBrokerConvertOutputStore();
      const errorBoxStore = getErrorBoxStore(ErrorBoxModel.BROKER_CONVERT_ERRORS_ID);
      const warningBoxStore = getErrorBoxStore(ErrorBoxModel.BROKER_CONVERT_WARNINGS_ID);

      return { TabId, outputStore, errorBoxStore, warningBoxStore };
   },
});
</script>
