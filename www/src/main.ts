import { createApp } from 'vue';
import {
   autoRunHandler as debugAutoRunHandler,
   loadAndAddFilesToFileManager,
   runHandler as acbRunHandler } from './acb_app.js';
import { runHandler as brokerConvertRunHandler } from './broker_convert_app.js';
import { AcbAppRunMode } from "./common/acb_app_types.js";
import { loadGitUserCaveatIssues } from './github.js';
import wasm_init, { get_acb_version } from './pkg/acb_wasm.js';
import { getSidebarInfoStore } from './vue/sidebar_info_store.js';
import { getTabStore, TabId } from './vue/tab_store.js';
import App from './vue/App.vue';

function createVueApp(): void {
   const tabStore = getTabStore();

   createApp(App, {
      onFilesDropped: loadAndAddFilesToFileManager,
      onRunAction: (mode: AcbAppRunMode) => {
         switch (tabStore.activeTab) {
            case TabId.AcbCalc:
               acbRunHandler(mode);
               break;
            case TabId.BrokerConvert:
               brokerConvertRunHandler(mode);
               break;
         }
      },
      onAutoRun: debugAutoRunHandler,
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
      loadGitUserCaveatIssues();

      console.log("Application initialization complete");
   } catch (error) {
     console.error("Application initialization failed:", error);
   }
}
