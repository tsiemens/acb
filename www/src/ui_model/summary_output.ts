import { RenderTable } from "../acb_wasm_types.js";
import { AcbOutputKindContainer, AcbOutputViewMode, TableOutputContainerBase } from "./acb_app_output.js";
import { ElemBuilder } from "../dom_utils.js";

export class SummaryOutputContainer extends TableOutputContainerBase {
    public static readonly ID: string = "acbSummaryOutput";

    public static get(): SummaryOutputContainer {
        const element = document.getElementById(SummaryOutputContainer.ID);
        if (!element) {
            throw new Error(`Element with id ${SummaryOutputContainer.ID} not found`);
        }
        return new SummaryOutputContainer(element);
    }

    public viewMode(): AcbOutputViewMode {
        return AcbOutputViewMode.Summary;
    }

    public populateTable(table: RenderTable) {
        const container = this.element;
        container.innerHTML = "";

        // Create header row using base class helper
        const headerRow = TableOutputContainerBase.makeTableHeaderRow(table);

        // Create table body
        const tbody = new ElemBuilder('tbody').build();
        for (const row of table.rows) {
            const tr = new ElemBuilder('tr').build();
            for (const cell of row) {
                tr.appendChild(new ElemBuilder('td').text(cell).build());
            }
            tbody.appendChild(tr);
        }

        let errorsAndNotes =
            TableOutputContainerBase.makeTableErrorsAndNotes(table);

        // Create the final table container using base class helper
        const tableContainer = TableOutputContainerBase.makeTableContainer(headerRow, tbody);
        container.appendChild(TableOutputContainerBase.makeTableTitle("Summary"));
        container.appendChild(errorsAndNotes.errorsDiv);
        container.appendChild(tableContainer);
        container.appendChild(errorsAndNotes.notesDiv);
    }
}

AcbOutputKindContainer.registerOutputKindContainer(
   SummaryOutputContainer.ID, SummaryOutputContainer.get);
