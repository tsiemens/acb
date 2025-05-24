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

      const addRow = function(rowItems: string[]) {
         const actionCol = 3;
         let isSell = rowItems[actionCol].search(/sell/i) >= 0;
         let isSfla = rowItems[actionCol].search(/sfla/i) >= 0;

         const rowElem = new ElemBuilder('tr').build();
         if (isSell) {
            rowElem.classList.add('sell-row');
         } else if (isSfla) {
            rowElem.classList.add('sfla-row');
         }
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

export class AcbOutput {
   public static setup() {
      ViewModeSelectorGroup.setup((mode: AcbOutputViewMode) => {
         AcbOutput.setActiveOutput(mode);
      })

      // Default to table output shown
      AcbOutput.setActiveOutputAndSyncTab(AcbOutputViewMode.SecurityTables);
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
