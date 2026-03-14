import { createApp } from 'vue';
import { loadAndAddFilesToFileManager, runHandler, initAppUI } from './acb_app.js';
import { AcbAppRunMode } from "./common/acb_app_types.js";
import { loadGitUserCaveatIssues } from './github.js';
import wasm_init, { get_acb_version } from './pkg/acb_wasm.js';
import { getSidebarInfoStore } from './vue/sidebar_info_store.js';
import { getFileManagerStore } from './vue/file_manager_store.js';
import { FileKind } from './vue/file_manager_store.js';
import { loadTestFile } from './debug.js';
import App from './vue/App.vue';

function createVueApp(): void {
   createApp(App, {
      onFilesDropped: loadAndAddFilesToFileManager,
      onRunAction: (mode: AcbAppRunMode) => {
         runHandler(mode);
      },
      onAutoRun: () => {
         loadTestFile((testFile) => {
            const store = getFileManagerStore();
            const encoder = new TextEncoder();
            store.addFile({
               name: testFile.name,
               kind: FileKind.AcbTxCsv,
               isDownloadable: false,
               useChecked: true,
               data: encoder.encode(testFile.contents),
            });
            runHandler(AcbAppRunMode.Run);
         });
      },
   }).mount('#app');
}

/**
 * Initialize the library and WASM module
 */
export async function initWasmLib(): Promise<void> {
   try {
     await wasm_init();
     console.log("Rust wasm init complete");
   } catch (error) {
     console.error("Failed to initialize rust wasm library:", error);
     throw error;
   }
 }

/**
 * Initialize the application
 */
export async function init(): Promise<void> {
   console.log("Starting application initialization");
   try {
      await initWasmLib();
      getSidebarInfoStore().acbVersion = `v${get_acb_version()}`;

      createVueApp();
      initAppUI();

      loadGitUserCaveatIssues();

      console.log("Application initialization complete");
   } catch (error) {
     console.error("Application initialization failed:", error);
   }
}
