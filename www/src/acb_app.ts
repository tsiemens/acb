import JSZip from "jszip";
import { Unit } from "./basic_utils.js";
import { AppFunctionMode } from "./common/acb_app_types.js";
import { CsvFilesLoader, FileLoadResult, FileStager, printMetadataForFileList } from "./file_reader.js";
import { run_acb, run_acb_summary } from './pkg/acb_wasm.js';
import { Result } from "./result.js";
import { AppExportResultOk, AppResultOk, AppSummaryResultOk, FileContent, RenderTable } from "./acb_wasm_types.js";
import { AcbOutput, AggregateOutputContainer, SecurityTablesOutputContainer, TextOutputContainer, YearHighlightSelector } from "./ui_model/acb_app_output.js";
import { AcbExtraOptions, SummaryDatePicker, ExportButton, FunctionModeSelector, InitialSymbolStateInputs, InitSecItem, RunButton } from "./ui_model/app_input.js";
import { ErrorBox } from "./ui_model/error_displays.js";
import { ClearFilesButton, FileDropArea, FileSelectorInput, SelectedFileList } from "./ui_model/file_input.js";
import { AutoRunCheckbox, DebugSettings } from "./ui_model/debug.js";
import { SummaryOutputContainer } from "./ui_model/summary_output.js";
import { loadTestFile } from "./debug.js";
import { InfoDialog, InfoListItem } from "./ui_model/info_dialogs.js";
import { CollapsibleRegion } from "./ui_model/components.js";
import { asError } from "./http_utils.js";


/**
 * Get colon-delimited initial security values (format is as expected by acb).
 * */
function getInitSecStrs(): Result<string[], string[]> {
   const items: InitSecItem[] = InitialSymbolStateInputs.get().getData();
   const initSecStrs: string[] = [];
   const errors: string[] = [];

   // Empty items are ignored
   for (const item of items) {
      if (item.security) {
         if (item.quantity && item.acb) {
            initSecStrs.push(
               `${item.security}:${item.quantity}:${item.acb}`,
            );
         } else {
            errors.push(`Invalid quantity and/or ACB for ${item.security}`);
         }
      } else if (item.quantity || item.acb) {
         errors.push("Missing security name");
      }
   }

   if (errors.length) {
      return Result.Err(errors);
   }
   return Result.Ok(initSecStrs);
}

function makeZip(files: FileContent[]): Promise<Blob> {
   return new Promise((resolve, reject) => {
      try {
         // Create a zip file from the file contents
         const zip = new JSZip();
         for (const file of files) {
            zip.file(file.fileName, file.content);
         }
         zip.generateAsync({ type: "blob" })
            .then(resolve)
            .catch(reject);
      } catch (error) {
         reject(asError(error));
      }
   });
}

function makeFilenameDateString(): string {
   let date_str = new Date().toISOString();
   // Replace colons and dots for filename safety
   date_str = date_str.replace(/[:.]/g, "-");
   return date_str;
}

function downloadBlob(filename: string, blob: Blob) {
   // Create a temporary link to trigger the download
   const url = URL.createObjectURL(blob);
   const a = document.createElement("a");
   a.href = url;
   a.style.display = "none";
   a.download = filename;
   document.body.appendChild(a);
   a.click();
   // Clean up the URL object
   document.body.removeChild(a);
   URL.revokeObjectURL(url);
}

function makeZipAndDownload(files: FileContent[]): void {
   makeZip(files).then((zipBlob) => {
      let date_str = makeFilenameDateString();
      const filename = `acb_export_${date_str}.zip`;
      downloadBlob(filename, zipBlob);
   }).catch((err: unknown) => {
      console.error("Error creating zip file: ", err);
      ErrorBox.getMain().showWith({
         title: "Export Error",
         descPre: "An error occurred while creating the export zip file:",
         errorText: String(err),
      });
   });
}

function downloadCsv(filenameBase: string, csvContent: string) {
   let date_str = makeFilenameDateString();
   const filename = `${filenameBase}_${date_str}.csv`;
   const blob = new Blob([csvContent], { type: "text/csv" });
   downloadBlob(filename, blob);
}

enum AcbAppRunMode {
   Normal = "normal",
   Export = "export",
}

class CommonRunOptions {
   constructor(
      public printFullDollarValues: boolean,
      public initSecs: string[],
   ) {}
}

function getCommonRunOptions(): Result<CommonRunOptions, Unit> {
   const printFullDollarValues: boolean =
      AcbExtraOptions.getPrintFullValuesCheckbox().checked;
   const initSecsRes = getInitSecStrs();
   if (initSecsRes.isErr()) {
      const errors = initSecsRes.unwrapErr();
      console.error("asyncRunAcb: initSecsRes error: ", errors);
      ErrorBox.getMain().showWith({
         title: "Processing Error",
         descPre: "There was a problem with the initial security values:",
         errorText: errors.join("\n"),
      });
      return Result.Err(Unit.get());
   }
   const initSecs = initSecsRes.unwrap();
   return Result.Ok(new CommonRunOptions(printFullDollarValues, initSecs));
}

async function asyncRunAcb(filenames: string[], contents: string[],
                           mode: AcbAppRunMode = AcbAppRunMode.Normal
) {
   console.debug("asyncRunAcb", filenames);
   const commonOptions = getCommonRunOptions();
   if (commonOptions.isErr()) {
      // Error already handled in getCommonRunOptions
      return;
   }
   const { printFullDollarValues, initSecs } = commonOptions.unwrap();

   const exportMode: boolean = mode === AcbAppRunMode.Export;

   try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any, @typescript-eslint/no-unsafe-assignment
      const jsRet: any = await run_acb(filenames, contents, initSecs,
         printFullDollarValues, exportMode);
      console.debug("asyncRunAcb: run_acb returned: ", jsRet);

      if (exportMode) {
         const ret = AppExportResultOk.fromJsValue(jsRet);
         makeZipAndDownload(ret.csvFiles);
         return;
      }

      const ret: AppResultOk = AppResultOk.fromJsValue(jsRet);

      AcbOutput.setAppFunctionViewMode(AppFunctionMode.Calculate);

      TextOutputContainer.get().setText(ret.textOutput);
      SecurityTablesOutputContainer.get().populateTables(ret.modelOutput);
      AggregateOutputContainer.get().populateTable(ret.modelOutput);
      YearHighlightSelector.get().updateSelectableYears(
         SecurityTablesOutputContainer.get().getYearsShownInverseOrdered()
      );
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
   const { printFullDollarValues, initSecs } = commonOptions.unwrap();

   // Also, pass split_annual_summary_gains as true (or add UI for it if needed).
   const splitAnnualSummaryGains = true;

   try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any, @typescript-eslint/no-unsafe-assignment
      const jsRet: any = await run_acb_summary(
         latestDate, filenames, contents, initSecs, splitAnnualSummaryGains, printFullDollarValues
      );
      console.debug("asyncRunAcbSummary: run_acb_summary returned: ", jsRet);
      const ret: AppSummaryResultOk = AppSummaryResultOk.fromJsValue(jsRet);

      if (mode === AcbAppRunMode.Export) {
         downloadCsv("acb_summary", ret.csvText);
         return;
      }

      AcbOutput.setAppFunctionViewMode(AppFunctionMode.TxSummary);

      // Display CSV text output
      TextOutputContainer.get().setText(ret.csvText);
      // Display summary table in its own container
      SummaryOutputContainer.get().populateTable(ret.summaryTable);
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
   const { printFullDollarValues, initSecs } = commonOptions.unwrap();

   const splitAnnualSummaryGains = false;

   try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any, @typescript-eslint/no-unsafe-assignment
      const jsRet: any = await run_acb_summary(
         latestDate, filenames, contents, initSecs, splitAnnualSummaryGains, printFullDollarValues
      );
      console.debug("asyncRunAcbShareTally: run_acb_summary returned: ", jsRet);
      const ret: AppSummaryResultOk = AppSummaryResultOk.fromJsValue(jsRet);

      const [shareTallyTable, csvText] = generateShareTallyRenderTable(ret.summaryTable);

      if (mode === AcbAppRunMode.Export) {
         downloadCsv("acb_share_tally", csvText);
         return;
      }

      AcbOutput.setAppFunctionViewMode(AppFunctionMode.TallyShares);

      // Display CSV text output
      TextOutputContainer.get().setText(csvText);
      SummaryOutputContainer.get().populateTable(shareTallyTable);
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

function addFilesToUse(fileList: FileList): void {
   printMetadataForFileList(fileList);
   for (const file of fileList) {
      if (file.type == "text/csv") {
         if (FileStager.globalInstance.isFileSelected(file)) {
            console.log("File", file.name, "already selected.");
         } else {
            const fileId = FileStager.globalInstance.addFileToUse(file);
            SelectedFileList.get().addFileListEntry(fileId, file.name);
         }
      } else {
         console.log("File " + file.name + " ignored. Not CSV.");
      }
   }
}

export function initAppUI() {
   InfoDialog.initAll();
   InfoListItem.initAll();

   FileDropArea.get().setup(addFilesToUse);
   FileSelectorInput.get().setup(addFilesToUse);
   SelectedFileList.get().setup((fileId: number) => {
      console.log("onRemoveFile", fileId);
      FileStager.globalInstance.removeFile(fileId);
   });
   ClearFilesButton.get().setup();
   CollapsibleRegion.initAll();

   FunctionModeSelector.get().setup();
   SummaryDatePicker.get().setup();

   function runHandler(acbRunMode: AcbAppRunMode = AcbAppRunMode.Normal) {
      const funcMode = FunctionModeSelector.get().getSelectedMode();
      const fileList = FileStager.globalInstance.getFilesToUseList();
      const loader = new CsvFilesLoader(fileList);
      loader.loadFiles((result: FileLoadResult) => {
         if (result.loadErrors.length > 0) {
            const error = result.loadErrors[0];
            ErrorBox.getMain().showWith({
               title: "Read Error",
               descPre: error.errorDesc,
               errorText: error.error,
            });
            return;
         }
         switch (funcMode) {
            case AppFunctionMode.Calculate:
               asyncRunAcb(result.loadedFileNames, result.loadedContent, acbRunMode)
                  .then(() => {}).catch((_: unknown) => {});
               break;
            case AppFunctionMode.TxSummary: {
               const datePicker = SummaryDatePicker.get();
               const latestDate = datePicker.getValue() || SummaryDatePicker.getDefaultDate(funcMode);
               asyncRunAcbSummary(result.loadedFileNames, result.loadedContent, latestDate, acbRunMode)
                  .then(() => {}).catch((_: unknown) => {});
               break;
            }
            case AppFunctionMode.TallyShares: {
               const datePicker = SummaryDatePicker.get();
               const latestDate = datePicker.getValue() || SummaryDatePicker.getDefaultDate(funcMode);
               asyncRunAcbShareTally(result.loadedFileNames, result.loadedContent, latestDate, acbRunMode)
                  .then(() => {}).catch((_: unknown) => {});
               break;
            }
         }
      });
   }

   RunButton.get().setup(() => { runHandler(AcbAppRunMode.Normal) });
   ExportButton.get().setup(() => { runHandler(AcbAppRunMode.Export) });

   InitialSymbolStateInputs.get().setup();

   AcbOutput.setup();

   DebugSettings.init();
   // Debug auto-run
   if (AutoRunCheckbox.get().checked()) {
      loadTestFile((testFile) => {
         asyncRunAcb([testFile.name], [testFile.contents])
            .then(() => {}).catch((_: unknown) => {});
      })
   }
}