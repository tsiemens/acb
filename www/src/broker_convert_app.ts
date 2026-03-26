import Papa from 'papaparse';
import { AcbAppRunMode } from './common/acb_app_types.js';
import { convert_xl_to_csv, convert_etrade_pdfs_to_csv, extract_etrade_pdf_data } from './pkg/acb_wasm.js';
import { fileBaseName, mustGet } from './basic_utils.js';
import { RenderTable } from './acb_wasm_types.js';
import { downloadSelectedFiles } from './download_utils.js';
import { FileKind, getFileManagerStore, modifyDrawerNotificationForUserAddedFiles } from './vue/file_manager_store.js';
import { getBrokerConvertOutputStore, type NamedTable } from './vue/broker_convert_output_store.js';
import { ErrorBox } from './vue/error_box_store.js';
import { getAppInputStore } from './vue/app_input_store.js';

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

interface EtradeConvertResult {
   csvText: string;
   warnings: string[];
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function etradeConvertResultFromJsValue(val: any): EtradeConvertResult {
   return {
      csvText: mustGet(val, 'csvText'),
      warnings: mustGet(val, 'warnings'),
   };
}

interface EtradeExtractResult {
   benefitsTable: RenderTable;
   tradesTable: RenderTable;
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function etradeExtractResultFromJsValue(val: any): EtradeExtractResult {
   return {
      benefitsTable: RenderTable.fromJsValue(mustGet(val, 'benefitsTable')),
      tradesTable: RenderTable.fromJsValue(mustGet(val, 'tradesTable')),
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
   const appInputStore = getAppInputStore();

   const xlsxFiles = fileStore.files.filter(
      f => f.kind === FileKind.QuestradeXlsx && f.useChecked && !f.warning
   );
   const etradePdfFiles = fileStore.files.filter(
      f => (f.kind === FileKind.EtradeTradeConfirmationPdf ||
            f.kind === FileKind.EtradeBenefitPdf) &&
           f.useChecked && !f.warning
   );

   if (xlsxFiles.length === 0 && etradePdfFiles.length === 0) {
      ErrorBox.getBrokerConvert().showWith({
         title: 'No Valid Files',
         descPre: 'Please add and select at least one valid xlsx or E*TRADE PDF file before running (use the file manager drawer).',
      });
      return;
   }

   const allNonFatalErrors: string[] = [];
   const addedFileIds: number[] = [];
   const namedTables: NamedTable[] = [];
   const allCsvTexts: string[] = [];

   // Process Questrade XLSX files
   for (const entry of xlsxFiles) {
      try {
         // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
         const jsRet = convert_xl_to_csv(entry.data, undefined, appInputStore.noFx);
         const result = xlConvertResultFromJsValue(jsRet);

         if (result.nonFatalErrors.length > 0) {
            allNonFatalErrors.push(
               ...result.nonFatalErrors.map(e => `${entry.name}: ${e}`)
            );
         }

         const csvName = fileBaseName(entry.name) + '.csv';
         allCsvTexts.push(result.csvText);

         const encoder = new TextEncoder();
         const addedFile = fileStore.addFile({
            name: csvName,
            kind: FileKind.AcbTxCsv,
            isDownloadable: true,
            useChecked: true,
            data: encoder.encode(result.csvText),
         });
         namedTables.push({
            name: addedFile.name,
            table: csvTextToRenderTable(result.csvText),
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

   // Process E*TRADE PDF files
   if (etradePdfFiles.length > 0) {
      try {
         // pdfPageTexts is always set before run (isDetecting gate on Run button).
         // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
         const pdfTexts = etradePdfFiles.map(f => f.pdfPageTexts!.join('\n'));
         const fileNames = etradePdfFiles.map(f => f.name);

         if (appInputStore.extractOnly) {
            // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
            const jsRet = extract_etrade_pdf_data(pdfTexts, fileNames);
            const result = etradeExtractResultFromJsValue(jsRet);

            namedTables.push(
               { name: 'E*TRADE Benefits (raw)', table: result.benefitsTable },
               { name: 'E*TRADE Trades (raw)', table: result.tradesTable },
            );
         } else {
            // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
            const jsRet = convert_etrade_pdfs_to_csv(pdfTexts, fileNames, !appInputStore.noFx);
            const result = etradeConvertResultFromJsValue(jsRet);

            if (result.warnings.length > 0) {
               allNonFatalErrors.push(
                  ...result.warnings.map(w => `E*TRADE: ${w}`)
               );
            }

            const csvName = 'etrade_transactions.csv';
            allCsvTexts.push(result.csvText);

            const encoder = new TextEncoder();
            const addedFile = fileStore.addFile({
               name: csvName,
               kind: FileKind.AcbTxCsv,
               isDownloadable: true,
               useChecked: true,
               data: encoder.encode(result.csvText),
            });
            namedTables.push({
               name: addedFile.name,
               table: csvTextToRenderTable(result.csvText),
            });
            addedFileIds.push(addedFile.id);
         }
      } catch (err) {
         const errMsg = typeof err === 'string' ? err : (err instanceof Error ? err.message : String(err));
         console.error('convert_etrade_pdfs_to_csv error: ', err);
         ErrorBox.getBrokerConvert().showWith({
            title: 'E*TRADE Conversion Error',
            descPre: 'An error occurred while converting E*TRADE PDFs:',
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
