import { loadFilesAsBytes, type FileByteResult } from "./file_reader.js";
import {
   FileEntry,
   FileKind,
   getFileManagerStore,
   type FileManagerState,
   modifyDrawerNotificationForUserAddedFiles,
} from "./vue/file_manager_store.js";
import { detect_file_kind, detect_file_kind_from_pdf_pages } from "./pkg/acb_wasm.js";
import { extractPdfPages } from "./pdf_text_util.js";
import { getConfigStore, loadConfigFromFileEntry } from "./vue/config_store.js";
import { confirm as confirmDialog } from "./vue/confirm_dialog_store.js";
import { showOptionDialog, type DialogOption } from "./vue/option_dialog_store.js";

const WASM_FILE_KIND_MAP: Record<string, FileKind> = {
   'AcbTxCsv': FileKind.AcbTxCsv,
   'QuestradeExcel': FileKind.QuestradeXlsx,
   'RbcDiCsv': FileKind.RbcDiCsv,
   'EtradeTradeConfirmationPdf': FileKind.EtradeTradeConfirmationPdf,
   'EtradeBenefitPdf': FileKind.EtradeBenefitPdf,
   'EtradeBenefitsExcel': FileKind.EtradeBenefitsExcel,
   'AcbConfigJson': FileKind.AcbConfigJson,
};

export interface FileDetectResult {
   kind: FileKind;
   warning?: string;
}

export function detectFileKindFromBytes(data: Uint8Array, fileName: string): FileDetectResult {
   // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
   const wasmResult = detect_file_kind(data, fileName);
   // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
   const kind: FileKind = WASM_FILE_KIND_MAP[wasmResult.kind as string] ?? FileKind.Other;
   // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
   const warning: string | undefined = wasmResult.warning as string | undefined;
   return { kind, warning };
}

function isPdfFileName(name: string): boolean {
   return name.toLowerCase().endsWith('.pdf');
}

async function detectAndUpdatePdfEntry(entry: FileEntry): Promise<void> {
   try {
      const pages = await extractPdfPages(entry.data.buffer as ArrayBuffer);
      // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
      const wasmResult = detect_file_kind_from_pdf_pages(pages);
      // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
      const kind: FileKind = WASM_FILE_KIND_MAP[wasmResult.kind as string] ?? FileKind.GenericPdf;
      // eslint-disable-next-line @typescript-eslint/no-unsafe-member-access
      const warning: string | undefined = wasmResult.warning as string | undefined;

      entry.kind = kind;
      entry.pdfPageTexts = pages;
      entry.useChecked = warning ? false : FileKind.isInput(kind);
      if (warning) {
         entry.warning = warning;
      }
   } catch (err) {
      const errMsg = typeof err === 'string' ? err : (err instanceof Error ? err.message : String(err));
      entry.warning = `PDF extraction failed: ${errMsg}`;
      entry.useChecked = false;
   }
   entry.isDetecting = false;
   console.debug(`Finished detecting file: ${entry.name}: kind=${entry.kind}, warning=${entry.warning ?? ''}`);
}

async function maybeLoadConfigEntry(entry: FileEntry): Promise<void> {
   const configStore = getConfigStore();

   // If a config already exists, ask the user before replacing.
   if (configStore.config !== null) {
      const confirmed = await confirmDialog({
         title: 'Replace Configuration?',
         message: 'A configuration file is already loaded. Do you want to replace it with the new file?',
         confirmLabel: 'Replace',
         cancelLabel: 'Keep Existing',
      });
      if (!confirmed) {
         // Remove the newly added file entry since the user declined.
         const fileStore = getFileManagerStore();
         fileStore.removeFiles([entry.id]);
         return;
      }
   }

   try {
      const configWarnings = loadConfigFromFileEntry(configStore, entry);
      if (configWarnings.length > 0) {
         entry.warning = configWarnings.join('; ');
      }
   } catch (err) {
      const errMsg = typeof err === 'string' ? err : (err instanceof Error ? err.message : String(err));
      entry.warning = `Config load error: ${errMsg}`;
   }
}

interface FileResultWithKind {
   result: FileByteResult;
   // undefined for PDFs — they are detected asynchronously after being added.
   nonPdfDetect?: FileDetectResult;
}

function isLoadableConfig(prepared: FileResultWithKind): boolean {
   if (!prepared.nonPdfDetect) {
      return false;
   }
   const warning = prepared.result.error ?? prepared.nonPdfDetect.warning;
   return prepared.nonPdfDetect.kind === FileKind.AcbConfigJson && !warning;
}

function addLoadedResultToStore(
   store: FileManagerState,
   fileResultWithKind: FileResultWithKind,
): void {
   const { result, nonPdfDetect } = fileResultWithKind;
   if (isPdfFileName(result.name)) {
      // Add as GenericPdf immediately, then detect asynchronously.
      const entry = store.addFile({
         name: result.name,
         kind: FileKind.GenericPdf,
         isDownloadable: false,
         useChecked: false,
         data: result.data,
         warning: result.error,
         isDetecting: !result.error,
      });
      if (!result.error) {
         detectAndUpdatePdfEntry(entry)
            .catch((err: unknown) => { console.error('PDF detect error:', err); });
      }
      return;
   }

   const detectResult = nonPdfDetect ?? detectFileKindFromBytes(result.data, result.name);
   const warning = result.error ?? detectResult.warning;
   // For configs without warnings we skip the dedup marker: if the user
   // accepts the replacement prompt the existing config is overwritten,
   // and if they decline the freshly added entry is removed anyway.
   const loadableConfig = isLoadableConfig(fileResultWithKind);
   const addedEntry = store.addFile(
      {
         name: result.name,
         kind: detectResult.kind,
         isDownloadable: false,
         useChecked: warning ? false : FileKind.isInput(detectResult.kind),
         data: result.data,
         warning,
      },
      { skipDedup: loadableConfig },
   );

   if (loadableConfig) {
      maybeLoadConfigEntry(addedEntry)
         .catch((err: unknown) => { console.error('Config load error:', err); });
   }
}

type ConflictAction = 'overwrite' | 'copy' | 'nonConflicting' | 'cancel';

async function promptFileNameConflict(
   conflictingNames: string[],
   hasNonConflicting: boolean,
): Promise<ConflictAction> {
   const plural = conflictingNames.length > 1;
   const namesList = conflictingNames.map((n) => `"${n}"`).join(', ');
   const message = plural
      ? `${String(conflictingNames.length)} dropped files share names with files already in the file manager (${namesList}). How would you like to handle the conflict?`
      : `A file named ${namesList} is already in the file manager. How would you like to handle the conflict?`;

   const options: DialogOption[] = [];
   options.push({ id: 'cancel', text: 'Cancel', affirmative: false });
   if (hasNonConflicting) {
      options.push({
         id: 'nonConflicting',
         text: 'Save non-conflicting only',
         affirmative: false,
      });
   }
   options.push({
      id: 'copy',
      text: plural ? 'Save as copies' : 'Save as copy',
      affirmative: true,
   });
   options.push({
      id: 'overwrite',
      text: plural ? 'Overwrite originals' : 'Overwrite original',
      affirmative: true,
   });

   const chosen = await showOptionDialog({
      title: 'File Name Conflict',
      message,
      options,
   });
   if (chosen === null) {
      return 'cancel';
   }
   return chosen as ConflictAction;
}

async function processLoadedFileResults(results: FileByteResult[]): Promise<void> {
   const store = getFileManagerStore();

   // Pre-detect non-PDF kinds so we can classify loadable configs separately
   // from the generic conflict flow below.
   const filesWithKinds: FileResultWithKind[] = results.map((result) => ({
      result,
      nonPdfDetect: isPdfFileName(result.name)
         ? undefined
         : detectFileKindFromBytes(result.data, result.name),
   }));

   // Build a map of existing file names → entry IDs. Conflicts within the
   // incoming batch (two dropped files sharing a name) are intentionally
   // ignored here — they are handled by the existing dedupe in addFile().
   const existingByName = new Map<string, number>();
   for (const f of store.files) {
      existingByName.set(f.name, f.id);
   }

   // Loadable configs have their own "Replace Configuration?" prompt, so we
   // exclude them from the generic file-name conflict dialog entirely and
   // treat them as non-conflicting here.
   const conflicting: FileResultWithKind[] = [];
   const nonConflicting: FileResultWithKind[] = [];
   for (const p of filesWithKinds) {
      if (!isLoadableConfig(p) && existingByName.has(p.result.name)) {
         conflicting.push(p);
      } else {
         nonConflicting.push(p);
      }
   }

   let action: ConflictAction = 'copy';
   if (conflicting.length > 0) {
      action = await promptFileNameConflict(
         conflicting.map((p) => p.result.name),
         nonConflicting.length > 0,
      );
   }

   if (action === 'cancel') {
      return;
   }

   let toAdd: FileResultWithKind[];
   if (action === 'overwrite') {
      const idsToRemove: number[] = [];
      for (const p of conflicting) {
         const id = existingByName.get(p.result.name);
         if (id !== undefined) {
            idsToRemove.push(id);
         }
      }
      store.removeFiles(idsToRemove);
      toAdd = [...nonConflicting, ...conflicting];
   } else if (action === 'nonConflicting') {
      toAdd = nonConflicting;
   } else {
      // 'copy': keep existing files; addFile() will dedupe the names.
      toAdd = [...nonConflicting, ...conflicting];
   }

   for (const p of toAdd) {
      addLoadedResultToStore(store, p);
   }
   modifyDrawerNotificationForUserAddedFiles(store);
}

export function loadAndAddFilesToFileManager(fileList: FileList): void {
   const files = Array.from(fileList);
   loadFilesAsBytes(files, (results) => {
      processLoadedFileResults(results)
         .catch((err: unknown) => { console.error('File load error:', err); });
   });
}
