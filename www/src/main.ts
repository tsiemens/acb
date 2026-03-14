import { createApp } from 'vue';
import { loadAndAddFilesToFileManager, runHandler, initAppUI } from './acb_app.js';
import { AcbAppRunMode } from "./common/acb_app_types.js";
import FileDropArea from './vue/FileDropArea.vue';
import SplitRunButton from './vue/SplitRunButton.vue';
import { loadGitUserCaveatIssues } from './github.js';
import wasm_init, { get_acb_version } from './pkg/acb_wasm.js';
import { ErrorBox } from './ui_model/error_displays.js';
import { getErrorBoxStore } from './vue/error_box_store.js';
import ErrorBoxVue from './vue/ErrorBox.vue';
import { getFileManagerStore } from './vue/file_manager_store.js';
import FileManagerDrawer from './vue/FileManagerDrawer.vue';
import AppInputControls from './vue/AppInputControls.vue';
import { getAppInputStore } from './vue/app_input_store.js';
import CollapsibleRegion from './vue/CollapsibleRegion.vue';
import SidebarInfo from './vue/SidebarInfo.vue';
import { getSidebarInfoStore } from './vue/sidebar_info_store.js';
import InfoDialogs from './vue/InfoDialogs.vue';
import { getInfoDialogStore } from './vue/info_dialog_store.js';
import SidebarInfoItems from './vue/SidebarInfoItems.vue';

function createVueApps(): void {
   // Inject components which have been converted to Vue apps.
   // Eventually, may be able to have a single app if everything gets converted (?).

   createApp(InfoDialogs, {
      store: getInfoDialogStore(),
   }).mount('#infoDialogsApp');

   createApp(ErrorBoxVue, {
      store: getErrorBoxStore(ErrorBox.MAIN_ERRORS_ID),
   }).mount(`#${ErrorBox.MAIN_ERRORS_ID}`);

   createApp(SidebarInfo, {
      store: getSidebarInfoStore(),
   }).mount('#sidebarInfoApp');

   // Git issues ErrorBox must mount after SidebarInfo, which provides its container
   createApp(ErrorBoxVue, {
      store: getErrorBoxStore(ErrorBox.GIT_ERRORS_ID),
      width: '100%',
   }).mount(`#${ErrorBox.GIT_ERRORS_ID}`);

   createApp(SidebarInfoItems).mount('#sidebarInfoItemsApp');

   createApp(FileDropArea, {
      onFilesDropped: loadAndAddFilesToFileManager,
   }).mount('#fileDropAreaApp');

   createApp(AppInputControls, {
      store: getAppInputStore(),
   }).mount('#appInputControlsApp');

   const fileManagerStore = getFileManagerStore();

   createApp(SplitRunButton, {
      store: fileManagerStore,
      onAction: (mode: AcbAppRunMode) => {
         runHandler(mode);
      },
   }).mount('#splitRunButtonApp');
   createApp(CollapsibleRegion).mount('#collapsibleRegionApp');

   createApp(FileManagerDrawer, {
      store: fileManagerStore,
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
