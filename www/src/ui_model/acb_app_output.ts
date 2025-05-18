import { ElemBuilder } from "../dom_utils.js";
import { AppRenderResult, RenderTable } from "../acb_wasm_types.js";
import { ElementModel } from "./model_lib.js";

enum AcbOutputTab {
   Table = "table",
   Text = "text",
}

abstract class AcbOutputKindContainer extends ElementModel {
   public static getAll(): Array<AcbOutputKindContainer> {
      return [
         TableOutputContainer.get(),
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

   public abstract label(): AcbOutputTab;
}

export class TextOutputContainer extends AcbOutputKindContainer {
   public static readonly ID: string = "acb-text-output";

   public static get(): TextOutputContainer {
      return new TextOutputContainer(
         ElementModel.getRequiredElementById(TextOutputContainer.ID));
   }

   public label(): AcbOutputTab { return AcbOutputTab.Text; }
}

export class TableOutputContainer extends AcbOutputKindContainer {
   public static readonly ID: string = "acb-table-output";

   public static get(): TableOutputContainer {
      return new TableOutputContainer(
         ElementModel.getRequiredElementById(TableOutputContainer.ID));
   }

   public label(): AcbOutputTab { return AcbOutputTab.Table; }

   private static makeTableHeaderRow(tableModel: RenderTable): HTMLElement {
      const tr = new ElemBuilder("tr").build();
      for (const header of tableModel.header) {
         tr.appendChild(new ElemBuilder("th").text(header).build());
      }
      return tr;
   }

   private static makeTableContainer(
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

   private static makeTableTitle(title: string): HTMLElement {
      return new ElemBuilder('div').classes(['table-title']).text(title)
         .build();
   }

   private static makeAggregateGainsTable(model: AppRenderResult): HTMLElement {
      const aggModel = model.aggregateGainsTable;
      console.log("Agg model:");
      console.log(aggModel);
      const tr = TableOutputContainer.makeTableHeaderRow(aggModel);
      const tbody = new ElemBuilder('tbody').build();
      for (const row of aggModel.rows) {
         const rowElem = new ElemBuilder('tr').build();
         for (const item of row) {
            const td = new ElemBuilder('td').text(item).build();
            rowElem.appendChild(td);
         }
         tbody.appendChild(rowElem);
      }
      return TableOutputContainer.makeTableContainer(tr, tbody);
   }

   private static addSymbolTableComponents(
      symbol: string, model: AppRenderResult,
      tablesContainer: HTMLElement): void {
      console.debug("addSymbolTableComponents for: ", symbol);

      const symbolModel = model.securityTables.get(symbol);
      if (!symbolModel) {
         throw new Error(`No symbol model found for ${symbol}`);
      }
      const tr = TableOutputContainer.makeTableHeaderRow(symbolModel);
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

      const symTableContainer = TableOutputContainer.makeTableContainer(tr, tbody);
      tablesContainer.appendChild(TableOutputContainer.makeTableTitle(symbol));
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

   public populateTable(model: AppRenderResult) {
      let tablesContainer = this.element;
      tablesContainer.innerHTML = ""; // Clear previous tables

      // Aggregate table
      tablesContainer.appendChild(
         TableOutputContainer.makeTableTitle("Aggregate Gains"));
      tablesContainer.appendChild(
         TableOutputContainer.makeAggregateGainsTable(model));

      // Symbol tables (securityTables is a Map object)
      const symbols = Array.from(model.securityTables.keys());
      symbols.sort()
      for (const symbol of symbols) {
         TableOutputContainer.addSymbolTableComponents(
            symbol, model, tablesContainer);
      }
   }
}

export class TabSelector extends ElementModel {
   public static getAll(): Array<TabSelector> {
      const tabLabelElems = document.getElementsByClassName('tab-label');
      const tabLabels: Array<TabSelector> = [];
      for (const tabLabel of tabLabelElems) {
         tabLabels.push(new TabSelector(tabLabel as HTMLElement));
      }
      return tabLabels;
   }

   public static getByLabel(label: AcbOutputTab): TabSelector {
      return new TabSelector(
         ElementModel.getRequiredElementByQuery(`[data-tab-label=${label}]`));
   }

   public setup(onTabActive: (tab: AcbOutputTab) => void) {
      this.element.addEventListener('click', (event) => {
         if (event.target) {
            let clickedTab = new TabSelector(event.target as HTMLElement);
            let clickedLabel = clickedTab.label();
            TabSelectorGroup.setActiveTab(clickedLabel);
            onTabActive(clickedLabel);
         }
      });
   }

   public label(): AcbOutputTab {
      let label_ = this.element.dataset.tabLabel;
      if (!label_) {
         throw Error("TabSelector has no label");
      }
      if (label_ == AcbOutputTab.Table.toString()) {
         return AcbOutputTab.Table;
      } else if (label_ == AcbOutputTab.Text.toString()) {
         return AcbOutputTab.Text;
      }
      throw Error(`Invalid AcbOutputTab: ${label_}`);
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

class TabSelectorGroup {
   public static setActiveTab(label: AcbOutputTab) {
      for (const tab of TabSelector.getAll()) {
         let active = tab.label() == label;
         tab.setActive(active);
      }
   }

   public static setup(onTabActive: (tab: AcbOutputTab) => void) {
      for (const tab of TabSelector.getAll()) {
         tab.setup(onTabActive);
      }
   }
}


export class AcbOutput {
   public static setup() {
      let onTabActive = (label: AcbOutputTab) => {
         AcbOutput.setActiveOutput(label);
      };
      TabSelectorGroup.setup(onTabActive);

      // Default to table output shown
      AcbOutput.setActiveOutputAndSyncTab(AcbOutputTab.Table);
   }

   public static setActiveOutput(label: AcbOutputTab) {
      for (const op of AcbOutputKindContainer.getAll()) {
         op.setActive(op.label() == label);
      }
   }

   public static setActiveOutputAndSyncTab(label: AcbOutputTab) {
      TabSelectorGroup.setActiveTab(label);
      AcbOutput.setActiveOutput(label);
   }
}
