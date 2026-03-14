import { createApp } from 'vue';
import { loadAndAddFilesToFileManager, runHandler, initAppUI } from './acb_app.js';
import { AcbAppRunMode } from "./common/acb_app_types.js";
import FileDropArea from './vue/FileDropArea.vue';
import SplitRunButton from './vue/SplitRunButton.vue';
import { loadGitUserCaveatIssues } from './github.js';
import wasm_init, { get_acb_version } from './pkg/acb_wasm.js';
import { WasmVersionDisplay } from './ui_model/misc.js';
import { ErrorBox } from './ui_model/error_displays.js';
import { getErrorBoxStore } from './vue/error_box_store.js';
import ErrorBoxVue from './vue/ErrorBox.vue';
import { getFileManagerStore } from './vue/file_manager_store.js';
import FileManagerDrawer from './vue/FileManagerDrawer.vue';
import AppInputControls from './vue/AppInputControls.vue';
import { getAppInputStore } from './vue/app_input_store.js';

function createVueApps(): void {
   // Inject components which have been converted to Vue apps.
   // Eventually, may be able to have a single app if everything gets converted (?).

   createApp(ErrorBoxVue, {
      store: getErrorBoxStore(ErrorBox.MAIN_ERRORS_ID),
   }).mount(`#${ErrorBox.MAIN_ERRORS_ID}`);

   createApp(ErrorBoxVue, {
      store: getErrorBoxStore(ErrorBox.GIT_ERRORS_ID),
      width: '100%',
   }).mount(`#${ErrorBox.GIT_ERRORS_ID}`);

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
      WasmVersionDisplay.get().setVersion(get_acb_version());

      createVueApps();
      initAppUI();

      loadGitUserCaveatIssues();

      console.log("Application initialization complete");
   } catch (error) {
     console.error("Application initialization failed:", error);
   }
}
