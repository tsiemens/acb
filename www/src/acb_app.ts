import { Unit } from "./basic_utils.js";
import { AcbAppRunMode, AppFunctionMode } from "./common/acb_app_types.js";
import { fileBytesToString } from "./file_reader.js";
import { FileEntry, FileKind, getFileManagerStore } from './vue/file_manager_store.js';
import { run_acb, run_acb_summary } from './pkg/acb_wasm.js';
import { Result } from "./result.js";
import { AppExportResultOk, AppResultOk, AppSummaryResultOk, RatesCacheUpdate, RenderTable } from "./acb_wasm_types.js";
import { loadRatesCache, mergeRatesCacheUpdate } from "./rates_cache.js";
import { getOutputStore, setAppFunctionViewMode } from "./vue/output_store.js";
import { getAppInputStore, getSummaryDate } from "./vue/app_input_store.js";
import { getConfigJsonForWasm } from "./vue/config_store.js";
import { ErrorBox } from "./vue/error_box_store.js";
import { downloadCsv, makeZipAndDownload } from "./download_utils.js";

function maybeMergeRatesCacheUpdate(update?: RatesCacheUpdate): void {
   if (update) {
      mergeRatesCacheUpdate(update);
   }
}

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
   const cachedRates = loadRatesCache();
   const configJson = getConfigJsonForWasm();

   try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any, @typescript-eslint/no-unsafe-assignment
      const jsRet: any = await run_acb(filenames, contents,
         printFullDollarValues, exportMode, cachedRates, configJson);
      console.debug("asyncRunAcb: run_acb returned: ", jsRet);

      if (exportMode) {
         const ret = AppExportResultOk.fromJsValue(jsRet);
         maybeMergeRatesCacheUpdate(ret.ratesCacheUpdate);
         makeZipAndDownload(ret.csvFiles);
         return;
      }

      const ret: AppResultOk = AppResultOk.fromJsValue(jsRet);
      maybeMergeRatesCacheUpdate(ret.ratesCacheUpdate);

      setAppFunctionViewMode(AppFunctionMode.Calculate);

      const outputStore = getOutputStore();
      outputStore.textOutput = ret.textOutput;
      outputStore.aggregateTable = ret.modelOutput.aggregateGainsTable;
      outputStore.securityTables = ret.modelOutput.securityTables;
      outputStore.selectedAffiliate = null;
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
   const cachedRates = loadRatesCache();
   const configJson = getConfigJsonForWasm();

   try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any, @typescript-eslint/no-unsafe-assignment
      const jsRet: any = await run_acb_summary(
         latestDate, filenames, contents, splitAnnualSummaryGains, printFullDollarValues,
         cachedRates, configJson
      );
      console.debug("asyncRunAcbSummary: run_acb_summary returned: ", jsRet);
      const ret: AppSummaryResultOk = AppSummaryResultOk.fromJsValue(jsRet);
      maybeMergeRatesCacheUpdate(ret.ratesCacheUpdate);

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
   const secIdx = txSummary.header.indexOf("security");
   if (secIdx < 0) {
      throw new Error(`Expected 'security' column in header, found: ${txSummary.header.join(', ')}`);
   }
   const sharesIdx = txSummary.header.indexOf("shares");
   if (sharesIdx < 0) {
      throw new Error(`Expected 'shares' column in header, found: ${txSummary.header.join(', ')}`);
   }
   const affIdx = txSummary.header.indexOf("affiliate");
   const hasMultipleAffiliates = affIdx >= 0 &&
      new Set(txSummary.rows.map(r => r[affIdx])).size > 1;

   const header = hasMultipleAffiliates
      ? ["security", "affiliate", "shares"]
      : ["security", "shares"];
   let csvText = header.join(",") + "\n";
   const rows = txSummary.rows.map((row) => {
      const tallyRow = hasMultipleAffiliates
         ? [row[secIdx], row[affIdx], row[sharesIdx]]
         : [row[secIdx], row[sharesIdx]];
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
   const cachedRates = loadRatesCache();
   const configJson = getConfigJsonForWasm();

   try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any, @typescript-eslint/no-unsafe-assignment
      const jsRet: any = await run_acb_summary(
         latestDate, filenames, contents, splitAnnualSummaryGains, printFullDollarValues,
         cachedRates, configJson
      );
      console.debug("asyncRunAcbShareTally: run_acb_summary returned: ", jsRet);
      const ret: AppSummaryResultOk = AppSummaryResultOk.fromJsValue(jsRet);
      maybeMergeRatesCacheUpdate(ret.ratesCacheUpdate);

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

function fileEntriesToNamesAndStringContents(entries: FileEntry[]
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

export function runHandler(acbRunMode: AcbAppRunMode = AcbAppRunMode.Run) {
   const appInputStore = getAppInputStore();
   const funcMode = appInputStore.functionMode;
   const fileStore = getFileManagerStore();

   const csvFiles = fileStore.files.filter(
      f => f.kind === FileKind.AcbTxCsv && f.useChecked && !f.warning
   );
   let [filenames, filesContents] = fileEntriesToNamesAndStringContents(csvFiles);

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