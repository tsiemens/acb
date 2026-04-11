/**
 * Debug bridge module — imports from debug.ts, acb_app.ts, file_manager_store,
 * and tab_store. Only imported by main.ts (and DebugPanel.vue for manifest/load).
 */

import { loadJSON } from "./http_utils.js";
import { runHandler } from "./acb_app.js";
import { detectFileKindFromBytes } from "./file_load.js";
import { FileKind, getFileManagerStore } from "./vue/file_manager_store.js";
import { getTabStore, tabs } from "./vue/tab_store.js";
import { runHandler as brokerConvertRunHandler } from "./broker_convert_app.js";
import { AcbAppRunMode } from "./common/acb_app_types.js";
import { detect_file_kind_from_pdf_pages } from "./pkg/acb_wasm.js";
import { getConfigStore, loadConfigFromFileEntry } from "./vue/config_store.js";

// ---------------------------------------------------------------------------
// Manifest types
// ---------------------------------------------------------------------------

export interface ManifestFile {
   path: string;   // relative to /samples/
   name: string;
}

export interface ManifestSet {
   id: string;
   label: string;
   files: ManifestFile[];
}

export interface Manifest {
   sets: ManifestSet[];
}

// ---------------------------------------------------------------------------
// Manifest loading (cached)
// ---------------------------------------------------------------------------

let _manifest: Manifest | null = null;

export async function loadManifest(): Promise<Manifest> {
   if (_manifest) return _manifest;
   const raw = await loadJSON("/samples/manifest.json");
   _manifest = raw as Manifest;
   return _manifest;
}

// ---------------------------------------------------------------------------
// Sample set loading
// ---------------------------------------------------------------------------

const WASM_FILE_KIND_MAP: Record<string, FileKind> = {
   'AcbTxCsv': FileKind.AcbTxCsv,
   'QuestradeExcel': FileKind.QuestradeXlsx,
   'RbcDiCsv': FileKind.RbcDiCsv,
   'EtradeTradeConfirmationPdf': FileKind.EtradeTradeConfirmationPdf,
   'EtradeBenefitPdf': FileKind.EtradeBenefitPdf,
   'EtradeBenefitsExcel': FileKind.EtradeBenefitsExcel,
   'AcbConfigJson': FileKind.AcbConfigJson,
};

async function fetchBytes(url: string): Promise<Uint8Array> {
   const resp = await fetch(url);
   if (!resp.ok) {
      throw new Error(`Failed to fetch ${url}: ${resp.status.toString()} ${resp.statusText}`);
   }
   const buf = await resp.arrayBuffer();
   return new Uint8Array(buf);
}

/** Load all files belonging to one or more sets into the file manager store. */
export async function loadSampleSet(setId: string): Promise<void> {
   const manifest = await loadManifest();

   let setsToLoad: ManifestSet[];
   if (setId === "all") {
      setsToLoad = manifest.sets;
   } else {
      const found = manifest.sets.find(s => s.id === setId);
      if (!found) {
         console.warn(`loadSampleSet: set '${setId}' not found in manifest`);
         return;
      }
      setsToLoad = [found];
   }

   const store = getFileManagerStore();
   const encoder = new TextEncoder();

   for (const set of setsToLoad) {
      for (const file of set.files) {
         const url = `/samples/${file.path}`;
         try {
            const data = await fetchBytes(url);

            if (file.name.toLowerCase().endsWith(".txt")) {
               // Pre-extracted PDF page text: treat content as a single page.
               const content = new TextDecoder().decode(data);
               // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
               const wasmResult = detect_file_kind_from_pdf_pages([content]);
               // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
               const kind: FileKind = WASM_FILE_KIND_MAP[wasmResult.kind as string] ?? FileKind.GenericPdf;
               // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
               const warning: string | undefined = wasmResult.warning as string | undefined;

               store.addFile({
                  name: file.name,
                  kind,
                  isDownloadable: false,
                  useChecked: warning ? false : FileKind.isInput(kind),
                  data: encoder.encode(content),
                  pdfPageTexts: [content],
                  warning,
               });
            } else {
               const detectResult = detectFileKindFromBytes(data, file.name);
               const warning = detectResult.warning;
               const addedEntry = store.addFile({
                  name: file.name,
                  kind: detectResult.kind,
                  isDownloadable: false,
                  useChecked: warning ? false : FileKind.isInput(detectResult.kind),
                  data,
                  warning,
               });

               if (detectResult.kind === FileKind.AcbConfigJson && !warning) {
                  try {
                     loadConfigFromFileEntry(getConfigStore(), addedEntry);
                  } catch (err) {
                     const msg = err instanceof Error ? err.message : String(err);
                     addedEntry.warning = `Config load error: ${msg}`;
                  }
               }
            }
         } catch (err) {
            const msg = err instanceof Error ? err.message : String(err);
            console.error(`loadSampleSet: failed to load ${file.name}:`, msg);
            store.addFile({
               name: file.name,
               kind: FileKind.Other,
               isDownloadable: false,
               useChecked: false,
               data: new Uint8Array(),
               warning: `Load failed: ${msg}`,
            });
         }
      }
   }
}

// ---------------------------------------------------------------------------
// Auto-run: load samples then trigger run when a tab becomes ready
// ---------------------------------------------------------------------------

const AUTO_RUN_POLL_INTERVAL_MS = 100;
const AUTO_RUN_TIMEOUT_MS = 5000;

/**
 * Load the sample set, then poll until any tab's run button becomes enabled
 * and trigger the corresponding run handler.
 */
export async function autoRunHandler(setId: string): Promise<void> {
   await loadSampleSet(setId);

   const tabStore = getTabStore();

   await new Promise<void>((resolve) => {
      const deadline = Date.now() + AUTO_RUN_TIMEOUT_MS;
      const timer = setInterval(() => {
         // Find the first tab with run enabled.
         for (const tab of tabs) {
            if (tabStore.runEnabledByTab.get(tab.id)) {
               clearInterval(timer);
               if (tab.id === "broker-convert") {
                  brokerConvertRunHandler(AcbAppRunMode.Run);
               } else {
                  runHandler(AcbAppRunMode.Run);
               }
               resolve();
               return;
            }
         }
         if (Date.now() >= deadline) {
            clearInterval(timer);
            console.warn("autoRunHandler: timed out waiting for run button to become enabled");
            resolve();
         }
      }, AUTO_RUN_POLL_INTERVAL_MS);
   });
}
