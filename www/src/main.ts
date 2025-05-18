import { initAppUI } from './acb_app.js';
import { loadGitUserCaveatIssues } from './github.js';
import wasm_init, { get_acb_version } from './pkg/acb_wasm.js';
import { WasmVersionDisplay } from './ui_model/misc.js';

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

      initAppUI();

      loadGitUserCaveatIssues();

      console.log("Application initialization complete");
   } catch (error) {
     console.error("Application initialization failed:", error);
   }
}