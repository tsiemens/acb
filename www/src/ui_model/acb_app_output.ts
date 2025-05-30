import { ElemBuilder } from "../dom_utils.js";
import { AppRenderResult, RenderTable } from "../acb_wasm_types.js";
import { ElementModel } from "./model_lib.js";

enum AcbOutputViewMode {
   SecurityTables = "security_tables",
   Aggregate = "aggregate",
   Text = "text",
}

abstract class AcbOutputKindContainer extends ElementModel {
   public static getAll(): Array<AcbOutputKindContainer> {
      return [
         SecurityTablesOutputContainer.get(),
         AggregateOutputContainer.get(),
         TextOutputContainer.get(),
      ];
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

abstract class TableOutputContainerBase extends AcbOutputKindContainer {
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
      tablesContainer.appendChild(TableOutputContainerBase.makeTableTitle(symbol));
      const errors = symbolModel.errors || [];
      for (const err of errors) {
         tablesContainer.appendChild(new ElemBuilder('p').classes(['error-text']).text(err).build());
      }
      if (errors.length > 0) {
         tablesContainer.appendChild(new ElemBuilder('p').text("Information is of parsed state only, and may not be fully correct.").build());
      }
      tablesContainer.appendChild(symTableContainer);
      for (const note of symbolModel.notes || []) {
         tablesContainer.appendChild(new ElemBuilder('p').text(note).build());
      }

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
}

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
      tablesContainer.appendChild(
         AggregateOutputContainer.makeAggregateGainsTable(model));
   }
}

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
}

class ViewModeSelectorGroup {
   public static setActiveViewMode(mode: AcbOutputViewMode) {
      for (const selector of OutputViewSelector.getAll()) {
         let active = selector.viewMode() == mode;
         selector.setActive(active);
      }
   }

   public static setup(onModeActive: (mode: AcbOutputViewMode) => void) {
      for (const selector of OutputViewSelector.getAll()) {
         selector.setup(onModeActive);
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

export class AcbOutput {
   public static setup() {
      ViewModeSelectorGroup.setup((mode: AcbOutputViewMode) => {
         AcbOutput.setActiveOutput(mode);
      })

      // Default to table output shown
      AcbOutput.setActiveOutputAndSyncTab(AcbOutputViewMode.SecurityTables);

      YearHighlightSelector.get().setup();
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
}
