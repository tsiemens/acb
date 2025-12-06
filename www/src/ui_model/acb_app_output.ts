import { ElemBuilder } from "../dom_utils.js";
import { AppRenderResult, RenderTable } from "../acb_wasm_types.js";
import { CheckboxElementModel, ElementModel } from "./model_lib.js";
import { AppFunctionMode } from "../common/acb_app_types.js";

export enum AcbOutputViewMode {
   SecurityTables = "security_tables",
   Aggregate = "aggregate",
   Summary = "summary",
   Text = "text",
}

export abstract class AcbOutputKindContainer extends ElementModel {

   private static allOutputKindContainerGetters: Map<string, () => AcbOutputKindContainer> =
      new Map();

   public static registerOutputKindContainer(name: string, getter: () => AcbOutputKindContainer) {
      AcbOutputKindContainer.allOutputKindContainerGetters.set(name, getter);
   }

   public static getAll(): Array<AcbOutputKindContainer> {
      const all = [];
      for (const getter of AcbOutputKindContainer.allOutputKindContainerGetters.values()) {
         all.push(getter());
      }
      return all;
   }

   public setActive(active: boolean) {
      const INACTIVE = 'inactive';
      if (active) {
         this.element.classList.remove(INACTIVE);
      } else {
         this.element.classList.add(INACTIVE);
      }
   }

   public abstract viewMode(): AcbOutputViewMode;
}

export class TextOutputContainer extends AcbOutputKindContainer {
   public static readonly ID: string = "acbTextOutput";

   public static get(): TextOutputContainer {
      return new TextOutputContainer(
         ElementModel.getRequiredElementById(TextOutputContainer.ID));
   }

   public viewMode(): AcbOutputViewMode { return AcbOutputViewMode.Text; }
}

AcbOutputKindContainer.registerOutputKindContainer(
   TextOutputContainer.ID, TextOutputContainer.get);

export abstract class TableOutputContainerBase extends AcbOutputKindContainer {
   protected static makeTableHeaderRow(tableModel: RenderTable): HTMLElement {
      const tr = new ElemBuilder("tr").build();
      for (const header of tableModel.header) {
         tr.appendChild(new ElemBuilder("th").text(header).build());
      }
      return tr;
   }

   protected static makeTableContainer(
      headerRowTr: HTMLElement, tbody: HTMLElement): HTMLElement {

      const table = new ElemBuilder('table').children([
         new ElemBuilder('thead').children([headerRowTr]).build(),
         tbody,
      ]).build();

      return new ElemBuilder('div')
         .classes(['table-fixed-head'])
         .children([table])
         .build();
   }

   protected static makeTableTitle(title: string): HTMLElement {
      return new ElemBuilder('div').classes(['table-title']).text(title)
         .build();
   }

   // Creates and returns divs for errors and notes from the symbol model.
   // These should generally be placed as siblings to the table container
   // (from makeTableContainer).
   protected static makeTableErrorsAndNotes(
      symbolModel: RenderTable): { errorsDiv: HTMLElement, notesDiv: HTMLElement } {

      const errorsWrapper = new ElemBuilder('div')
         .classes(['security-errors'])
         .build();

      const errors = symbolModel.errors || [];
      for (const err of errors) {
         errorsWrapper.appendChild(new ElemBuilder('p').text(err).build());
      }
      if (errors.length == 0) {
         errorsWrapper.style.display = 'none'; // Hide if no errors
      }

      const notesWrapper = new ElemBuilder('div')
         .classes(['security-notes'])
         .build();

      let notes = symbolModel.notes || []
      for (const note of notes) {
         notesWrapper.appendChild(new ElemBuilder('p').text(note).build());
      }
      if (notes.length == 0) {
         notesWrapper.style.display = 'none'; // Hide if no notes
      }

      return { errorsDiv: errorsWrapper, notesDiv: notesWrapper };
   }
}

export class SecurityTablesOutputContainer extends TableOutputContainerBase {
   public static readonly ID: string = "acbSecurityTablesOutput";

   public static get(): SecurityTablesOutputContainer {
      return new SecurityTablesOutputContainer(
         ElementModel.getRequiredElementById(SecurityTablesOutputContainer.ID));
   }

   public viewMode(): AcbOutputViewMode { return AcbOutputViewMode.SecurityTables; }

   private static addSymbolTableComponents(
      symbol: string, model: AppRenderResult,
      tablesContainer: HTMLElement): void {
      console.debug("addSymbolTableComponents for: ", symbol);

      const symbolModel = model.securityTables.get(symbol);
      if (!symbolModel) {
         throw new Error(`No symbol model found for ${symbol}`);
      }

      const yearsSet = new Set<string>();
      for (const row of symbolModel.rows) {
         const settleDate = row[2]; // Assuming date is in yyyy-mm-dd format
         const year = settleDate.split('-')[0];
         if (year) {
            yearsSet.add(year);
         }
      }

      const wrapperDiv = new ElemBuilder('div')
         .classes(['security-wrapper'])
         .attributes({
            'data-activity-years': Array.from(yearsSet).join(','),
            'data-has-error': symbolModel.errors && symbolModel.errors.length > 0 ? 'true' : 'false'
         })
         .build();

      const tr = TableOutputContainerBase.makeTableHeaderRow(symbolModel);
      const tbody = new ElemBuilder('tbody').build();

      const addRow = function (rowItems: string[]) {
         const SETTLE_DATE_COL = 2;
         const ACTION_COL = 3;

         let isBuy = rowItems[ACTION_COL].search(/buy/i) >= 0;
         let isSell = rowItems[ACTION_COL].search(/sell/i) >= 0;
         let isSfla = rowItems[ACTION_COL].search(/sprf/i) >= 0;
         let isSplit = rowItems[ACTION_COL].search(/split/i) >= 0;

         const rowElem = new ElemBuilder('tr').build();
         if (isBuy) {
            rowElem.classList.add('buy-row');
         } else if (isSell) {
            rowElem.classList.add('sell-row');
         } else if (isSfla) {
            rowElem.classList.add('sfla-row');
         } else if (isSplit) {
            rowElem.classList.add('split-row');
         } else {
            rowElem.classList.add('other-row');
         }

         // Parse year out of rowItems[SETTLE_DATE_COL], formatted as
         // yyyy-mm-dd
         const year: string = rowItems[SETTLE_DATE_COL].split('-')[0] || "unknown";
         rowElem.classList.add(`year-${year}-row`);

         for (const item of rowItems) {
            const td = new ElemBuilder('td').text(item).build();
            rowElem.appendChild(td);
         }
         tbody.appendChild(rowElem);
      };

      for (const row of symbolModel.rows) {
         addRow(row);
      }
      addRow(symbolModel.footer);

      const symTableContainer = TableOutputContainerBase.makeTableContainer(tr, tbody);
      wrapperDiv.appendChild(TableOutputContainerBase.makeTableTitle(symbol));

      let errorsAndNotes =
         TableOutputContainerBase.makeTableErrorsAndNotes(symbolModel);
      if (errorsAndNotes.errorsDiv.hasChildNodes()) {
         errorsAndNotes.errorsDiv.appendChild(new ElemBuilder('p').text(
            "Information is of parsed state only, and may not be fully correct.")
            .build());
      }

      wrapperDiv.appendChild(errorsAndNotes.errorsDiv);
      wrapperDiv.appendChild(symTableContainer);
      wrapperDiv.appendChild(errorsAndNotes.notesDiv);

      tablesContainer.appendChild(wrapperDiv);

      SecurityTablesOutputContainer.setYearRowStyles(
         YearHighlightSelector.get().getSelectedYear()
      );
   }

   public getYearsShownInverseOrdered(): number[] {
      const years = new Set<number>();
      const rows = document.querySelectorAll('[class*="year-"][class*="-row"]');
      rows.forEach(row => {
         row.classList.forEach(cls => {
            if (cls.startsWith('year-') && cls.endsWith('-row')) {
               // Extract the year part from "year-X-row"
               const yearStr = cls.slice(5, -4);
               const year = parseInt(yearStr, 10);
               if (!isNaN(year)) {
                  years.add(year);
               }
            }
         });
      });
      // Convert years to a backward-sorted array
      return Array.from(years).sort((a, b) => b - a);
   }

   /**
    * Highlights rows with this year (based on their row class)
    * @param yearToHighlight
    */
   public static setYearRowStyles(yearToHighlight: string | null) {
      const styleSheet = document.styleSheets[0];

      // Remove any existing rules for year highlighting
      for (let i = styleSheet.cssRules.length - 1; i >= 0; i--) {
         const rule = styleSheet.cssRules[i];
         if (rule.cssText.includes(`.year-`) && rule.cssText.includes(`-row`)) {
            styleSheet.deleteRule(i);
         }
      }

      // Collect all year classes currently in use
      const yearClasses = new Set<string>();
      const rows = document.querySelectorAll('[class*="year-"][class*="-row"]');
      rows.forEach(row => {
         row.classList.forEach(cls => {
            if (cls.startsWith('year-') && cls.endsWith('-row')) {
               yearClasses.add(cls);
            }
         });
      });

      // Add opacity filter for all year classes except the highlighted one
      yearClasses.forEach(yearClass => {
         if (yearToHighlight && yearClass !== `year-${yearToHighlight}-row`) {
            styleSheet.insertRule(
               `.${yearClass} { filter: opacity(0.4); }`,
               styleSheet.cssRules.length
            );
         }
      });
   }

   public populateTables(model: AppRenderResult) {
      let tablesContainer = this.element;
      tablesContainer.innerHTML = ""; // Clear previous tables

      // Symbol tables (securityTables is a Map object)
      const symbols = Array.from(model.securityTables.keys());
      symbols.sort()
      for (const symbol of symbols) {
         SecurityTablesOutputContainer.addSymbolTableComponents(
            symbol, model, tablesContainer);
      }
   }

   public static setSecurityWrapperStyles(selectedYear: string | null) {
      const styleSheet = document.styleSheets[0];

      // Remove any existing rules for security wrapper visibility
      for (let i = styleSheet.cssRules.length - 1; i >= 0; i--) {
         const rule = styleSheet.cssRules[i];
         if (rule.cssText.includes('.security-wrapper')) {
            styleSheet.deleteRule(i);
         }
      }

      if (selectedYear) {
         // Add rule to show wrappers with the selected year and no errors
         styleSheet.insertRule(
            `.security-wrapper[data-activity-years*="${selectedYear}"] { display: block; }`,
            styleSheet.cssRules.length
         );
         styleSheet.insertRule(
            `.security-wrapper[data-has-error="true"] { display: block; }`,
            styleSheet.cssRules.length
         );

         // Add rule to hide wrappers without the selected year
         styleSheet.insertRule(
            `.security-wrapper:not([data-has-error="true"]):not([data-activity-years*="${selectedYear}"]) { display: none; }`,
            styleSheet.cssRules.length
         );
      } else {
         // Ensure all wrappers are visible if no year is selected
         styleSheet.insertRule(
            `.security-wrapper { display: block; }`,
            styleSheet.cssRules.length
         );
      }
   }
}

AcbOutputKindContainer.registerOutputKindContainer(
   SecurityTablesOutputContainer.ID, SecurityTablesOutputContainer.get);

export class AggregateOutputContainer extends TableOutputContainerBase {
   public static readonly ID: string = "acbAggregateOutput";

   public static get(): AggregateOutputContainer {
      return new AggregateOutputContainer(
         ElementModel.getRequiredElementById(AggregateOutputContainer.ID));
   }

   public viewMode(): AcbOutputViewMode { return AcbOutputViewMode.Aggregate; }

   private static makeAggregateGainsTable(model: AppRenderResult): HTMLElement {
      const aggModel = model.aggregateGainsTable;
      console.log("Agg model:");
      console.log(aggModel);
      const tr = AggregateOutputContainer.makeTableHeaderRow(aggModel);
      const tbody = new ElemBuilder('tbody').build();
      for (const row of aggModel.rows) {
         const rowElem = new ElemBuilder('tr').build();
         for (const item of row) {
            const td = new ElemBuilder('td').text(item).build();
            rowElem.appendChild(td);
         }
         tbody.appendChild(rowElem);
      }
      return AggregateOutputContainer.makeTableContainer(tr, tbody);
   }

   public populateTable(model: AppRenderResult) {
      let tablesContainer = this.element;
      tablesContainer.innerHTML = ""; // Clear previous tables

      // Aggregate table
      tablesContainer.appendChild(
         AggregateOutputContainer.makeTableTitle("Aggregate Gains"));

      let errorsAndNotes =
         TableOutputContainerBase.makeTableErrorsAndNotes(model.aggregateGainsTable);

      tablesContainer.appendChild(errorsAndNotes.errorsDiv);
      tablesContainer.appendChild(
         AggregateOutputContainer.makeAggregateGainsTable(model));
      tablesContainer.appendChild(errorsAndNotes.notesDiv);
   }
}

AcbOutputKindContainer.registerOutputKindContainer(
   AggregateOutputContainer.ID, AggregateOutputContainer.get);

// Each output view mode has its own selector, for the views
// that mode supports.
export class OutputViewSelector extends ElementModel {
   public static getAll(): Array<OutputViewSelector> {
      const tabLabelElems = document.getElementsByClassName('view-mode-btn');
      const tabLabels: Array<OutputViewSelector> = [];
      for (const tabLabel of tabLabelElems) {
         tabLabels.push(new OutputViewSelector(tabLabel as HTMLElement));
      }
      return tabLabels;
   }

   public setup(onViewModeActive: (tab: AcbOutputViewMode) => void) {
      this.element.addEventListener('click', (event) => {
         if (event.target) {
            let clickedSelector = new OutputViewSelector(event.target as HTMLElement);
            let clickedMode = clickedSelector.viewMode();
            ViewModeSelectorGroup.setActiveViewMode(clickedMode);
            onViewModeActive(clickedMode);
         }
      });
   }

   public viewMode(): AcbOutputViewMode {
      let viewMode_ = this.element.dataset.viewMode;
      if (!viewMode_) {
         throw Error("TabSelector has no viewMode");
      }
      if (viewMode_ == AcbOutputViewMode.SecurityTables.toString()) {
         return AcbOutputViewMode.SecurityTables;
      } else if (viewMode_ == AcbOutputViewMode.Text.toString()) {
         return AcbOutputViewMode.Text;
      } else if (viewMode_ == AcbOutputViewMode.Aggregate.toString()) {
         return AcbOutputViewMode.Aggregate;
      } else if (viewMode_ == AcbOutputViewMode.Summary.toString()) {
         return AcbOutputViewMode.Summary;
      }
      throw Error(`Invalid AcbOutputViewMode: ${viewMode_}`);
   }

   public setActive(active: boolean) {
      const ACTIVE = 'active';
      if (active) {
         this.element.classList.add(ACTIVE);
      } else {
         this.element.classList.remove(ACTIVE);
      }
   }

   public isActive(): boolean {
      return this.element.classList.contains('active');
   }
}

class ViewModeSelectorGroup {
   public static setActiveViewMode(mode: AcbOutputViewMode) {
      for (const selector of OutputViewSelector.getAll()) {
         let active = selector.viewMode() == mode;
         selector.setActive(active);
      }
   }

   public static getActiveViewMode(): AcbOutputViewMode | null {
      for (const selector of OutputViewSelector.getAll()) {
         if (selector.isActive()) {
            return selector.viewMode();
         }
      }
      return null;
   }

   public static setup(onModeActive: (mode: AcbOutputViewMode) => void) {
      for (const selector of OutputViewSelector.getAll()) {
         selector.setup(onModeActive);
      }
   }

   // Hides any selectors that are not in the provided set.
   // eg. Used to hide the "Summary" tab if we just ran a regular ACB calculation.
   public static setSelectableViewModes(modes: Array<AcbOutputViewMode>) {
      for (const selector of OutputViewSelector.getAll()) {
         selector.element.style.display = (modes.includes(selector.viewMode())) ? 'block' : 'none';
      }
   }
}

export class YearHighlightSelector extends ElementModel {
   public static readonly ID: string = "yearHighlightSelect";

   public static get(): YearHighlightSelector {
      return new YearHighlightSelector(
         ElementModel.getRequiredElementById(YearHighlightSelector.ID));
   }

   public setup() {
      this.element.addEventListener('change', () => {
         const selectedYear = this.getSelectedYear();
         SecurityTablesOutputContainer.setYearRowStyles(selectedYear);
         if (InactiveYearHideCheckbox.get().isChecked()) {
            SecurityTablesOutputContainer.setSecurityWrapperStyles(selectedYear);
         } else {
            SecurityTablesOutputContainer.setSecurityWrapperStyles(null);
         }
      });
   }

   public getSelectedYear(): string | null {
      const selectElem = this.element as HTMLSelectElement;
      const selectedValue = selectElem.value;
      return selectedValue === "None" ? null : selectedValue;
   }

   public updateSelectableYears(years: number[]) {
      const selectElem = this.element as HTMLSelectElement;
      const currentSelection = selectElem.value;

      // Clear all old options
      selectElem.innerHTML = "";

      // Add "None" option at the start
      const noneOption = new ElemBuilder("option")
         .text("None")
         .attributes({ value: "None" })
         .build();
      selectElem.appendChild(noneOption);

      // Add year options
      for (const year of years) {
         const yearOption = new ElemBuilder("option")
         .text(year.toString())
         .attributes({ value: year.toString() })
         .build();
         selectElem.appendChild(yearOption);
      }

      // Restore previous selection if still available, otherwise default to "None"
      if (years.includes(parseInt(currentSelection))) {
         selectElem.value = currentSelection;
      } else {
         selectElem.value = "None";
      }
   }
}

export class InactiveYearHideCheckbox extends CheckboxElementModel {
   public static readonly ID: string = "hideNoActivityCheckbox";

   public static get(): InactiveYearHideCheckbox {
      return new InactiveYearHideCheckbox(
         ElementModel.getRequiredElementById(InactiveYearHideCheckbox.ID));
   }

   public setup() {
      this.setChangeListener(() => {
         const isChecked = InactiveYearHideCheckbox.get().isChecked();
         const selectedYear = isChecked ? YearHighlightSelector.get().getSelectedYear() : null;
         SecurityTablesOutputContainer.setSecurityWrapperStyles(selectedYear);
      });
   }
}

function selectableViewModesForAppFunction(funcMode: AppFunctionMode): Array<AcbOutputViewMode> {
   switch (funcMode) {
      case AppFunctionMode.Calculate:
         return [
            AcbOutputViewMode.SecurityTables,
            AcbOutputViewMode.Aggregate,
            AcbOutputViewMode.Text,
         ];
      case AppFunctionMode.TxSummary:
         return [
            AcbOutputViewMode.Summary,
            AcbOutputViewMode.Text,
         ];
      case AppFunctionMode.TallyShares:
         return [
            AcbOutputViewMode.Summary,
            AcbOutputViewMode.Text,
         ];
   }
}

export class AcbOutput {
   public static setup() {
      ViewModeSelectorGroup.setup((mode: AcbOutputViewMode) => {
         AcbOutput.setActiveOutput(mode);
      })

      let defaultAppFunction = AppFunctionMode.Calculate;
      AcbOutput.setAppFunctionViewMode(defaultAppFunction);

      YearHighlightSelector.get().setup();
      InactiveYearHideCheckbox.get().setup();
   }

   public static setActiveOutput(mode: AcbOutputViewMode) {
      for (const op of AcbOutputKindContainer.getAll()) {
         op.setActive(op.viewMode() == mode);
      }
   }

   public static setActiveOutputAndSyncTab(viewMode: AcbOutputViewMode) {
      ViewModeSelectorGroup.setActiveViewMode(viewMode);
      AcbOutput.setActiveOutput(viewMode);
   }

   public static setAppFunctionViewMode(funcMode: AppFunctionMode) {
      const modes = selectableViewModesForAppFunction(funcMode);
      AcbOutput.setSelectableViewModes(modes);
   }

   private static setSelectableViewModes(modes: Array<AcbOutputViewMode>) {
      // Get selected view mode before we hide any selectors
      const selectedMode = ViewModeSelectorGroup.getActiveViewMode();
      // Check if selectedMode is in modes
      if (selectedMode !== null && !modes.includes(selectedMode)) {
         // If not, select the first mode in modes
         AcbOutput.setActiveOutputAndSyncTab(modes[0]);
      }

      ViewModeSelectorGroup.setSelectableViewModes(modes);
   }
}
