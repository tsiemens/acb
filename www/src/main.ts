import { createApp } from 'vue';
import { loadAndAddFilesToFileManager, runHandler as acbRunHandler } from './acb_app.js';
import { runHandler as brokerConvertRunHandler } from './broker_convert_app.js';
import { AcbAppRunMode } from "./common/acb_app_types.js";
import { loadGitUserCaveatIssues, loadAndCheckVersions } from './github.js';
import wasm_init, { get_acb_version } from './pkg/acb_wasm.js';
import { webappVersion } from './versions.js';
import { getConfigStore } from './vue/config_store.js';
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
      const acbVersion = get_acb_version();
      getSidebarInfoStore().acbVersion = `v${acbVersion}`;

      // Initialize config store early so any cached config appears in the
      // file drawer on page load (requires WASM for validation).
      getConfigStore();

      createVueApp();
      loadGitUserCaveatIssues();
      loadAndCheckVersions(acbVersion, webappVersion);

      console.log("Application initialization complete");
   } catch (error) {
     console.error("Application initialization failed:", error);
   }
}
