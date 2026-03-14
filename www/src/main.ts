import { createApp } from 'vue';
import { loadAndAddFilesToFileManager, runHandler, initAppUI } from './acb_app.js';
import { AcbAppRunMode } from "./common/acb_app_types.js";
import { loadGitUserCaveatIssues } from './github.js';
import wasm_init, { get_acb_version } from './pkg/acb_wasm.js';
import { ErrorBox } from './ui_model/error_displays.js';
import { getErrorBoxStore } from './vue/error_box_store.js';
import ErrorBoxVue from './vue/ErrorBox.vue';
import { getFileManagerStore } from './vue/file_manager_store.js';
import FileManagerDrawer from './vue/FileManagerDrawer.vue';
import CollapsibleRegion from './vue/CollapsibleRegion.vue';
import { getSidebarInfoStore } from './vue/sidebar_info_store.js';
import InfoDialogs from './vue/InfoDialogs.vue';
import { getInfoDialogStore } from './vue/info_dialog_store.js';
import { getOutputStore } from './vue/output_store.js';
import { FileKind } from './vue/file_manager_store.js';
import { loadTestFile } from './debug.js';
import Sidebar from './vue/Sidebar.vue';
import AppHeader from './vue/AppHeader.vue';
import MainContent from './vue/MainContent.vue';

function createVueApps(): void {
   createApp(InfoDialogs, {
      store: getInfoDialogStore(),
   }).mount('#infoDialogsApp');

   // Sidebar must mount before git issues ErrorBox, since SidebarInfo provides its container
   createApp(Sidebar).mount('#sidebarApp');

   createApp(ErrorBoxVue, {
      store: getErrorBoxStore(ErrorBox.GIT_ERRORS_ID),
      width: '100%',
   }).mount(`#${ErrorBox.GIT_ERRORS_ID}`);

   // MainContent must mount before CollapsibleRegion, since OutputArea provides its container
   createApp(MainContent, {
      onFilesDropped: loadAndAddFilesToFileManager,
      onRunAction: (mode: AcbAppRunMode) => {
         runHandler(mode);
      },
   }).mount('#mainContentApp');

   createApp(CollapsibleRegion, {
      store: getOutputStore(),
   }).mount('#collapsibleRegionApp');

   createApp(AppHeader, {
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
   }).mount('#appHeaderApp');

   createApp(FileManagerDrawer, {
      store: getFileManagerStore(),
      onFilesDropped: loadAndAddFilesToFileManager,
   }).mount('#fileManagerApp');
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

      createVueApps();
      initAppUI();

      loadGitUserCaveatIssues();

      console.log("Application initialization complete");
   } catch (error) {
     console.error("Application initialization failed:", error);
   }
}
