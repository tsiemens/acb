import { createApp } from 'vue';
import { initAppUI } from './acb_app.js';
import { loadGitUserCaveatIssues } from './github.js';
import wasm_init, { get_acb_version } from './pkg/acb_wasm.js';
import { WasmVersionDisplay } from './ui_model/misc.js';
import { ErrorBox } from './ui_model/error_displays.js';
import { getErrorBoxStore } from './vue/error_box_store.js';
import ErrorBoxVue from './vue/ErrorBox.vue';
import { getFileManagerStore, FileKind } from './vue/file_manager_store.js';
import FileManagerDrawer from './vue/FileManagerDrawer.vue';

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

   const fileManagerStore = getFileManagerStore();
   // Sample files. TODO delete this.
   const d = (k: FileKind) => FileKind.isDownloadableDefault(k);
   fileManagerStore.addFile( { name: 'transactions_2024.csv',           kind: FileKind.AcbTxCsv,      isDownloadable: d(FileKind.AcbTxCsv),      useChecked: true,  data: new Uint8Array() });
   fileManagerStore.addFile( { name: 'transactions_2023.csv',           kind: FileKind.AcbTxCsv,      isDownloadable: d(FileKind.AcbTxCsv),      useChecked: false, data: new Uint8Array() });
   fileManagerStore.addFile( { name: 'acb_export_2024-01-15T12-00.zip', kind: FileKind.AcbOutputZip,  isDownloadable: d(FileKind.AcbOutputZip),  useChecked: false, data: new Uint8Array() });
   fileManagerStore.addFile( { name: 'acb_export_2023-12-31T08-30.zip', kind: FileKind.AcbOutputZip,  isDownloadable: d(FileKind.AcbOutputZip),  useChecked: false, data: new Uint8Array() });
   fileManagerStore.addFile( { name: 'summary_output.txt',              kind: FileKind.OutputText,    isDownloadable: d(FileKind.OutputText),    useChecked: false, data: new Uint8Array() });
   fileManagerStore.addFile( { name: 'questrade_export.xlsx',           kind: FileKind.QuestradeXlsx, isDownloadable: d(FileKind.QuestradeXlsx), useChecked: true,  data: new Uint8Array() });
   fileManagerStore.addFile( { name: 'unrecognized_file.dat',           kind: FileKind.Other,         isDownloadable: d(FileKind.Other),         useChecked: false, data: new Uint8Array(), warning: 'File type could not be determined' });
   fileManagerStore.hasNotification = true;
   createApp(FileManagerDrawer, { store: fileManagerStore }).mount('#fileManagerApp');
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
