import { Unit } from "./basic_utils.js";
import { AcbAppRunMode, AppFunctionMode } from "./common/acb_app_types.js";
import { fileBytesToString, loadFilesAsBytes } from "./file_reader.js";
import { FileEntry, FileKind, getFileManagerStore, modifyDrawerNotificationForUserAddedFiles } from './vue/file_manager_store.js';
import { loadTestFile } from "./debug.js";
import { run_acb, run_acb_summary } from './pkg/acb_wasm.js';
import { Result } from "./result.js";
import { AppExportResultOk, AppResultOk, AppSummaryResultOk, RenderTable } from "./acb_wasm_types.js";
import { getOutputStore, setAppFunctionViewMode } from "./vue/output_store.js";
import { getAppInputStore, getSummaryDate } from "./vue/app_input_store.js";
import { ErrorBox } from "./vue/error_box_store.js";
import { downloadCsv, makeZipAndDownload } from "./download_utils.js";

class CommonRunOptions {
   constructor(
      public printFullDollarValues: boolean,
   ) {}
}

function getCommonRunOptions(): Result<CommonRunOptions, Unit> {
   const printFullDollarValues: boolean = getAppInputStore().printFullValues;
   return Result.Ok(new CommonRunOptions(printFullDollarValues));
}

async function asyncRunAcb(filenames: string[], contents: string[],
                           mode: AcbAppRunMode = AcbAppRunMode.Run
) {
   console.debug("asyncRunAcb", filenames);
   const commonOptions = getCommonRunOptions();
   if (commonOptions.isErr()) {
      // Error already handled in getCommonRunOptions
      return;
   }
   const { printFullDollarValues } = commonOptions.unwrap();

   const exportMode: boolean = mode === AcbAppRunMode.Export;

   try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any, @typescript-eslint/no-unsafe-assignment
      const jsRet: any = await run_acb(filenames, contents,
         printFullDollarValues, exportMode);
      console.debug("asyncRunAcb: run_acb returned: ", jsRet);

      if (exportMode) {
         const ret = AppExportResultOk.fromJsValue(jsRet);
         makeZipAndDownload(ret.csvFiles);
         return;
      }

      const ret: AppResultOk = AppResultOk.fromJsValue(jsRet);

      setAppFunctionViewMode(AppFunctionMode.Calculate);

      const outputStore = getOutputStore();
      outputStore.textOutput = ret.textOutput;
      outputStore.aggregateTable = ret.modelOutput.aggregateGainsTable;
      outputStore.securityTables = ret.modelOutput.securityTables;
      ErrorBox.getMain().hide();
   } catch (err) {
      let errMsg = typeof err === "string" ? err : (err instanceof Error ? err.message : String(err));
      console.error("asyncRunAcb caught error: ", err);
      ErrorBox.getMain().showWith({
         title: "Processing Error",
         descPre: "An error occurred while processing ACB. Please review the error details below:",
         errorText: errMsg,
         descPost: "If this seems unexpected, try clearing your cache."
      });
   }
}

// Handler for summary mode
async function asyncRunAcbSummary(filenames: string[], contents: string[], latestDate: Date, mode: AcbAppRunMode) {
   console.debug("asyncRunAcbSummary", filenames);
   const commonOptions = getCommonRunOptions();
   if (commonOptions.isErr()) {
      // Error already handled in getCommonRunOptions
      return;
   }
   const { printFullDollarValues } = commonOptions.unwrap();

   // Also, pass split_annual_summary_gains as true (or add UI for it if needed).
   const splitAnnualSummaryGains = true;

   try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any, @typescript-eslint/no-unsafe-assignment
      const jsRet: any = await run_acb_summary(
         latestDate, filenames, contents, splitAnnualSummaryGains, printFullDollarValues
      );
      console.debug("asyncRunAcbSummary: run_acb_summary returned: ", jsRet);
      const ret: AppSummaryResultOk = AppSummaryResultOk.fromJsValue(jsRet);

      if (mode === AcbAppRunMode.Export) {
         downloadCsv("acb_summary", ret.csvText);
         return;
      }

      setAppFunctionViewMode(AppFunctionMode.TxSummary);

      const outputStore = getOutputStore();
      outputStore.textOutput = ret.csvText;
      outputStore.summaryTable = ret.summaryTable;
      if (ret.summaryTable.errors && ret.summaryTable.errors.length > 0) {
         ErrorBox.getMain().showWith({
            title: "Processing Error(s)",
            descPre: "The following errors were generated during processing:",
            errorText: ret.summaryTable.errors.map(err => err.trim()).join("\n"),
         });
      } else {
         ErrorBox.getMain().hide();
      }
   } catch (err) {
      let errMsg = typeof err === "string" ? err : (err instanceof Error ? err.message : String(err));
      console.error("asyncRunAcbSummary caught error: ", err);
      ErrorBox.getMain().showWith({
         title: "Processing Error",
         descPre: "An error occurred while processing the summary. Please review the error details below:",
         errorText: errMsg,
         descPost: "If this seems unexpected, try clearing your cache."
      });
   }
}

function generateShareTallyRenderTable(txSummary: RenderTable): [RenderTable, string] {
   if (txSummary.header[0] !== "security") {
      throw new Error(`Expected 'security' column at index 0, found '${txSummary.header[0]}'`);
   }
   if (txSummary.header[4] !== "shares") {
      throw new Error(`Expected 'shares' column at index 4, found '${txSummary.header[4]}'`);
   }

   const header = ["security", "shares"];
   let csvText = header.join(",") + "\n";
   const rows = txSummary.rows.map((row) => {
      const sec = row[0];
      const shares = row[4];
      let tallyRow = [sec, shares];
      csvText += tallyRow.join(",") + "\n";
      return tallyRow;
   });
   const table = new RenderTable(
      header, rows, txSummary.footer, txSummary.notes, txSummary.errors);
   return [table, csvText];
}

// Handler for share tally mode
async function asyncRunAcbShareTally(filenames: string[], contents: string[], latestDate: Date, mode: AcbAppRunMode) {
   console.debug("asyncRunAcbShareTally", filenames);
   const commonOptions = getCommonRunOptions();
   if (commonOptions.isErr()) {
      // Error already handled in getCommonRunOptions
      return;
   }
   const { printFullDollarValues } = commonOptions.unwrap();

   const splitAnnualSummaryGains = false;

   try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any, @typescript-eslint/no-unsafe-assignment
      const jsRet: any = await run_acb_summary(
         latestDate, filenames, contents, splitAnnualSummaryGains, printFullDollarValues
      );
      console.debug("asyncRunAcbShareTally: run_acb_summary returned: ", jsRet);
      const ret: AppSummaryResultOk = AppSummaryResultOk.fromJsValue(jsRet);

      const [shareTallyTable, csvText] = generateShareTallyRenderTable(ret.summaryTable);

      if (mode === AcbAppRunMode.Export) {
         downloadCsv("acb_share_tally", csvText);
         return;
      }

      setAppFunctionViewMode(AppFunctionMode.TallyShares);

      const outputStore = getOutputStore();
      outputStore.textOutput = csvText;
      outputStore.summaryTable = shareTallyTable;
      if (shareTallyTable.errors && shareTallyTable.errors.length > 0) {
         ErrorBox.getMain().showWith({
            title: "Processing Error(s)",
            descPre: "The following errors were generated during processing:",
            errorText: shareTallyTable.errors.map(err => err.trim()).join("\n"),
         });
      } else {
         ErrorBox.getMain().hide();
      }
   } catch (err) {
      let errMsg = typeof err === "string" ? err : (err instanceof Error ? err.message : String(err));
      console.error("asyncRunAcbShareTally caught error: ", err);
      ErrorBox.getMain().showWith({
         title: "Processing Error",
         descPre: "An error occurred while processing the summary. Please review the error details below:",
         errorText: errMsg,
         descPost: "If this seems unexpected, try clearing your cache."
      });
   }
}

function detectFileKind(file: File): FileKind {
   if (file.type === 'text/csv' || file.name.endsWith('.csv')) {
      return FileKind.AcbTxCsv;
   }
   if (
      file.name.endsWith('.xlsx') ||
      file.type === 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'
   ) {
      return FileKind.QuestradeXlsx;
   }
   return FileKind.Other;
}

// NOTE (until refactoring is done): This adds files to the new
// file manager drawer.
export function loadAndAddFilesToFileManager(fileList: FileList): void {
   const files = Array.from(fileList);
   loadFilesAsBytes(files, (results) => {
      const store = getFileManagerStore();
      results.forEach((result, i) => {
         const kind = detectFileKind(files[i]);
         store.addFile({
            name: result.name,
            kind,
            isDownloadable: false,
            useChecked: result.error ? false : FileKind.isInput(kind),
            data: result.data,
            warning: result.error,
         });
      });
      modifyDrawerNotificationForUserAddedFiles(store);
   });
}

function fileEntiesToNamesAndStringContents(entries: FileEntry[]
   ): [filenames: string[], contents: string[]] {
   const filenames: string[] = [];
   const contents: string[] = [];

   for (const entry of entries) {
      if (entry.warning) {
         console.warn(`Skipping file ${entry.name} due to warning: ${entry.warning}`);
         continue;
      }
      if (!entry.useChecked) {
         console.log(`Skipping file ${entry.name} because useChecked is false.`);
         continue;
      }
      const contentStr = fileBytesToString(entry.data);
      filenames.push(entry.name);
      contents.push(contentStr);
   }
   return [filenames, contents];
}

export function autoRunHandler(): void {
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
}

export function runHandler(acbRunMode: AcbAppRunMode = AcbAppRunMode.Run) {
   const appInputStore = getAppInputStore();
   const funcMode = appInputStore.functionMode;
   const fileStore = getFileManagerStore();

   const csvFiles = fileStore.files.filter(
      f => f.kind === FileKind.AcbTxCsv && f.useChecked && !f.warning
   );
   let [filenames, filesContents] = fileEntiesToNamesAndStringContents(csvFiles);

   // TODO temporary.
   // Run button should ideally be disabled until at least one
   // valid file is selected, but this is a quick way to prevent errors from
   // trying to run with no files.
   if (filenames.length === 0) {
      ErrorBox.getMain().showWith({
         title: "No Valid Files",
         descPre: "Please add and select at least one valid CSV file before running (use the new file manager drawer).",
      });
      return;
   }

   switch (funcMode) {
      case AppFunctionMode.Calculate:
         asyncRunAcb(filenames, filesContents, acbRunMode)
            .then(() => {}).catch((_: unknown) => {});
         break;
      case AppFunctionMode.TxSummary: {
         const latestDate = getSummaryDate(appInputStore);
         asyncRunAcbSummary(filenames, filesContents, latestDate, acbRunMode)
            .then(() => {}).catch((_: unknown) => {});
         break;
      }
      case AppFunctionMode.TallyShares: {
         const latestDate = getSummaryDate(appInputStore);
         asyncRunAcbShareTally(filenames, filesContents, latestDate, acbRunMode)
            .then(() => {}).catch((_: unknown) => {});
         break;
      }
   }
}