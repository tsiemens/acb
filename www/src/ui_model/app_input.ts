import { ButtonElementModel, ElementModel } from "./model_lib.js";
import { childHasFocus, ElemBuilder, InputElemBuilder } from "../dom_utils.js";

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

export class AcbExtraOptions {
   public static getPrintFullValuesCheckbox(): HTMLInputElement {
      return document.getElementById('printFullValuesCheckbox') as HTMLInputElement;
   }
}

export class InitSecItem {
   constructor(
      public security: string,
      public quantity: string,
      public acb: string,
   ) {}
}

export class InitialSymbolStateInputs extends ElementModel {
   public static readonly ID: string = "initialSymbolStateInputs";

   public static get(): InitialSymbolStateInputs {
      return new InitialSymbolStateInputs(
         ElementModel.getRequiredElementById(InitialSymbolStateInputs.ID));
   }

   public setup() {
      this.addNewEmptyRow();
   }

   private static handleFocusChange(event: Event) {
      // Sleep just a bit, because when we tab to a new input, it transiently focuses
      // the document body, not the next element.
      if (event.target) {
         let cell = event.target as HTMLElement;
         setTimeout(() => {
            const row = cell.parentElement;
            if (row && !childHasFocus(row)) {
               InitialSymbolStateInputs.highlightRowErrors(row);
            }
         }, 100);
      }
   }

   private addNewEmptyRow() {
      const buttonClickWrapper = (event: Event) => {
         if (event.target) {
            InitialSymbolStateInputs.get()
               .handleRowButtonClick(event.target as HTMLElement);
         }
      };

      const newInitDiv = new ElemBuilder("div")
         .classes(["init-sec-row"])
         .children([
            new InputElemBuilder("text").classes(["init-sec-input", "init-sec-name"])
               .eventListener("focus", InitialSymbolStateInputs.handleFocusChange)
               .eventListener("focusout", InitialSymbolStateInputs.handleFocusChange)
               .placeholder("SECURITY")
               .build(),
            new InputElemBuilder("number").classes(["init-sec-input", "init-sec-quant"])
               .eventListener("focus", InitialSymbolStateInputs.handleFocusChange)
               .eventListener("focusout", InitialSymbolStateInputs.handleFocusChange)
               .placeholder("quantity").pattern("[0-9]+")
               .build(),
            new InputElemBuilder("number").classes(["init-sec-input", "init-sec-acb"])
               .eventListener("focus", InitialSymbolStateInputs.handleFocusChange)
               .eventListener("focusout", InitialSymbolStateInputs.handleFocusChange)
               .placeholder("total cost basis (CAD)")
               .build(),
            new ElemBuilder("button")
               .classes(["btn", "btn-secondary", "btn-skinny", "init-sec-button"])
               .text("Add") // This is set as "X" when the row is in delete mode
               .eventListener("click", buttonClickWrapper)
               .eventListener("focus", InitialSymbolStateInputs.handleFocusChange)
               .eventListener("focusout", InitialSymbolStateInputs.handleFocusChange)
               .build(),
         ])
         .build();

      this.element.appendChild(newInitDiv);
   }

   /** The row button doubles as both an Add or delete, depending on the row
    * state. This handles both cases.
    */
   private handleRowButtonClick(button: HTMLElement) {
      if (button.dataset.deleteOnClick) {
         const row = button.parentElement;
         if (row) {
            this.element.removeChild(row);
         }
      } else {
         this.addNewEmptyRow();
         button.dataset.deleteOnClick = 'true';
         button.innerText = "X";
      }
   }

   /** performs some basic validation of the row, and highlights/styles
    * cells which need correction.
    */
   private static highlightRowErrors(rowElem: HTMLElement) {
      const row = InitialSymbolStateInputs.getRowContents(rowElem);

      const setError = (elem: HTMLElement, err: boolean) => {
         if (err) {
            elem.classList.add("init-sec-input-error");
         } else {
            elem.classList.remove("init-sec-input-error");
         }
      };

      if (row.secInput.value) {
         setError(row.secInput, false);
         setError(row.secQuantInput, !row.secQuantInput.value);
         setError(row.secAcbInput, !row.secAcbInput.value);
      } else if (!row.secInput.value && (row.secQuantInput.value || row.secAcbInput.value)) {
         setError(row.secInput, true);
         setError(row.secQuantInput, false);
         setError(row.secAcbInput, false);
      } else {
         setError(row.secInput, false);
         setError(row.secQuantInput, false);
         setError(row.secAcbInput, false);
      }
   }

   public static getRowContents(row: Element): {
      secInput: HTMLInputElement,
      secQuantInput: HTMLInputElement,
      secAcbInput: HTMLInputElement
   } {
      return {
         secInput: row.getElementsByClassName("init-sec-name")[0] as HTMLInputElement,
         secQuantInput: row.getElementsByClassName("init-sec-quant")[0]as HTMLInputElement,
         secAcbInput: row.getElementsByClassName("init-sec-acb")[0] as HTMLInputElement,
      };
   }

   public static getRowData(row: Element): InitSecItem {
      const rowContents = InitialSymbolStateInputs.getRowContents(row);
      const security = rowContents.secInput.value;
      const quant = rowContents.secQuantInput.value;
      const acb = rowContents.secAcbInput.value;
      return new InitSecItem(security, quant, acb);
   }

   public getData(): InitSecItem[] {
      let items: InitSecItem[] = [];
      for (const rowElem of this.element.children) {
         items.push(InitialSymbolStateInputs.getRowData(rowElem));
      }
      return items;
   }
}