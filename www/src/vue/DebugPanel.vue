<template>
  <div v-if="isDebugMode" class="user-actions">
    <button class="btn btn-secondary" @click="panelVisible = !panelVisible">Debug Settings</button>

    <div v-show="panelVisible" class="debug-panel">
      <h4>Debug Settings</h4>
      <button class="btn btn-primary btn-debug" @click="generateGithubOpenIssues">Generate GitHub Open Issues Warning</button>
      <button class="btn btn-primary btn-debug" @click="generateGithubFetchError">Generate GitHub Fetch Error</button>
      <button class="btn btn-primary btn-debug" @click="addMockFiles">Add Mock Files</button>
      <div class="checkbox-container">
        <input
          type="checkbox"
          id="debugAutoloadCheckbox"
          :checked="autoRunChecked"
          @change="onAutoRunChange"
        >
        <label for="debugAutoloadCheckbox">Auto-Run On Load</label>
      </div>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent, ref, type PropType } from 'vue';
import { handleGitUserCaveatIssues } from '../github.js';
import { isDebugModeEnabled } from '../debug.js';
import {
   getFileManagerStore,
   FileKind,
   modifyDrawerNotificationForUserAddedFiles,
} from './file_manager_store.js';

export default defineComponent({
   name: 'DebugPanel',
   props: {
      onAutoRun: {
         type: Function as PropType<() => void>,
         required: false,
      },
   },
   setup(props) {
      const isDebugMode = isDebugModeEnabled();
      const panelVisible = ref(false);

      const autoRunChecked = isDebugMode &&
         window.location.search.includes("debug_autoload=true");

      function generateGithubOpenIssues() {
         handleGitUserCaveatIssues(["some caveat"]);
      }

      function generateGithubFetchError() {
         const date = new Date();
         if ((date.getSeconds() % 2) === 0) {
            handleGitUserCaveatIssues({"message": "Sample issue from github"});
         } else {
            handleGitUserCaveatIssues({"bla": "bar"});
         }
      }

      function addMockFiles() {
         const store = getFileManagerStore();
         const d = (k: FileKind) => FileKind.isDownloadableDefault(k);
         store.addFile({ name: 'transactions_2024.csv',           kind: FileKind.AcbTxCsv,      isDownloadable: d(FileKind.AcbTxCsv),      useChecked: true,  data: new Uint8Array() });
         store.addFile({ name: 'transactions_2023.csv',           kind: FileKind.AcbTxCsv,      isDownloadable: d(FileKind.AcbTxCsv),      useChecked: false, data: new Uint8Array() });
         store.addFile({ name: 'acb_export_2024-01-15T12-00.zip', kind: FileKind.AcbOutputZip,  isDownloadable: d(FileKind.AcbOutputZip),  useChecked: false, data: new Uint8Array() });
         store.addFile({ name: 'acb_export_2023-12-31T08-30.zip', kind: FileKind.AcbOutputZip,  isDownloadable: d(FileKind.AcbOutputZip),  useChecked: false, data: new Uint8Array() });
         store.addFile({ name: 'summary_output.txt',              kind: FileKind.OutputText,    isDownloadable: d(FileKind.OutputText),    useChecked: false, data: new Uint8Array() });
         store.addFile({ name: 'questrade_export.xlsx',           kind: FileKind.QuestradeXlsx, isDownloadable: d(FileKind.QuestradeXlsx), useChecked: true,  data: new Uint8Array() });
         store.addFile({ name: 'unrecognized_file.dat',           kind: FileKind.Other,         isDownloadable: d(FileKind.Other),         useChecked: false, data: new Uint8Array(), warning: 'File type could not be determined' });
         modifyDrawerNotificationForUserAddedFiles(store);
      }

      function onAutoRunChange(event: Event) {
         const checked = (event.target as HTMLInputElement).checked;
         const url = new URL(window.location.href);
         if (checked) {
            url.searchParams.set("debug_autoload", "true");
         } else {
            url.searchParams.delete("debug_autoload");
         }
         window.location.href = url.toString();
      }

      // Trigger auto-run if checked
      if (autoRunChecked && props.onAutoRun) {
         props.onAutoRun();
      }

      return {
         isDebugMode, panelVisible, autoRunChecked,
         generateGithubOpenIssues, generateGithubFetchError,
         addMockFiles, onAutoRunChange,
      };
   },
});
</script>

<style scoped>
.debug-panel {
  position: absolute;
  border: 1px solid #ccc;
  padding: 10px;
  margin-top: 5px;
  background-color: #f9f9f9;
  z-index: 1;
  width: 300px;
  translate: -150px;
}

.btn-debug {
  margin-bottom: 5px;
}
</style>
