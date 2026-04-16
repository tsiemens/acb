import { Unit } from "./basic_utils.js";
import { AcbAppRunMode, AppFunctionMode } from "./common/acb_app_types.js";
import { fileBytesToString } from "./file_reader.js";
import { FileEntry, FileKind, getFileManagerStore, modifyDrawerNotificationForUserAddedFiles } from './vue/file_manager_store.js';
import { run_acb, run_acb_summary } from './pkg/acb_wasm.js';
import { Result } from "./result.js";
import { AppExportResultOk, AppResultOk, AppSummaryResultOk, RatesCacheUpdate } from "./acb_wasm_types.js";
import { loadRatesCache, mergeRatesCacheUpdate } from "./rates_cache.js";
import { getOutputStore, setAppFunctionViewMode } from "./vue/output_store.js";
import { getAppInputStore, getSummaryDate } from "./vue/app_input_store.js";
import { getConfigJsonForWasm } from "./vue/config_store.js";
import { ErrorBox } from "./vue/error_box_store.js";
import { downloadCsv, makeZipAndDownload } from "./download_utils.js";
import { openTampermonkeyScriptDialog } from "./vue/info_dialog_store.js";
import { generateTamperMonkeyScript } from "./tampermonkey_gen.js";
import { showTampermonkeyExportDialog } from "./vue/tampermonkey_dialog_store.js";
import {
   collectSecuritiesWithErrors,
   extractSellData,
   generateShareTallyRenderTable,
} from "./render_table_utils.js";

function showSecurityErrors(secsWithErrors: string[], descPost?: string): void {
   ErrorBox.getMain().showWith({
      title: "Processing Error(s)",
      descPre: "Errors were generated for the following securities:",
      errorText: secsWithErrors.join(", "),
      descPost: descPost ??
         "See the per-security output tables for error details.",
   });
}

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
         if (ret.securitiesWithErrors.length > 0) {
            showSecurityErrors(ret.securitiesWithErrors,
               "Please fix the errors before exporting.");
            return;
         }
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

      const secsWithErrors = collectSecuritiesWithErrors(
         ret.modelOutput.securityTables);
      if (secsWithErrors.length > 0) {
         showSecurityErrors(secsWithErrors);
      } else {
         ErrorBox.getMain().hide();
      }
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

async function asyncRunAcbForTampermonkey(filenames: string[], contents: string[]) {
   console.debug("asyncRunAcbForTampermonkey", filenames);
   const commonOptions = getCommonRunOptions();
   if (commonOptions.isErr()) {
      return;
   }
   const { printFullDollarValues } = commonOptions.unwrap();

   const cachedRates = loadRatesCache();
   const configJson = getConfigJsonForWasm();

   try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any, @typescript-eslint/no-unsafe-assignment
      const jsRet: any = await run_acb(filenames, contents,
         printFullDollarValues, /*exportMode=*/ false, cachedRates, configJson);

      const ret: AppResultOk = AppResultOk.fromJsValue(jsRet);
      maybeMergeRatesCacheUpdate(ret.ratesCacheUpdate);

      const secsWithErrors = collectSecuritiesWithErrors(ret.modelOutput.securityTables);
      if (secsWithErrors.length > 0) {
         showSecurityErrors(secsWithErrors,
            "Please fix the errors before generating the Tampermonkey script.");
         return;
      }

      const sellData = extractSellData(ret.modelOutput.securityTables);
      if (sellData.sellYears.length === 0) {
         // TODO clear error box and use warning box ?
         ErrorBox.getMain().showWith({
            title: "No Sell Transactions",
            descPre: "No sell transactions were found for non-registered affiliates. " +
               "There is nothing to include in a Tampermonkey script.",
         });
         return;
      }

      const dialogResult = await showTampermonkeyExportDialog({
         years: sellData.sellYears,
         affiliates: sellData.affiliateBaseNames,
      });
      if (dialogResult === null) return;

      const filtered = sellData.entries.filter(e => {
         const entryYear = parseInt(e.date.split('-')[0], 10);
         if (entryYear !== dialogResult.year) return false;
         if (dialogResult.affiliate !== null && e.affiliate !== dialogResult.affiliate) return false;
         return true;
      });

      const scriptContent = generateTamperMonkeyScript(filtered);
      const encoder = new TextEncoder();
      const fileStore = getFileManagerStore();
      const affSuffix = dialogResult.affiliate ? `_${dialogResult.affiliate}` : '';
      const fileName = `acb_ws_autofill_${String(dialogResult.year)}${affSuffix}.user.js`;
      const file = fileStore.addFile({
         name: fileName,
         kind: FileKind.TampermonkeyScript,
         isDownloadable: true,
         useChecked: false,
         data: encoder.encode(scriptContent),
      });
      modifyDrawerNotificationForUserAddedFiles(fileStore);
      openTampermonkeyScriptDialog(file.name, scriptContent);
   } catch (err) {
      let errMsg = typeof err === "string" ? err : (err instanceof Error ? err.message : String(err));
      console.error("asyncRunAcbForTampermonkey caught error: ", err);
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

      const summaryErrors = ret.summaryTable.errors ?? [];

      if (mode === AcbAppRunMode.Export) {
         if (summaryErrors.length > 0) {
            ErrorBox.getMain().showWith({
               title: "Processing Error(s)",
               descPre: "The following errors were generated during processing. Please fix the errors before exporting:",
               errorText: summaryErrors.map(err => err.trim()).join("\n"),
            });
            return;
         }
         downloadCsv("acb_summary", ret.csvText);
         return;
      }

      setAppFunctionViewMode(AppFunctionMode.TxSummary);

      const outputStore = getOutputStore();
      outputStore.textOutput = ret.csvText;
      outputStore.summaryTable = ret.summaryTable;
      if (summaryErrors.length > 0) {
         ErrorBox.getMain().showWith({
            title: "Processing Error(s)",
            descPre: "The following errors were generated during processing:",
            errorText: summaryErrors.map(err => err.trim()).join("\n"),
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

      const tallyErrors = shareTallyTable.errors ?? [];

      if (mode === AcbAppRunMode.Export) {
         if (tallyErrors.length > 0) {
            ErrorBox.getMain().showWith({
               title: "Processing Error(s)",
               descPre: "The following errors were generated during processing. Please fix the errors before exporting:",
               errorText: tallyErrors.map(err => err.trim()).join("\n"),
            });
            return;
         }
         downloadCsv("acb_share_tally", csvText);
         return;
      }

      setAppFunctionViewMode(AppFunctionMode.TallyShares);

      const outputStore = getOutputStore();
      outputStore.textOutput = csvText;
      outputStore.summaryTable = shareTallyTable;
      if (tallyErrors.length > 0) {
         ErrorBox.getMain().showWith({
            title: "Processing Error(s)",
            descPre: "The following errors were generated during processing:",
            errorText: tallyErrors.map(err => err.trim()).join("\n"),
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

   if (acbRunMode === AcbAppRunMode.ExportTampermonkeyScript) {
      asyncRunAcbForTampermonkey(filenames, filesContents)
         .then(() => {}).catch((_: unknown) => {});
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