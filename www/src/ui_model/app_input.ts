import { ButtonElementModel, ElementModel } from "./model_lib.js";
import { childHasFocus, ElemBuilder, InputElemBuilder } from "../dom_utils.js";
import { AppFunctionMode } from "../common/acb_app_types.js";

export class RunButton extends ButtonElementModel {
   public static readonly ID: string = "runButton";

   public static get(): RunButton {
      return new RunButton(
         ElementModel.getRequiredElementById(RunButton.ID));
   }

   public setup(runApp: () => void) {
      this.setEnabled(false);
      this.setClickListener((_event) => {
         runApp();
      });
   }
}

export class ExportButton extends ButtonElementModel {
   public static readonly ID: string = "exportButton";

   public static get(): ExportButton {
      return new ExportButton(
         ElementModel.getRequiredElementById(ExportButton.ID));
   }

   public setup(runAppForExport: () => void) {
      this.setEnabled(false);
      this.setClickListener((_event) => {
         runAppForExport();
      });
   }
}

export class AcbExtraOptions {
   public static getPrintFullValuesCheckbox(): HTMLInputElement {
      return document.getElementById('printFullValuesCheckbox') as HTMLInputElement;
   }
}

export class FunctionModeSelector extends ElementModel {
   public static readonly ID: string = "acbFeatureModeSelect";

   public static get(): FunctionModeSelector {
      return new FunctionModeSelector(
         ElementModel.getRequiredElementById(FunctionModeSelector.ID));
   }

   public setup() {
      this.element.addEventListener("change", () => {
         SummaryDatePicker.get().updateForCurrentMode();
      });
   }

   public getSelectedMode(): AppFunctionMode {
      return (this.element as HTMLSelectElement).value as AppFunctionMode;
   }
}

export class SummaryDatePicker extends ElementModel {
   public static readonly ID: string = "acbSummaryDatePicker";

   private static lastPickedDates = new Map<AppFunctionMode, Date>();

   public static get(): SummaryDatePicker {
      return new SummaryDatePicker(
         ElementModel.getRequiredElementById(SummaryDatePicker.ID));
   }

   public static getLabel(): HTMLLabelElement {
      let label = document.querySelector(`label[for="${SummaryDatePicker.ID}"]`)

      if (!label) {
         throw new Error(`Could not find label for ${SummaryDatePicker.ID}`);
      }
      return label as HTMLLabelElement;
   }

   public setup() {
      this.updateForCurrentMode();
      this.element.addEventListener("change", () => {
         const date = SummaryDatePicker.get().getValue();
         if (date) {
            const mode = FunctionModeSelector.get().getSelectedMode();
            SummaryDatePicker.lastPickedDates.set(mode, date);
         }
      });
   }

   public setVisibility(visible: boolean) {
      (this.element).style.display = visible ? "inline-block" : "none";
      SummaryDatePicker.getLabel().style.display = visible ? "inline-block" : "none";
   }

   public updateForCurrentMode() {
      const funcMode = FunctionModeSelector.get().getSelectedMode();
      console.debug("DatePicker.updateVisibilityForCurrentMode", funcMode);
      this.setVisibility(funcMode === AppFunctionMode.TxSummary ||
                         funcMode === AppFunctionMode.TallyShares);

      // Check if we have a saved date for this mode
      let date = SummaryDatePicker.lastPickedDates.get(funcMode);
      if (!date) {
         date = SummaryDatePicker.getDefaultDate(funcMode);
      }
      (this.element as HTMLInputElement).value = date.toISOString().split('T')[0];
   }

   public getValue(): Date | null {
      const value = (this.element as HTMLInputElement).value;
      return value ? new Date(value) : null;
   }

   public static getDefaultDate(mode: AppFunctionMode): Date {
      const now = new Date();
      if (mode === AppFunctionMode.TxSummary) {
         let lastYear = now.getFullYear() - 1;
         return new Date(`${lastYear.toString()}-12-31`);
      }
      return now;
   }
}