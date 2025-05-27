import { CsvFilesLoader, FileLoadResult, FileStager, printMetadataForFileList } from "./file_reader.js";
import { run_acb } from './pkg/acb_wasm.js';
import { Result } from "./result.js";
import { AppResultOk } from "./acb_wasm_types.js";
import { AcbOutput, AggregateOutputContainer, SecurityTablesOutputContainer, TextOutputContainer } from "./ui_model/acb_app_output.js";
import { AcbExtraOptions, InitialSymbolStateInputs, InitSecItem, RunButton } from "./ui_model/app_input.js";
import { ErrorBox } from "./ui_model/error_displays.js";
import { ClearFilesButton, FileDropArea, FileSelectorInput, SelectedFileList } from "./ui_model/file_input.js";
import { AutoRunCheckbox, DebugSettings } from "./ui_model/debug.js";
import { loadTestFile } from "./debug.js";
import { InfoDialog, InfoListItem } from "./ui_model/info_dialogs.js";
import { CollapsibleRegion } from "./ui_model/components.js";

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

async function asyncRunAcb(filenames: string[], contents: string[]) {
   console.debug("asyncRunAcb", filenames);
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
      return;
   }
   const initSecs = initSecsRes.unwrap();

   try {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any, @typescript-eslint/no-unsafe-assignment
      const jsRet: any = await run_acb(filenames, contents, initSecs,
         printFullDollarValues);
      console.debug("asyncRunAcb: run_acb returned: ", jsRet);
      const ret: AppResultOk = AppResultOk.fromJsValue(jsRet);

      TextOutputContainer.get().setText(ret.textOutput);
      SecurityTablesOutputContainer.get().populateTables(ret.modelOutput);
      AggregateOutputContainer.get().populateTable(ret.modelOutput);
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

function loadAllFileInfoAndRun() {
   console.log("loadAllFileInfoAndRun");
   const fileList = FileStager.globalInstance.getFilesToUseList();

   const loader = new CsvFilesLoader(fileList);
   loader.loadFiles((result: FileLoadResult) => {
      console.debug("loadAllFileInfoAndRun: loadFiles result: ", result);
      if (result.loadErrors.length > 0) {
         // Show the first error.
         const error = result.loadErrors[0];
         console.log("Error loading files: ", result.loadErrors);
         ErrorBox.getMain().showWith({
            title: "Read Error",
            descPre: error.errorDesc,
            errorText: error.error,
         });
         return;
      }

      asyncRunAcb(result.loadedFileNames, result.loadedContent)
         .then(() => {}).catch((_: unknown) => {});
   });
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
   RunButton.get().setup(loadAllFileInfoAndRun);

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