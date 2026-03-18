import Papa from 'papaparse';
import { AcbAppRunMode } from './common/acb_app_types.js';
import { convert_xl_to_csv } from './pkg/acb_wasm.js';
import { fileBaseName, mustGet } from './basic_utils.js';
import { RenderTable } from './acb_wasm_types.js';
import { downloadSelectedFiles } from './download_utils.js';
import { FileKind, getFileManagerStore, modifyDrawerNotificationForUserAddedFiles } from './vue/file_manager_store.js';
import { getBrokerConvertOutputStore, type NamedTable } from './vue/broker_convert_output_store.js';
import { ErrorBox } from './vue/error_box_store.js';

interface XlConvertResult {
   csvText: string;
   nonFatalErrors: string[];
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function xlConvertResultFromJsValue(val: any): XlConvertResult {
   return {
      csvText: mustGet(val, 'csvText'),
      nonFatalErrors: mustGet(val, 'nonFatalErrors'),
   };
}

function csvTextToRenderTable(csvText: string): RenderTable {
   const parsed = Papa.parse<string[]>(csvText, {
      header: false,
      skipEmptyLines: true,
   });
   const rows = parsed.data;
   if (rows.length === 0) {
      return new RenderTable([], [], []);
   }
   return new RenderTable(rows[0], rows.slice(1), []);
}

export function runHandler(mode: AcbAppRunMode): void {
   const fileStore = getFileManagerStore();
   const outputStore = getBrokerConvertOutputStore();

   const xlsxFiles = fileStore.files.filter(
      f => f.kind === FileKind.QuestradeXlsx && f.useChecked && !f.warning
   );

   if (xlsxFiles.length === 0) {
      ErrorBox.getBrokerConvert().showWith({
         title: 'No Valid Files',
         descPre: 'Please add and select at least one valid xlsx file before running (use the file manager drawer).',
      });
      return;
   }

   const allNonFatalErrors: string[] = [];
   const addedFileIds: number[] = [];
   const namedTables: NamedTable[] = [];
   const allCsvTexts: string[] = [];

   for (const entry of xlsxFiles) {
      try {
         // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
         const jsRet = convert_xl_to_csv(entry.data, undefined);
         const result = xlConvertResultFromJsValue(jsRet);

         if (result.nonFatalErrors.length > 0) {
            allNonFatalErrors.push(
               ...result.nonFatalErrors.map(e => `${entry.name}: ${e}`)
            );
         }

         const csvName = fileBaseName(entry.name) + '.csv';
         namedTables.push({
            name: csvName,
            table: csvTextToRenderTable(result.csvText),
         });
         allCsvTexts.push(result.csvText);

         const encoder = new TextEncoder();
         const addedFile = fileStore.addFile({
            name: csvName,
            kind: FileKind.AcbTxCsv,
            isDownloadable: true,
            useChecked: true,
            data: encoder.encode(result.csvText),
         });
         addedFileIds.push(addedFile.id);
      } catch (err) {
         const errMsg = typeof err === 'string' ? err : (err instanceof Error ? err.message : String(err));
         console.error(`convert_xl_to_csv error for ${entry.name}: `, err);
         ErrorBox.getBrokerConvert().showWith({
            title: 'Conversion Error',
            descPre: `An error occurred while converting ${entry.name}:`,
            errorText: errMsg,
         });
         return;
      }
   }

   // Populate the output area.
   outputStore.transactionsTables = namedTables;
   outputStore.textOutput = allCsvTexts.join('\n\n');

   // Select the newly added files in the file manager.
   fileStore.setSelectedByIds(new Set(addedFileIds));
   modifyDrawerNotificationForUserAddedFiles(fileStore);

   if (mode === AcbAppRunMode.Export) {
      const addedFiles = fileStore.files.filter(f => addedFileIds.includes(f.id));
      downloadSelectedFiles(addedFiles);
   }

   if (allNonFatalErrors.length > 0) {
      ErrorBox.getBrokerConvert().showWith({
         title: 'Conversion Warning(s)',
         descPre: 'The conversion completed with the following warnings:',
         errorText: allNonFatalErrors.join('\n'),
      });
   } else {
      ErrorBox.getBrokerConvert().hide();
   }
}
